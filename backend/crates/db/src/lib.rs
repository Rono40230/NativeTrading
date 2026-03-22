pub mod assets;
pub mod bougies;
pub mod calendrier;
pub mod config;
pub mod entrainements;
pub mod rockets;
pub mod signaux;
pub mod volatilite;

use common::{Result, TradingError};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &str) -> Result<Self> {
        let chemin = path
            .strip_prefix("sqlite://")
            .or_else(|| path.strip_prefix("sqlite:"))
            .unwrap_or(path);

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", chemin))
            .map_err(|e| TradingError::Database(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));

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

    /// Accès au pool SQLx pour les crates qui en ont besoin directement.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
