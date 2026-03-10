use chrono::{TimeZone, Utc};
use common::{Asset, Candle, Result, Signal, TradingError, Timeframe};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::str::FromStr;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &str) -> Result<Self> {
        // Retirer le préfixe sqlite: éventuel pour construire les options
        let chemin = path
            .strip_prefix("sqlite://")
            .or_else(|| path.strip_prefix("sqlite:"))
            .unwrap_or(path);

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", chemin))
            .map_err(|e| TradingError::Database(e.to_string()))?
            .create_if_missing(true);

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

    /// Insère un lot de bougies (ignore les doublons via UNIQUE)
    pub async fn inserer_bougies(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        bougies: &[Candle],
    ) -> Result<u64> {
        let asset_str = asset.as_str();
        let tf_str = timeframe.as_str();
        let mut inseres = 0u64;

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
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
            inseres += res.rows_affected();
        }
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

    /// Enregistre un signal en base
    pub async fn inserer_signal(&self, signal: &Signal) -> Result<()> {
        let tp_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal.id.to_string())
        .bind(signal.asset.as_str())
        .bind(signal.timeframe.as_str())
        .bind(format!("{:?}", signal.direction))
        .bind(signal.score)
        .bind(signal.prix_entree)
        .bind(signal.stop_loss)
        .bind(tp_json)
        .bind(&signal.strategie)
        .bind(signal.cree_le.timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }

    /// Nombre de bougies stockées pour un asset/timeframe
    pub async fn compter_bougies(&self, asset: &Asset, timeframe: &Timeframe) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as n FROM bougies WHERE asset = ? AND timeframe = ?",
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(row.get::<i64, _>("n"))
    }

    /// Récupère les derniers signaux enregistrés
    pub async fn obtenir_signaux(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT id, asset, timeframe, direction, score, prix_entree,
                    stop_loss, take_profit, strategie, cree_le
             FROM signaux ORDER BY cree_le DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let signaux: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<String, _>("id"),
                    "asset": row.get::<String, _>("asset"),
                    "timeframe": row.get::<String, _>("timeframe"),
                    "direction": row.get::<String, _>("direction"),
                    "score": row.get::<f64, _>("score"),
                    "prix_entree": row.get::<f64, _>("prix_entree"),
                    "stop_loss": row.get::<f64, _>("stop_loss"),
                    "take_profit": row.get::<String, _>("take_profit"),
                    "strategie": row.get::<String, _>("strategie"),
                    "cree_le": row.get::<i64, _>("cree_le"),
                })
            })
            .collect();

        Ok(signaux)
    }
}
