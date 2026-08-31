//! Méthodes DB bougies — requêtes analytiques et utilitaires.
//! Complément de bougies.rs (méthodes critiques insert/fetch).

use chrono::{TimeZone, Utc};
use common::{Asset, Candle, Result, Timeframe, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Extrêmes de prix (plus haut / plus bas) d'un asset sur une fenêtre
    /// temporelle, sur le timeframe demandé. None si aucune bougie couverte.
    /// Sert au calcul d'excursion favorable (MFE) des trades historiques.
    pub async fn extremes_bougies(
        &self,
        asset: &str,
        timeframe: &str,
        t0: i64,
        t1: i64,
    ) -> Result<Option<(f64, f64)>> {
        let row = sqlx::query(
            "SELECT MAX(high) as maxi, MIN(low) as mini
             FROM bougies
             WHERE asset = ? AND timeframe = ? AND timestamp >= ? AND timestamp <= ?",
        )
        .bind(asset)
        .bind(timeframe)
        .bind(t0)
        .bind(t1)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(row.and_then(|r| {
            let maxi: Option<f64> = r.get("maxi");
            let mini: Option<f64> = r.get("mini");
            match (maxi, mini) {
                (Some(h), Some(l)) => Some((h, l)),
                _ => None,
            }
        }))
    }

    /// Toutes les bougies d'un asset/timeframe sans plafond (ordre ASC)
    pub async fn obtenir_bougies_toutes(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
    ) -> Result<Vec<Candle>> {
        let rows = sqlx::query(
            "SELECT timestamp, open, high, low, close, volume
             FROM bougies
             WHERE asset = ? AND timeframe = ?
             ORDER BY timestamp ASC",
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
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
            .collect())
    }

    /// Retourne toutes les combinaisons (asset_str, timeframe_str) ayant ≥ min_bougies en DB,
    /// restreintes aux assets marqués `ml_actif=1` dans la table `assets`.
    /// Utilise la table `bougies_stats` pré-calculée (évite le COUNT(*) lent sur 10M lignes).
    pub async fn combinaisons_entrainables(
        &self,
        min_bougies: i64,
    ) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query(
            "SELECT s.asset, s.timeframe
             FROM bougies_stats s
             JOIN assets a ON a.id = s.asset AND a.ml_actif = 1
             WHERE s.nb >= ?
             ORDER BY s.asset, s.timeframe",
        )
        .bind(min_bougies)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| (r.get::<String, _>("asset"), r.get::<String, _>("timeframe")))
            .collect())
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
