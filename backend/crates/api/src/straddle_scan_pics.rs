//! Scan périodique de pics ATR pour tous les assets actifs.
//!
//! Tourne toutes les 5 minutes (indépendamment de la boucle Straddle).
//! Seuil de détection : ratio_atr > 1.3
//! Seuil de signal (Ollama) : ≥1.5 — géré par straddle_boucle.
use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::signal_engine::SignalEngine;

/// Ratio ATR minimum pour enregistrer un pic (sans déclencher signal Ollama).
const SEUIL_DETECTION: f64 = 1.3;

/// Anti-doublon : pas de second pic pour le même asset/TF avant N minutes.
const ANTI_DOUBLON_MIN: i64 = 30;

/// Démarre la surveillance de pics en background — ne bloque pas.
pub fn demarrer_scan_pics(db: Arc<Database>, _signal_engine: Arc<SignalEngine>) {
    tokio::spawn(async move {
        // Délai initial pour laisser la DB se charger
        sleep(Duration::from_secs(90)).await;
        loop {
            scanner_tous_assets(&db).await;
            sleep(Duration::from_secs(5 * 60)).await;
        }
    });
    tracing::info!("📡 Scan pics ATR démarré (5 min, tous assets dynamiques)");
}

async fn scanner_tous_assets(db: &Arc<Database>) {
    let assets = match db.lister_assets().await {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!("Scan pics: impossible de charger les assets: {}", e);
            return;
        }
    };

    for asset_db in &assets {
        let asset = match Asset::try_from(asset_db.id.as_str()) {
            Ok(a) => a,
            Err(_) => {
                tracing::debug!("Scan pics: asset inconnu ignoré: {}", asset_db.id);
                continue;
            }
        };
        let tf = if asset_db.type_asset == "crypto" {
            Timeframe::M5
        } else {
            Timeframe::M15
        };
        scanner_asset(db, &asset, &tf, &asset_db.type_asset).await;
    }
}

async fn scanner_asset(db: &Arc<Database>, asset: &Asset, tf: &Timeframe, _type_asset: &str) {
    // Anti-doublon
    match db::straddle_pics::dernier_pic_asset(
        db.pool(),
        asset.as_str(),
        tf.as_str(),
        ANTI_DOUBLON_MIN,
    )
    .await
    {
        Ok(Some(_)) => return, // Pic récent déjà enregistré
        Err(e) => {
            tracing::warn!(
                "Scan pics anti-doublon {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
        Ok(None) => {}
    }

    // Chargement des bougies
    let bougies = match db.obtenir_bougies(asset, tf, 60).await {
        Ok(b) if b.len() >= 20 => b,
        Ok(_) => return, // Pas assez de données
        Err(e) => {
            tracing::debug!(
                "Scan pics bougies {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
    };

    // Calcul ATR
    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_valides: Vec<f64> = atr_vals.iter().copied().filter(|v| !v.is_nan()).collect();
    if atr_valides.len() < 2 {
        return;
    }
    let atr_actuel = match atr_valides.last().copied() {
        Some(v) => v,
        None => return,
    };
    let n_moy = atr_valides.len().min(14);
    let atr_moyen = atr_valides.iter().rev().take(n_moy).sum::<f64>() / n_moy as f64;

    if atr_actuel <= 0.0 || atr_moyen <= 0.0 {
        return;
    }

    let ratio_atr = atr_actuel / atr_moyen;
    if ratio_atr < SEUIL_DETECTION {
        return;
    }

    let prix = bougies.last().map(|b| b.close).unwrap_or(0.0);
    if prix <= 0.0 {
        return;
    }

    let donnees = DonneesAtr {
        prix,
        atr_actuel,
        atr_moyen,
        ratio_atr,
    };
    enregistrer_pic(db, asset, tf, donnees).await;
}

struct DonneesAtr {
    prix: f64,
    atr_actuel: f64,
    atr_moyen: f64,
    ratio_atr: f64,
}

async fn enregistrer_pic(db: &Arc<Database>, asset: &Asset, tf: &Timeframe, donnees: DonneesAtr) {
    let DonneesAtr {
        prix,
        atr_actuel,
        atr_moyen,
        ratio_atr,
    } = donnees;
    let maintenant = Utc::now();

    // Annonces économiques prochaines (fenêtre ±90 min)
    let ts_now = maintenant.timestamp();
    let annonces: Vec<serde_json::Value> = db.lire_calendrier_cache(3600).await.unwrap_or_default();

    // Créneaux Straddle validés pour cet asset
    let creneaux = db::straddle::lister_creneaux_asset(db.pool(), asset.as_str())
        .await
        .unwrap_or_default();
    let creneaux_valides: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .cloned()
        .collect();

    let res = crate::straddle_categorisation::categoriser(
        &annonces,
        maintenant,
        &creneaux_valides,
        asset.as_str(),
    );

    let kill_zone_active = smc::kill_zone::est_en_kill_zone(maintenant);

    let pic = db::straddle_pics::NouveauPic {
        asset: asset.as_str().to_string(),
        timeframe: tf.as_str().to_string(),
        timestamp_pic: ts_now,
        prix,
        atr_actuel,
        atr_moyen_14: atr_moyen,
        ratio_atr,
        categorie: res.categorie.as_str().to_string(),
        evenement_nom: res.evenement_nom,
        evenement_devise: res.evenement_devise,
        evenement_impact: res.evenement_impact,
        minutes_avant_evt: res.minutes_avant_evt,
        session_active: res.session_active,
        kill_zone_active,
    };

    match db::straddle_pics::inserer_pic(db.pool(), &pic).await {
        Ok(id) => {
            tracing::info!(
                "📊 Pic ATR #{} {}/{} ratio={:.2}× cat={}",
                id,
                asset.as_str(),
                tf.as_str(),
                ratio_atr,
                pic.categorie
            );
        }
        Err(e) => {
            tracing::warn!(
                "Scan pics: insertion {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
        }
    }
}
