#![allow(unused_variables, dead_code)]
use anyhow::Result;
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::scheduler::demarrer_scheduler;
use crate::signal_engine::SignalEngine;
use smc::v12::sentiment::SentimentScore;

pub struct AppState {
    pub db: Arc<Database>,
    pub pipeline_ml: Arc<RwLock<PipelineML>>,
    /// État du job de réentraînement incrémental (Phase 8.4)
    pub retrain_state: Arc<tokio::sync::RwLock<crate::ml_retrain_handler::RetainState>>,
    /// Moteur de génération automatique de signaux SMC
    pub signal_engine: Arc<SignalEngine>,
    /// Cache Fear & Greed Index (TTL 1h) — (Instant du fetch, données JSON)
    pub fear_greed_cache: Arc<tokio::sync::RwLock<Option<(std::time::Instant, serde_json::Value)>>>,
    /// Sentiment composite 0-100 par classe (refresh 30 min par le worker).
    pub sentiment: Arc<RwLock<Option<SentimentScore>>>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let db_path =
            std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());

        // Créer le dossier parent si nécessaire (ex: data/)
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let db = Database::new(&db_path).await?;
        db.run_migrations().await?;
        tracing::info!("Base de données initialisée + migrations: {}", db_path);

        // Pipeline ML — tente de recharger les modèles persistés
        let mut pipeline_ml = PipelineML::new();
        let modele_deja_charge = match pipeline_ml.charger_depuis_disque() {
            Ok(true) => {
                tracing::info!("Pipeline ML rechargé depuis disque");
                true
            }
            Ok(false) => {
                tracing::info!(
                    "Pipeline ML initialisé (pas de modèle persisté — entraînement immédiat prévu)"
                );
                false
            }
            Err(e) => {
                tracing::warn!("Impossible de charger le pipeline ML: {}", e);
                false
            }
        };

        // Démarrage automatique du Signal Engine au lancement du serveur
        let db = Arc::new(db);
        let pipeline_ml = Arc::new(RwLock::new(pipeline_ml));
        let signal_engine = Arc::new(SignalEngine::new());
        // Cache Fear & Greed (TTL 1h) — partagé entre l'endpoint et le worker sentiment.
        let fear_greed_cache: Arc<
            tokio::sync::RwLock<Option<(std::time::Instant, serde_json::Value)>>,
        > = Arc::new(tokio::sync::RwLock::new(None));
        signal_engine.demarrer(db.clone(), pipeline_ml.clone());
        tracing::info!("🤖 Signal Engine démarré automatiquement");

        // Scheduler ML : entraînement immédiat si pas de modèle, puis quotidien à 00h00 UTC
        demarrer_scheduler(db.clone(), pipeline_ml.clone(), modele_deja_charge);
        tracing::info!("⏰ Scheduler ML activé (immédiat si pas de modèle, puis 00h00 UTC)");

        // Surveillance ML toutes les 6h : ré-entraînement auto si accuracy_val < 52%
        crate::scheduler::demarrer_surveillance_ml(db.clone(), pipeline_ml.clone());
        tracing::info!("🔍 Surveillance ML activée (check toutes les 6h, seuil 52%)");

        // Boucle Straddle automatique toutes les 15 min (assets dynamiques)
        crate::straddle_boucle::demarrer_boucle_straddle(
            db.clone(),
            signal_engine.clone(),
            pipeline_ml.clone(),
        );

        // Scan pics ATR toutes les 5 min (tous assets dynamiques, seuil 1.3)
        crate::straddle_scan_pics::demarrer_scan_pics(db.clone(), signal_engine.clone());

        // Refresh calendrier économique en arrière-plan (toutes les 30 min)
        crate::calendar_handlers::demarrer_refresh_calendrier_job(db.clone());

        // Job de réconciliation des signaux Straddle ouverts (toutes les 5 min)
        crate::straddle_feedback_job::demarrer_job_feedback(db.clone());

        // Moniteur temps-réel des positions Straddle (trailing + SL progressif, cycle 60s)
        crate::straddle_moniteur_position::demarrer_moniteur_straddle(db.clone());

        // Job de calibration automatique des seuils (toutes les 6h)
        crate::straddle_calibration::demarrer_calibration(db.clone());

        // Job de calibration automatique des seuils Rockets (toutes les 6h)
        crate::rockets_calibration::demarrer_calibration_rockets(db.clone());

        // Boucle analyse SMC Directionnel (toutes les 15 min)
        let sentiment_slot: Arc<RwLock<Option<SentimentScore>>> =
            Arc::new(RwLock::new(None));
        crate::smc_boucle::demarrer_boucle_smc(
            db.clone(),
            signal_engine.clone(),
            pipeline_ml.clone(),
            sentiment_slot.clone(),
        );

        // Worker sentiment composite (cycle 30 min) — alimente le post-filtre directionnel
        crate::sentiment_composite::demarrer_worker_sentiment(
            db.clone(),
            sentiment_slot.clone(),
            fear_greed_cache.clone(),
        );
        tracing::info!("📊 Sentiment composite activé (worker 30 min)");

        // Job de réconciliation des signaux SMC ouverts (toutes les 5 min)
        crate::smc_feedback_job::demarrer_job_feedback_smc(db.clone());

        // Job de calibration automatique des seuils SMC (toutes les 6h)
        crate::smc_calibration_job::demarrer_calibration_smc(db.clone());

        // Job de détection de patterns d'échec récurrents (toutes les 6h)
        crate::patterns_echec_job::demarrer_job_patterns_echec(db.clone());

        // Job quotidien de mise à jour des valeur_pips (paires JPY)
        crate::pip_updater::demarrer_pip_updater(db.clone());

        Ok(Self {
            db,
            pipeline_ml,
            retrain_state: Arc::new(tokio::sync::RwLock::new(
                crate::ml_retrain_handler::RetainState::default(),
            )),
            signal_engine,
            fear_greed_cache,
            sentiment: sentiment_slot,
        })
    }
}
