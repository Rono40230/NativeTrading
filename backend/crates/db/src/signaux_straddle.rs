use chrono::Utc;
use common::{Asset, Result, Timeframe, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Vérifie un anti-doublon récent filtré par stratégie.
    /// Compte les signaux actifs, ou fermés récemment, dans une fenêtre stricte.
    pub async fn signal_recent_existe_strategie(
        &self,
        asset: &Asset,
        tf: &Timeframe,
        strategie: &str,
        min: i64,
    ) -> Result<bool> {
        let seuil = Utc::now().timestamp() - min * 60;
        let row = sqlx::query(
            "SELECT COUNT(*) as n FROM signaux
                         WHERE asset = ? AND timeframe = ? AND LOWER(strategie) = LOWER(?)
               AND cree_le >= ?
               AND (statut = 'Actif' OR COALESCE(ferme_le, 0) >= ?)",
        )
        .bind(asset.as_str())
        .bind(tf.as_str())
        .bind(strategie)
        .bind(seuil)
        .bind(seuil)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.get::<i64, _>("n") > 0)
    }
}
