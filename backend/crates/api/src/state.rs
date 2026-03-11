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

        let mut pipeline_ml = PipelineML::new();
        match pipeline_ml.charger_depuis_disque() {
            Ok(true) => tracing::info!("Pipeline ML rechargé depuis disque"),
            Ok(false) => tracing::info!("Pipeline ML initialisé (pas de modèle persisté — entraînement requis)"),
            Err(e) => tracing::warn!("Impossible de charger le pipeline ML: {}", e),
        }

        // Pré-initialiser la clé Twelvedata depuis .env si absente de la DB
        if let Ok(cle) = std::env::var("TWELVEDATA_API_KEY") {
            if !cle.is_empty() {
                if let Ok(None) = db.lire_config("twelvedata_api_key").await {
                    if let Err(e) = db.ecrire_config("twelvedata_api_key", &cle).await {
                        tracing::warn!("Impossible de persister la clé Twelvedata: {}", e);
                    } else {
                        tracing::info!("Clé API Twelvedata initialisée depuis .env");
                    }
                }
            }
        }

        Ok(Self {
            db: Arc::new(db),
            pipeline_ml: Arc::new(Mutex::new(pipeline_ml)),
        })
    }
}
