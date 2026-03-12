use anyhow::Result;
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Database>,
    pub pipeline_ml: Arc<Mutex<PipelineML>>,
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
            Ok(false) => tracing::info!("Pipeline ML initialisé (pas de modèle persisté — entraînement requis)"),
            Err(e) => tracing::warn!("Impossible de charger le pipeline ML: {}", e),
        }

        Ok(Self {
            db: Arc::new(db),
            pipeline_ml: Arc::new(Mutex::new(pipeline_ml)),
        })
    }
}
