use anyhow::Result;
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::scheduler::demarrer_scheduler;
use crate::signal_engine::SignalEngine;

pub struct AppState {
    pub db: Arc<Database>,
    pub pipeline_ml: Arc<Mutex<PipelineML>>,
    /// Port IB Gateway (4002 = paper, 4001 = live)
    pub ib_port: u16,
    /// Client ID pour la connexion IB (doit être unique par connexion)
    pub ib_client_id: i32,
    /// Moteur de génération automatique de signaux SMC
    pub signal_engine: Arc<SignalEngine>,
    /// Dernier contexte backtest formaté — injecté dans les analyses LLM SMC.
    pub contexte_backtest: Arc<tokio::sync::RwLock<Option<String>>>,
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

        // Configuration IB Gateway depuis variables d'environnement ou valeurs par défaut
        let ib_port = std::env::var("IB_GATEWAY_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(4002);
        let ib_client_id = std::env::var("IB_GATEWAY_CLIENT_ID")
            .ok()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(100);

        tracing::info!(
            "IB Gateway configuré: 127.0.0.1:{} (client_id={})",
            ib_port,
            ib_client_id
        );

        // Démarrage automatique du Signal Engine au lancement du serveur
        let db = Arc::new(db);
        let pipeline_ml = Arc::new(Mutex::new(pipeline_ml));
        let signal_engine = Arc::new(SignalEngine::new());
        signal_engine.demarrer(db.clone(), pipeline_ml.clone());
        tracing::info!("🤖 Signal Engine démarré automatiquement");

        // Scheduler ML : entraînement immédiat si pas de modèle, puis quotidien à 00h00 UTC
        demarrer_scheduler(db.clone(), pipeline_ml.clone(), modele_deja_charge);
        tracing::info!("⏰ Scheduler ML activé (immédiat si pas de modèle, puis 00h00 UTC)");

        // Surveillance ML toutes les 6h : ré-entraînement auto si accuracy_val < 52%
        crate::scheduler::demarrer_surveillance_ml(db.clone(), pipeline_ml.clone());
        tracing::info!("🔍 Surveillance ML activée (check toutes les 6h, seuil 52%)");

        // Boucle Straddle automatique toutes les 15 min (assets dynamiques)
        crate::straddle_boucle::demarrer_boucle_straddle(db.clone(), signal_engine.clone());

        // Scan pics ATR toutes les 5 min (tous assets dynamiques, seuil 1.3)
        crate::straddle_scan_pics::demarrer_scan_pics(db.clone(), signal_engine.clone());

        // Job de réconciliation des signaux Straddle ouverts (toutes les 5 min)
        crate::straddle_feedback_job::demarrer_job_feedback(db.clone());

        // Job de calibration automatique des seuils (toutes les 6h)
        crate::straddle_calibration::demarrer_calibration(db.clone());

        // Job de calibration automatique des seuils Rockets (toutes les 6h)
        crate::rockets_calibration::demarrer_calibration_rockets(db.clone());

        // Boucle analyse SMC Directionnel (toutes les 15 min)
        crate::smc_boucle::demarrer_boucle_smc(db.clone(), signal_engine.clone());

        // Job de réconciliation des signaux SMC ouverts (toutes les 5 min)
        crate::smc_feedback_job::demarrer_job_feedback_smc(db.clone());

        // Job de calibration automatique des seuils SMC (toutes les 6h)
        crate::smc_calibration_job::demarrer_calibration_smc(db.clone());

        // Job quotidien de mise à jour des valeur_pips (paires JPY)
        crate::pip_updater::demarrer_pip_updater(db.clone());

        Ok(Self {
            db,
            pipeline_ml,
            ib_port,
            ib_client_id,
            signal_engine,
            contexte_backtest: Arc::new(tokio::sync::RwLock::new(None)),
            fear_greed_cache: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }
}
