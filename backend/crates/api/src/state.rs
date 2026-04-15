use anyhow::Result;
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ig_lightstreamer::IgLightstreamer;
use crate::ig_session::IgSession;
use crate::scheduler::demarrer_scheduler;
use crate::signal_engine::SignalEngine;

pub struct AppState {
    pub db: Arc<Database>,
    pub pipeline_ml: Arc<Mutex<PipelineML>>,
    /// État du job de réentraînement incrémental (Phase 8.4)
    pub retrain_state: Arc<tokio::sync::RwLock<crate::ml_retrain_handler::RetainState>>,
    /// Session IG Markets (CST + X-SECURITY-TOKEN, TTL 5h, relogin auto)
    pub ig_session: Arc<Mutex<IgSession>>,
    /// Client Lightstreamer IG — streaming CHART: OHLC temps réel
    pub ig_lightstreamer: Arc<IgLightstreamer>,
    /// Moteur de génération automatique de signaux SMC
    pub signal_engine: Arc<SignalEngine>,
    /// Cache Fear & Greed Index (TTL 1h) — (Instant du fetch, données JSON)
    pub fear_greed_cache: Arc<tokio::sync::RwLock<Option<(std::time::Instant, serde_json::Value)>>>,
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

        // Session IG Markets — login immédiat en arrière-plan si credentials présents
        let ig_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let ig_session = Arc::new(Mutex::new(IgSession::new(ig_client)));

        // Démarrage automatique du Signal Engine au lancement du serveur
        let db = Arc::new(db);
        let pipeline_ml = Arc::new(Mutex::new(pipeline_ml));
        let signal_engine = Arc::new(SignalEngine::new());
        signal_engine.demarrer(db.clone(), pipeline_ml.clone());
        tracing::info!("🤖 Signal Engine démarré automatiquement");

        // Pré-connexion IG en arrière-plan au démarrage (db déjà dans Arc)
        {
            let ig_init = ig_session.clone();
            let db_init = db.clone();
            tokio::spawn(async move {
                let mut session = ig_init.lock().await;
                match session.login(&db_init).await {
                    Ok(()) => tracing::info!("✅ IG Markets: connecté au démarrage"),
                    Err(e) => tracing::warn!("⚠️  IG Markets: login différé — {}", e),
                }
            });
        }

        // Client Lightstreamer — démarrage de la boucle de streaming
        let (ls_client, _rx) = IgLightstreamer::new(ig_session.clone(), db.clone());
        let ig_lightstreamer = Arc::new(ls_client);
        {
            let ls = ig_lightstreamer.clone();
            tokio::spawn(async move {
                ls.run().await;
            });
        }
        tracing::info!("📡 IG Lightstreamer: boucle de streaming démarrée");

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

        // Job de réconciliation des signaux Straddle ouverts (toutes les 5 min)
        crate::straddle_feedback_job::demarrer_job_feedback(db.clone());

        // Job de calibration automatique des seuils (toutes les 6h)
        crate::straddle_calibration::demarrer_calibration(db.clone());

        // Job de calibration automatique des seuils Rockets (toutes les 6h)
        crate::rockets_calibration::demarrer_calibration_rockets(db.clone());

        // Boucle analyse SMC Directionnel (toutes les 15 min)
        crate::smc_boucle::demarrer_boucle_smc(db.clone(), signal_engine.clone(), pipeline_ml.clone());

        // Job de réconciliation des signaux SMC ouverts (toutes les 5 min)
        crate::smc_feedback_job::demarrer_job_feedback_smc(db.clone());

        // Job de calibration automatique des seuils SMC (toutes les 6h)
        crate::smc_calibration_job::demarrer_calibration_smc(db.clone());

        // Job de détection de patterns d'échec récurrents (toutes les 6h)
        crate::patterns_echec_job::demarrer_job_patterns_echec(db.clone());

        // Job quotidien de mise à jour des valeur_pips (paires JPY)
        crate::pip_updater::demarrer_pip_updater(db.clone(), ig_session.clone());

        Ok(Self {
            db,
            pipeline_ml,
            retrain_state: Arc::new(tokio::sync::RwLock::new(
                crate::ml_retrain_handler::RetainState::default(),
            )),
            ig_session,
            ig_lightstreamer,
            signal_engine,
            fear_greed_cache: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }
}
