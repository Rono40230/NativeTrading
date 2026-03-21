use chrono::{TimeZone, Utc};
use common::{Asset, Candle, Result, Timeframe, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Insère un lot de bougies en une seule transaction (ignore les doublons via UNIQUE).
    /// Beaucoup plus rapide que N inserts individuels et libère le lock SQLite immédiatement.
    pub async fn inserer_bougies(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        bougies: &[Candle],
    ) -> Result<u64> {
        if bougies.is_empty() {
            return Ok(0);
        }
        let asset_str = asset.as_str();
        let tf_str = timeframe.as_str();
        let mut inseres = 0u64;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        for bougie in bougies {
            let ts = bougie.timestamp.timestamp();
            let res = sqlx::query(
                "INSERT OR IGNORE INTO bougies
                 (asset, timeframe, timestamp, open, high, low, close, volume)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(asset_str)
            .bind(tf_str)
            .bind(ts)
            .bind(bougie.open)
            .bind(bougie.high)
            .bind(bougie.low)
            .bind(bougie.close)
            .bind(bougie.volume)
            .execute(&mut *tx)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
            inseres += res.rows_affected();
        }

        tx.commit()
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(inseres)
    }

    /// Récupère les N dernières bougies d'un asset/timeframe (ordre ASC)
    pub async fn obtenir_bougies(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        limit: i64,
    ) -> Result<Vec<Candle>> {
        let rows = sqlx::query(
            "SELECT timestamp, open, high, low, close, volume
             FROM bougies
             WHERE asset = ? AND timeframe = ?
             ORDER BY timestamp DESC
             LIMIT ?",
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let mut bougies: Vec<Candle> = rows
            .iter()
            .map(|r| {
                let ts: i64 = r.get("timestamp");
                Candle {
                    timestamp: Utc.timestamp_opt(ts, 0).single().unwrap_or(Utc::now()),
                    open: r.get("open"),
                    high: r.get("high"),
                    low: r.get("low"),
                    close: r.get("close"),
                    volume: r.get("volume"),
                }
            })
            .collect();

        bougies.reverse(); // DESC → ASC
        Ok(bougies)
    }

    /// Nombre de bougies stockées pour un asset/timeframe
    pub async fn compter_bougies(&self, asset: &Asset, timeframe: &Timeframe) -> Result<i64> {
        let row =
            sqlx::query("SELECT COUNT(*) as n FROM bougies WHERE asset = ? AND timeframe = ?")
                .bind(asset.as_str())
                .bind(timeframe.as_str())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.get::<i64, _>("n"))
    }

    /// Couverture données : count + min/max timestamp par asset × timeframe
    pub async fn obtenir_couverture_donnees(&self) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT asset, timeframe,
                    COUNT(*) as n,
                    MIN(timestamp) as min_ts,
                    MAX(timestamp) as max_ts
             FROM bougies
             GROUP BY asset, timeframe
             ORDER BY asset, timeframe",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "asset":     r.get::<String, _>("asset"),
                    "timeframe": r.get::<String, _>("timeframe"),
                    "count":     r.get::<i64, _>("n"),
                    "min_ts":    r.get::<i64, _>("min_ts"),
                    "max_ts":    r.get::<i64, _>("max_ts"),
                })
            })
            .collect())
    }
}
