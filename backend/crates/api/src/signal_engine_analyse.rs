//! Logique d'analyse des assets pour le Signal Engine.
//! Séparé de signal_engine.rs pour respecter la limite de 300 lignes.
//! La logique par asset est dans signal_engine_asset.rs.
use common::{Asset, Signal};
use db::{strategies_params::lire_smc_params, Database};
use ml::PipelineML;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use strategies::smc_directional::SmcDirectionalStrategy;
use tokio::sync::{broadcast, RwLock};

use super::signal_engine::{assets_fallback, INTERVALLE_SECS, TIMEFRAMES};

pub(crate) async fn boucle_detection(
    running: Arc<AtomicBool>,
    prochain: Arc<std::sync::Mutex<i64>>,
    db: Arc<Database>,
    pipeline_ml: Arc<RwLock<PipelineML>>,
    tx: broadcast::Sender<Signal>,
    score_news: Arc<AtomicI32>,
    fg_valeur: Arc<AtomicI32>,
) {
    tracing::info!(
        "🤖 Signal Engine démarré — cycle {}s | assets dynamiques × {} TF",
        INTERVALLE_SECS,
        TIMEFRAMES.len()
    );

    while running.load(Ordering::SeqCst) {
        let ts_debut = chrono::Utc::now().timestamp();
        {
            if let Ok(mut guard) = prochain.lock() {
                *guard = ts_debut + INTERVALLE_SECS as i64;
            }
        }

        // Rechargement des paramètres SMC depuis la DB à chaque cycle
        let smc_params = lire_smc_params(db.pool()).await;
        let strategie = SmcDirectionalStrategy { params: smc_params };

        analyser_tous_assets(&strategie, &db, &pipeline_ml, &tx, &score_news, &fg_valeur).await;

        let steps = INTERVALLE_SECS / 5;
        for _ in 0..steps {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    tracing::info!("🛑 Signal Engine arrêté");
}

async fn analyser_tous_assets(
    strategie: &SmcDirectionalStrategy,
    db: &Arc<Database>,
    pipeline_ml: &Arc<RwLock<PipelineML>>,
    tx: &broadcast::Sender<Signal>,
    score_news: &Arc<AtomicI32>,
    fg_valeur: &Arc<AtomicI32>,
) {
    // E.1 — Suspension SMC Directionnel si événement macro High-impact dans ≤30 min
    match db.fenetre_macro_smc_dans_minutes().await {
        Ok(Some((titre, minutes))) => {
            tracing::info!(
                "⏸ SMC Directionnel suspendu — '{}' dans {} min",
                titre,
                minutes
            );
            return;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("Vérification fenêtre macro SMC: {}", e),
    }
    let assets_actifs = match db.lister_assets().await {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|r| crate::utils::parse_asset(&r.id))
            .collect::<Vec<Asset>>(),
        Err(e) => {
            tracing::warn!(
                "Signal Engine — impossible de charger les assets DB: {} — fallback",
                e
            );
            assets_fallback()
        }
    };

    tracing::debug!("Signal Engine — analyse {} assets", assets_actifs.len());

    for asset in &assets_actifs {
        for timeframe in TIMEFRAMES {
            if let Err(e) = crate::signal_engine_asset::analyser_asset(
                strategie,
                db,
                pipeline_ml,
                tx,
                asset,
                timeframe,
                score_news,
                fg_valeur,
            )
            .await
            {
                tracing::warn!(
                    "Signal Engine — {}/{}: {}",
                    asset.as_str(),
                    timeframe.as_str(),
                    e
                );
            }
        }
    }
}
