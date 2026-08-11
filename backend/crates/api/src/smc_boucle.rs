//! Boucle automatique d'analyse SMC toutes les 15 minutes.
//!
//! Pipeline : DB bougies → scorer SMC → catégorisation → seuils calibrés
//! → few-shot feedbacks → filtre LLM → signal publié + feedback inséré.
use common::{Asset, Timeframe};
use db::Database;
use ml::PipelineML;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::signal_engine::SignalEngine;
use crate::smc_signal_ollama::{appeler_smc_et_publier, ParamsSmc};

/// Garde anti-double-start. La boucle SMC doit n'être spawnée qu'une fois
/// (déjà lancée par AppState::new). Un second appel est un no-op + warning.
static SMC_DEMARREE: AtomicBool = AtomicBool::new(false);

/// Marque la boucle SMC comme démarrée. Retourne `true` si c'est le premier
/// appel (le spawn doit avoir lieu), `false` sinon. Fonction pure → testable.
fn marquer_smc_demarree() -> bool {
    !SMC_DEMARREE.swap(true, Ordering::SeqCst)
}

/// Intervalle entre deux cycles complets.
const INTERVALLE_SEC: u64 = 900; // 15 min
/// Anti-doublon : pas de second signal SMC sur le même asset/TF avant N minutes.
const ANTI_DOUBLON_MIN: i64 = 60;
/// Seuil SMC minimal par défaut avant calibration.
const SEUIL_SCORE_DEFAUT: f64 = 70.0;

/// Démarre la boucle en background — ne bloque pas.
pub fn demarrer_boucle_smc(
    db: Arc<Database>,
    signal_engine: Arc<SignalEngine>,
    pipeline_ml: Arc<RwLock<PipelineML>>,
) {
    if !marquer_smc_demarree() {
        tracing::warn!("⚠️  Boucle SMC déjà démarrée — second spawn ignoré");
        return;
    }
    tokio::spawn(async move {
        sleep(Duration::from_secs(120)).await;
        loop {
            let assets = db.lister_assets().await.unwrap_or_default();
            let nb = assets.len();
            let debut_cycle = std::time::Instant::now();
            let futs = assets.iter().filter_map(|asset_db| {
                let asset = Asset::try_from(asset_db.id.as_str()).ok()?;
                let tf = if asset_db.type_asset == "crypto" { Timeframe::M5 } else { Timeframe::M15 };
                let db = db.clone(); let se = signal_engine.clone(); let ml = pipeline_ml.clone();
                Some(async move { analyser_asset(db, se, ml, asset, tf).await; })
            });
            futures_util::future::join_all(futs).await;
            let duree_cycle = debut_cycle.elapsed();
            tracing::info!(
                "📐 Boucle SMC cycle terminé ({} assets) en {:.1}s",
                nb,
                duree_cycle.as_secs_f64()
            );
            sleep(Duration::from_secs(INTERVALLE_SEC)).await;
        }
    });
    tracing::info!("📐 Boucle SMC Directionnel démarrée (15 min, assets dynamiques depuis DB)");
}

async fn analyser_asset(
    db: Arc<Database>,
    signal_engine: Arc<SignalEngine>,
    pipeline_ml: Arc<RwLock<PipelineML>>,
    asset: Asset,
    tf: Timeframe,
) {
    // Anti-doublon
    match db.signal_recent_existe(&asset, &tf, ANTI_DOUBLON_MIN).await {
        Ok(true) => return,
        Err(e) => {
            tracing::warn!(
                "SMC boucle anti-doublon {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
        Ok(false) => {}
    }

    // Bougies
    let bougies = match db.obtenir_bougies(&asset, &tf, 200).await {
        Ok(b) if b.len() >= 30 => b,
        Ok(_) => return,
        Err(e) => {
            tracing::warn!(
                "SMC boucle DB bougies {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
    };

    // Scoring SMC
    let score = match smc::scorer(&bougies) {
        Some(s) => s,
        None => return, // Tendance indécise ou données insuffisantes
    };

    // Indicateurs pour seuils et contexte
    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_valides: Vec<f64> = atr_vals.iter().copied().filter(|v| !v.is_nan()).collect();
    if atr_valides.len() < 2 {
        return;
    }
    let atr14 = match atr_valides.last().copied() {
        Some(v) => v,
        None => return,
    };
    let prix = bougies.last().map(|b| b.close).unwrap_or(0.0);
    if prix <= 0.0 || atr14 <= 0.0 {
        return;
    }
    let n_moy = atr_valides.len().min(14);
    let atr_moyen = atr_valides.iter().rev().take(n_moy).sum::<f64>() / n_moy as f64;
    let atr_ratio = atr14 / atr_moyen.max(f64::EPSILON);

    let rsi_vals = indicators::calculer_rsi(&bougies, 14);
    let rsi = rsi_vals
        .iter()
        .rev()
        .find(|v| !v.is_nan())
        .copied()
        .unwrap_or(50.0);

    let now = chrono::Utc::now();

    // Catégorisation SMC
    let categ = crate::smc_categorisation::categoriser_smc(
        score.order_block,
        score.ifvg,
        score.imbalance,
        score.fibonacci > 5.0,
        score.kill_zone_active,
        score.sweep_detecte,
        now,
    );

    // Seuils calibrés pour ce triplet (asset, tf, categorie)
    let asset_str = asset.as_str();
    let tf_str = tf.as_str();
    let seuils =
        db::smc_calibration::charger_seuils(db.pool(), asset_str, tf_str, categ.categorie.as_str())
            .await;

    // Catégorie invalide → skip
    if seuils.invalide {
        tracing::debug!(
            "SMC boucle {}/{}: catégorie {} invalide, skip",
            asset_str,
            tf_str,
            categ.categorie.as_str()
        );
        return;
    }

    // Score insuffisant
    let seuil_score = seuils.score_smc_seuil.max(SEUIL_SCORE_DEFAUT - 5.0);
    if score.total < seuil_score {
        return;
    }

    // Gate ML : rejeter si modèle insuffisamment confiant
    let seuil_smc: f64 = sqlx::query_scalar(
        "SELECT valeur FROM configuration WHERE cle = 'seuil_confiance_smc'",
    )
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse().ok())
    .unwrap_or(0.60);
    let confiance_ml: f64 = {
        let ml = pipeline_ml.read().await;
        if ml.est_pret() {
            match ml.predire(&bougies) {
                Ok(pred) if pred.confiance < seuil_smc => {
                    tracing::debug!(
                        "SMC {}/{}: ML peu confiant ({:.2} < {:.2}), skip",
                        asset_str,
                        tf_str,
                        pred.confiance,
                        seuil_smc,
                    );
                    return;
                }
                Ok(pred) => pred.confiance,
                Err(_) => 0.0,
            }
        } else {
            0.0
        }
    };

    // Feedbacks few-shot (5 derniers trades clôturés sur ce triplet)
    let feedbacks = db::smc_feedback::lister_feedbacks_asset_categorie(
        db.pool(),
        asset_str,
        tf_str,
        categ.categorie.as_str(),
        5,
    )
    .await
    .unwrap_or_default();

    // Direction et SL/TP
    let direction_str = match score.direction {
        common::Direction::Long => "Haussier",
        common::Direction::Short => "Baissier",
        common::Direction::Both => return, // ne devrait pas arriver après scorer()
    };
    let sl = if score.direction == common::Direction::Long {
        prix - atr14
    } else {
        prix + atr14
    };
    let tp1 = if score.direction == common::Direction::Long {
        prix + atr14 * 1.5
    } else {
        prix - atr14 * 1.5
    };

    let params = ParamsSmc {
        asset: &asset,
        tf: &tf,
        direction_str,
        prix,
        sl,
        tp1,
        atr14,
        atr_ratio,
        rsi,
        score_smc: score.total,
        confiance_ml,
        kill_zone_active: score.kill_zone_active,
        sweep_detecte: score.sweep_detecte,
        categorie: &categ.categorie,
        session_active: &categ.session_active,
        feedbacks: &feedbacks,
        conviction_seuil: seuils.conviction_seuil,
        // Features pour snapshot ML (P13 SMC)
        features_ohlcv: ml::extraire_features(&bougies).unwrap_or_default(),
        tendance_pts: score.tendance,
        order_block_pts: score.order_block,
        ifvg_pts: score.ifvg,
        fibonacci_pts: score.fibonacci,
        imbalance_pts: score.imbalance,
    };

    if let Err(e) = appeler_smc_et_publier(&db, &signal_engine, params).await {
        tracing::warn!("SMC boucle {}/{}: {}", asset_str, tf_str, e);
    }
}

#[cfg(test)]
#[path = "smc_boucle_tests.rs"]
mod tests;
