use chrono::Utc;
use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Lit une valeur de configuration depuis la table `configuration`
    pub async fn lire_config(&self, cle: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT valeur FROM configuration WHERE cle = ?")
            .bind(cle)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.map(|r| r.get::<String, _>("valeur")))
    }

    /// Insère ou met à jour une valeur de configuration
    pub async fn ecrire_config(&self, cle: &str, valeur: &str) -> Result<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO configuration (cle, valeur, maj_le) VALUES (?, ?, ?)
             ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur, maj_le = excluded.maj_le",
        )
        .bind(cle)
        .bind(valeur)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }
}
