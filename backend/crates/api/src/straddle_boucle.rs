use common::{Asset, Timeframe};
use db::Database;
use ml::PipelineML;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

use crate::signal_engine::SignalEngine;
use crate::straddle_boucle_analyse::{analyser_asset, WhipsawDelais};

const SEUIL_SIGNAL_DEFAUT: f64 = 1.5;

static STRADDLE_DEMARREE: AtomicBool = AtomicBool::new(false);
fn marquer_straddle_demarree() -> bool {
    !STRADDLE_DEMARREE.swap(true, Ordering::SeqCst)
}

pub fn demarrer_boucle_straddle(
    db: Arc<Database>,
    signal_engine: Arc<SignalEngine>,
    pipeline_ml: Arc<RwLock<PipelineML>>,
) {
    if !marquer_straddle_demarree() {
        tracing::warn!("⚠️  Boucle Straddle déjà démarrée — second spawn ignoré");
        return;
    }
    tokio::spawn(async move {
        sleep(Duration::from_secs(180)).await;
        let whipsaw_delais: WhipsawDelais = Arc::new(Mutex::new(HashMap::new()));
        loop {
            let assets = db.lister_assets().await.unwrap_or_default();
            let nb = assets.len();
            let seuil_straddle: f64 = db
                .lire_config("seuil_confiance_straddle")
                .await
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.75);
            let (atr_ui, sl_mult, tp_mult_1, tp_mult_2, tp_mult_3): (f64, f64, f64, f64, f64) = {
                let p = db::strategies_params::lire_straddle_params(db.pool()).await;
                let atr = if p.atr_seuil.is_finite() && p.atr_seuil > 0.0 {
                    p.atr_seuil
                } else {
                    SEUIL_SIGNAL_DEFAUT
                };
                (atr, p.sl_mult, p.tp_mult_1, p.tp_mult_2, p.tp_mult_3)
            };
            tracing::debug!(
                "Straddle auto cycle: atr_ui={:.2}, seuil_ml={:.2}, assets_count={}",
                atr_ui,
                seuil_straddle,
                nb
            );
            let actifs_corr: HashSet<String> =
                db::straddle_suivi_position::lister_suivi_actifs(db.pool())
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.asset)
                    .collect();
            let mut tasks: Vec<(Asset, Timeframe, bool)> = Vec::new();
            for asset_db in &assets {
                let asset = match Asset::try_from(asset_db.id.as_str()) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let tf = Timeframe::M15;
                let skip_ww = {
                    let key = asset.as_str();
                    let mut map = whipsaw_delais.lock().await;
                    if let Some(&echeance) = map.get(key) {
                        if Instant::now() >= echeance {
                            map.remove(key);
                            true
                        } else {
                            continue;
                        }
                    } else {
                        false
                    }
                };
                tasks.push((asset, tf, skip_ww));
            }
            futures_util::future::join_all(tasks.into_iter().map(|(asset, tf, skip_ww)| {
                let (db, se, ml, wd, ac) = (
                    db.clone(),
                    signal_engine.clone(),
                    pipeline_ml.clone(),
                    whipsaw_delais.clone(),
                    actifs_corr.clone(),
                );
                async move {
                    analyser_asset(
                        &db,
                        &se,
                        &ml,
                        seuil_straddle,
                        atr_ui,
                        sl_mult,
                        tp_mult_1,
                        tp_mult_2,
                        tp_mult_3,
                        &asset,
                        &tf,
                        &wd,
                        skip_ww,
                        &ac,
                    )
                    .await;
                }
            }))
            .await;
            tracing::debug!("🌪️  Boucle Straddle cycle terminé ({} assets)", nb);
            sleep(Duration::from_secs(15 * 60)).await;
        }
    });
    tracing::info!("🌪️  Boucle Straddle auto démarrée (15 min, assets dynamiques depuis DB)");
}
