use anyhow::Result;
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::Mutex;

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

        // Pipeline ML
        let mut pipeline_ml = PipelineML::new();
        match pipeline_ml.charger_depuis_disque() {
            Ok(true) => tracing::info!("Pipeline ML rechargé depuis disque"),
            Ok(false) => tracing::info!(
                "Pipeline ML initialisé (pas de modèle persisté — entraînement requis)"
            ),
            Err(e) => tracing::warn!("Impossible de charger le pipeline ML: {}", e),
        }

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

        Ok(Self {
            db: Arc::new(db),
            pipeline_ml: Arc::new(Mutex::new(pipeline_ml)),
            ib_port,
            ib_client_id,
            signal_engine: Arc::new(SignalEngine::new()),
        })
    }
}
