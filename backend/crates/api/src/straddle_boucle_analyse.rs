//! Analyse initiale Straddle : vérifications rapides + préparation du contexte.
use chrono::Utc;
use common::{Asset, Candle, Timeframe};
use db::Database;
use ml::PipelineML;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

use crate::signal_engine::SignalEngine;
use crate::straddle_boucle_signal::evaluer_et_publier_straddle;

pub(crate) type WhipsawDelais = Arc<Mutex<HashMap<String, Instant>>>;

pub(crate) const ANTI_DOUBLON_MIN: i64 = 30;

/// Données validées transmises à `evaluer_et_publier_straddle`.
pub(crate) struct ContexteSignalStraddle {
    pub bougies: Vec<Candle>,
    pub atr_actuel: f64,
    pub atr_moyen: f64,
    pub prix: f64,
    pub ratio_atr: f64,
    pub annonces: Vec<serde_json::Value>,
    pub correlation_active: bool,
    pub atr_seuil_ui: f64,
    pub sl_mult: f64,
    pub tp_mult_1: f64,
    pub tp_mult_2: f64,
    pub tp_mult_3: f64,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn analyser_asset(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    pipeline_ml: &Arc<RwLock<PipelineML>>,
    seuil_straddle: f64,
    atr_seuil_ui: f64,
    sl_mult: f64,
    tp_mult_1: f64,
    tp_mult_2: f64,
    tp_mult_3: f64,
    asset: &Asset,
    tf: &Timeframe,
    whipsaw_delais: &WhipsawDelais,
    skip_whipsaw: bool,
    actifs_corr: &HashSet<String>,
) {
    match db
        .signal_recent_existe_strategie(asset, tf, "Straddle", ANTI_DOUBLON_MIN)
        .await
    {
        Ok(true) => return,
        Err(e) => {
            tracing::warn!(
                "Straddle auto: anti-doublon {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
        Ok(false) => {}
    }
    let correlation_active = crate::straddle_utils::groupe_correlation(asset.as_str())
        .map(|g| {
            g.iter()
                .any(|a| *a != asset.as_str() && actifs_corr.contains(*a))
        })
        .unwrap_or(false);
    let bougies = match db.obtenir_bougies(asset, tf, 100).await {
        Ok(b) if b.len() >= 30 => b,
        Ok(b) => {
            tracing::debug!(
                "Straddle {}/{}: {} bougies insuffisantes",
                asset.as_str(),
                tf.as_str(),
                b.len()
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                "Straddle: bougies {}/{}: {}",
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
    if ratio_atr < atr_seuil_ui {
        return;
    }
    let now = Utc::now();
    let maintenant = now.timestamp();
    let dans_90min = maintenant + 5400;
    let annonces_brutes = db.lire_calendrier_cache(3600).await.unwrap_or_default();
    if annonces_brutes.is_empty() {
        let db_refresh = Arc::clone(db);
        tokio::spawn(async move {
            let n = crate::calendar_handlers::rafraichir_calendrier(db_refresh.as_ref())
                .await
                .len();
            tracing::debug!("Straddle auto: refresh calendrier ({} événements)", n);
        });
    }
    let annonces: Vec<serde_json::Value> = annonces_brutes
        .into_iter()
        .filter(|a| {
            a["impact"].as_str() == Some("High")
                && a["date_heure"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| {
                        let ts = dt.timestamp();
                        (maintenant..=dans_90min).contains(&ts)
                    })
                    .unwrap_or(false)
        })
        .collect();

    evaluer_et_publier_straddle(
        db,
        signal_engine,
        pipeline_ml,
        seuil_straddle,
        asset,
        tf,
        whipsaw_delais,
        skip_whipsaw,
        ContexteSignalStraddle {
            bougies,
            atr_actuel,
            atr_moyen,
            prix,
            ratio_atr,
            annonces,
            correlation_active,
            atr_seuil_ui,
            sl_mult,
            tp_mult_1,
            tp_mult_2,
            tp_mult_3,
        },
    )
    .await;
}
