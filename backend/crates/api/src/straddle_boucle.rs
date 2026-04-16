//! Boucle automatique d'analyse Straddle au démarrage.
//!
//! Tourne toutes les 15 minutes pour un ensemble d'assets/timeframes.
//! Reproduit la logique de `straddle_signal_handler` sans passer par HTTP.
//! Pipeline unifié : DB → signal_engine.publier() → Telegram.
use chrono::{Datelike, Timelike, Utc};
use common::{Asset, Timeframe};
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::signal_engine::SignalEngine;
use crate::straddle_ml_gate::{evaluer_ml_straddle, MlContexteStraddle};
use crate::straddle_signal_ollama::{appeler_ollama_et_publier, ParamsOllama};

/// Anti-doublon : pas de second signal Straddle sur le même asset/TF avant N minutes.
const ANTI_DOUBLON_MIN: i64 = 60;

/// Seuil ratio ATR par défaut (utilisé si pas de calibration disponible).
const SEUIL_SIGNAL_DEFAUT: f64 = 1.5;

/// Démarre la boucle en background — ne bloque pas.
pub fn demarrer_boucle_straddle(
    db: Arc<Database>,
    signal_engine: Arc<SignalEngine>,
    pipeline_ml: Arc<Mutex<PipelineML>>,
) {
    tokio::spawn(async move {
        // Délai initial : laisser la DB et les bougies se charger
        sleep(Duration::from_secs(180)).await;
        loop {
            let assets = db.lister_assets().await.unwrap_or_default();
            let nb = assets.len();
            // Lire le seuil une seule fois par cycle (pas par asset)
            let seuil_straddle: f64 = db.lire_config("seuil_confiance_straddle").await
                .ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0.75);
            for asset_db in &assets {
                let asset = match Asset::try_from(asset_db.id.as_str()) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let tf = Timeframe::M15;
                analyser_asset(&db, &signal_engine, &pipeline_ml, seuil_straddle, &asset, &tf).await;
            }
            tracing::debug!("🌪️  Boucle Straddle cycle terminé ({} assets)", nb);
            sleep(Duration::from_secs(15 * 60)).await;
        }
    });
    tracing::info!("🌪️  Boucle Straddle auto démarrée (15 min, assets dynamiques depuis DB)");
}

async fn analyser_asset(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
    seuil_straddle: f64,
    asset: &Asset,
    tf: &Timeframe,
) {
    // Anti-doublon
    match db.signal_recent_existe(asset, tf, ANTI_DOUBLON_MIN).await {
        Ok(true) => return,
        Err(e) => {
            tracing::warn!(
                "Straddle auto: erreur anti-doublon {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
        Ok(false) => {}
    }

    // Bougies et indicateurs
    let bougies = match db.obtenir_bougies(asset, tf, 100).await {
        Ok(b) if b.len() >= 30 => b,
        Ok(b) => {
            tracing::debug!(
                "Straddle auto {}/{}: {} bougies insuffisantes",
                asset.as_str(),
                tf.as_str(),
                b.len()
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                "Straddle auto: DB bougies {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
    };

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
    let prix = bougies.last().map(|b| b.close).unwrap_or(0.0);
    if prix <= 0.0 || atr_actuel <= 0.0 {
        return;
    }
    let ratio_atr = atr_actuel / atr_moyen.max(f64::EPSILON);

    // Seuil ATR : vérification préliminaire avec seuil par défaut (avant catégorisation)
    if ratio_atr < SEUIL_SIGNAL_DEFAUT {
        return;
    }

    let now = Utc::now();
    let maintenant = now.timestamp();
    let dans_90min = maintenant + 5400;

    let annonces: Vec<serde_json::Value> = db
        .lire_calendrier_cache(3600)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|a| {
            a["impact"].as_str() == Some("High")
                && a["date_heure"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| {
                        let ts = dt.timestamp();
                        ts >= maintenant && ts <= dans_90min
                    })
                    .unwrap_or(false)
        })
        .collect();

    let asset_str = asset.as_str().to_string();
    let creneaux = db::straddle::lister_creneaux_asset(db.pool(), &asset_str)
        .await
        .unwrap_or_default();
    let creneaux_actifs: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .collect();

    let jours = [
        "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
    ];
    let jour = jours[now.weekday().num_days_from_monday() as usize % 7];
    let heure = now.hour();

    let mut ctx = format!(
        "=== CONTEXTE STRADDLE TEMPS RÉEL ===\n\
        Asset: {asset_str} | Timeframe: {tf_str} | {jour} {heure:02}h UTC\n\
        Prix: {prix:.5} | ATR actuel: {atr:.5} | ATR moyen 14p: {moy:.5} | Ratio ATR: {ratio:.2}×\n\
        Session active: {session} | Positions ouvertes: 0 | Drawdown: 0.0%\n",
        tf_str = tf.as_str(),
        atr = atr_actuel,
        moy = atr_moyen,
        ratio = ratio_atr,
        session = smc::kill_zone::nom_kill_zone(now).unwrap_or("Hors session"),
    );

    if annonces.is_empty() {
        ctx.push_str("Annonces HIGH impact < 90min: aucune\n");
    } else {
        ctx.push_str("Annonces HIGH impact < 90min:\n");
        for a in &annonces {
            let dans = a["date_heure"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| (dt.timestamp() - maintenant) / 60)
                .unwrap_or(0);
            ctx.push_str(&format!(
                "  - {} | {} | dans {}min\n",
                a["titre"].as_str().unwrap_or("?"),
                a["devise"].as_str().unwrap_or("?"),
                dans
            ));
        }
    }

    if creneaux_actifs.is_empty() {
        ctx.push_str("Créneaux historiques: aucun\n");
    } else {
        ctx.push_str("Créneaux historiques validés (sur 2 ans de données):\n");
        for c in creneaux_actifs.iter().take(3) {
            let jours_label = c.jour_semaine
                .map(|j| ["Lun","Mar","Mer","Jeu","Ven","Sam","Dim"].get(j as usize).copied().unwrap_or("?"))
                .unwrap_or("tous jours");
            let timing = c.timing_optimal.as_deref().unwrap_or("-");
            let fenetre = c.fenetre_entree.as_deref().unwrap_or("-");
            let whipsaw = c.whipsaw_minutes.map(|w| format!("{}min", w)).unwrap_or_else(|| "-".into());
            ctx.push_str(&format!(
                "  {jours_label} {hd}–{hf} UTC | ATR×{atr:.2} | freq {freq:.0}% | wr {wr}% | timing:{timing} | fenêtre:{fenetre} | whipsaw:{whipsaw}\n",
                hd = c.heure_debut,
                hf = c.heure_fin,
                atr = c.atr_moyen.unwrap_or(0.0),
                freq = c.frequence.unwrap_or(0.0) * 100.0,
                wr = c.backtest_winrate
                    .map(|w| format!("{:.0}", w))
                    .unwrap_or_else(|| "?".to_string()),
            ));
        }
    }

    // Catégoriser le contexte actuel (pour cibler les feedbacks historiques pertinents)
    let creneaux_valides_complets: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .cloned()
        .collect();
    let categorie_ctx = crate::straddle_categorisation::categoriser(
        &annonces,
        now,
        &creneaux_valides_complets,
        asset.as_str(),
    );

    // Charger les seuils calibrés pour cette paire + catégorie
    let seuils = db::straddle_calibration::charger_seuils(
        db.pool(),
        asset.as_str(),
        categorie_ctx.categorie.as_str(),
    )
    .await;

    // Catégorie marquée invalide → skip
    if seuils.invalide {
        tracing::debug!(
            "Straddle boucle {}/{}: catégorie {} invalide (WR < 50%), skip",
            asset.as_str(),
            tf.as_str(),
            categorie_ctx.categorie.as_str()
        );
        return;
    }

    // Seuil ATR calibré : vérification affinée
    if ratio_atr < seuils.ratio_atr {
        tracing::debug!(
            "Straddle boucle {}/{}: ratio {:.2} < seuil calibré {:.2}",
            asset.as_str(),
            tf.as_str(),
            ratio_atr,
            seuils.ratio_atr
        );
        return;
    }

    // Charger les feedbacks clôturés pour cette paire + catégorie (few-shot)
    let feedbacks = db::straddle_feedback::lister_recents_asset_categorie(
        db.pool(),
        asset.as_str(),
        categorie_ctx.categorie.as_str(),
        10,
    )
    .await
    .unwrap_or_default();

    // Gate ML Straddle : si ML très confiant d'un côté → signal directionnel préférable, skip
    // Si ML indécis → bonus contexte pour Ollama
    let ml_contexte = evaluer_ml_straddle(pipeline_ml, &bougies, asset.as_str(), tf.as_str(), seuil_straddle).await;
    match ml_contexte {
        MlContexteStraddle::Directionnel(direction) => {
            tracing::debug!(
                "Straddle {}/{}: ML confiant direction {} — signal directionnel préférable, skip",
                asset.as_str(),
                tf.as_str(),
                direction
            );
            return;
        }
        MlContexteStraddle::Indecis(texte) => ctx.push_str(&texte),
        MlContexteStraddle::NonDisponible => {}
    }

    let params = ParamsOllama {
        prix,
        atr: atr_actuel,
        ctx: &ctx,
        feedbacks: &feedbacks,
        categorie: &categorie_ctx.categorie,
        score_seuil: seuils.score_llm,
        annonces: &annonces,
        bougies: &bougies,
        ratio_atr,
    };
    if let Err(e) = appeler_ollama_et_publier(db, signal_engine, asset, tf, params).await {
        tracing::warn!("Straddle auto {}/{}: {}", asset.as_str(), tf.as_str(), e);
    }
}

// ── Gate ML — voir straddle_ml_gate.rs ───────────────────────────────────────
