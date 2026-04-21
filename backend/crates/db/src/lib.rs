pub mod ab_test;
pub mod asset_params;
pub mod assets;
pub mod bougies;
pub mod bougies_ext;
pub mod calendrier;
pub mod config;
pub mod entrainements;
pub mod ml_feature_importance;
pub mod ml_feedback;
pub mod ml_feedback_rockets;
pub mod ml_feedback_straddle;
pub mod ml_samples;
pub mod news_lus;
pub mod regles_rejet;
pub mod rockets;
pub mod rockets_blacklist;
pub mod rockets_listing;
pub mod rockets_analyses;
pub mod rockets_calibration;
pub mod rockets_config;
pub mod rockets_feedback;
pub mod rockets_feedback_reclassify;
pub mod rockets_feedback_stats;
pub mod rockets_feedback_trader;
pub mod rockets_features;
pub mod signaux;
pub mod signaux_lecture;
pub mod smc_calibration;
pub mod smc_feedback;
pub mod smc_feedback_stats;
pub mod straddle;
pub mod straddle_calibration;
pub mod straddle_feedback;
pub mod straddle_feedback_stats;
pub mod smc_features;
pub mod straddle_features;
pub mod straddle_pics;
pub mod straddle_suivi_position;
pub mod strategies_params;
pub mod volatilite;

use common::{Result, TradingError};

use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};

use std::str::FromStr;

/// Retourne le label de session de marché pour le timestamp Unix courant.
/// London: 07h-16h UTC | New York: 12h-21h UTC | Asia: 22h-07h UTC.
pub fn session_sortie_courante(ts_unix: i64) -> String {
    let heure = (ts_unix % 86_400) / 3_600; // heure UTC 0-23
    if (7..12).contains(&heure) {
        "London".to_string()
    } else if (12..16).contains(&heure) {
        "London+NY".to_string()
    } else if (16..21).contains(&heure) {
        "New York".to_string()
    } else {
        "Asia/Off".to_string()
    }
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &str) -> Result<Self> {
        let chemin = path
            .strip_prefix("sqlite://")
            .or_else(|| path.strip_prefix("sqlite:"))
            .unwrap_or(path);        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", chemin))
            .map_err(|e| TradingError::Database(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10))
            .disable_statement_logging();


        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))
    }

    /// Construit un Database depuis un pool existant (ex. pour appels ponctuels sans migration).
    pub fn depuis_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Accès au pool SQLx pour les crates qui en ont besoin directement.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::{Asset, Candle, Timeframe};

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    fn bougie(close: f64, offset_s: i64) -> Candle {
        Candle {
            timestamp: Utc::now() + chrono::Duration::seconds(offset_s),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1000.0,
        }
    }

    #[tokio::test]
    async fn inserer_et_compter_bougies() {
        let db = db_test().await;
        let bougies = vec![bougie(100.0, -120), bougie(101.0, -60), bougie(102.0, 0)];
        let n = db
            .inserer_bougies(&Asset::BTC, &Timeframe::M1, &bougies)
            .await
            .expect("insert OK");
        assert_eq!(n, 3, "3 bougies insérées");
    }

    #[tokio::test]
    async fn inserer_bougies_ignore_doublons() {
        let db = db_test().await;
        let bougies = vec![bougie(100.0, 0)];
        db.inserer_bougies(&Asset::BTC, &Timeframe::M1, &bougies)
            .await
            .unwrap();
        // Même bougie (même timestamp) → INSERT OR IGNORE → 0 rows affected
        let n = db
            .inserer_bougies(&Asset::BTC, &Timeframe::M1, &bougies)
            .await
            .expect("insert OK");
        assert_eq!(n, 0, "doublon ignoré");
    }

    #[tokio::test]
    async fn signal_recent_existe_retourne_false_si_vide() {
        let db = db_test().await;
        let existe = db
            .signal_recent_existe(&Asset::BTC, &Timeframe::M15, 30)
            .await
            .expect("query OK");
        assert!(!existe, "Aucun signal → false");
    }
}
