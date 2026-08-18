use chrono::{TimeZone, Utc};
use common::{Asset, Candle, Result, Timeframe, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Insère un lot de bougies avec une source explicite.
    /// `source` : 'binance' | 'bybit_ws' | 'mt5' | 'csv'
    pub async fn inserer_bougies_avec_source(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        bougies: &[Candle],
        source: &str,
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
                 (asset, timeframe, timestamp, open, high, low, close, volume, source)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(asset_str)
            .bind(tf_str)
            .bind(ts)
            .bind(bougie.open)
            .bind(bougie.high)
            .bind(bougie.low)
            .bind(bougie.close)
            .bind(bougie.volume)
            .bind(source)
            .execute(&mut *tx)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
            inseres += res.rows_affected();
        }

        tx.commit()
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        // Mettre à jour bougies_stats pour que combinaisons_entrainables() reste rapide
        if inseres > 0 {
            let _ = sqlx::query(
                "INSERT INTO bougies_stats (asset, timeframe, nb)
                 VALUES (?, ?, (SELECT COUNT(*) FROM bougies WHERE asset = ? AND timeframe = ?))
                 ON CONFLICT(asset, timeframe) DO UPDATE SET
                 nb = (SELECT COUNT(*) FROM bougies WHERE asset = excluded.asset AND timeframe = excluded.timeframe)",
            )
            .bind(asset_str)
            .bind(tf_str)
            .bind(asset_str)
            .bind(tf_str)
            .execute(&self.pool)
            .await;
        }

        Ok(inseres)
    }

    /// Insère un lot de bougies (étiquette REST Bybit par défaut — la série
    /// de référence de l'app ; l'ancien défaut 'binance' était mensonger).
    /// Compatibilité avec les anciens appels qui ne précisent pas la source.
    pub async fn inserer_bougies(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        bougies: &[Candle],
    ) -> Result<u64> {
        self.inserer_bougies_avec_source(asset, timeframe, bougies, "bybit_rest")
            .await
    }

    /// Récupère toutes les bougies depuis `nb_jours` jours en arrière (ordre ASC).
    /// Utilise un filtre `WHERE timestamp >= ?` — pas de cap arbitraire.
    pub async fn obtenir_bougies_depuis_jours(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        nb_jours: u32,
    ) -> Result<Vec<Candle>> {
        let depuis = Utc::now().timestamp() - (nb_jours as i64 * 86_400);
        let rows = sqlx::query(
            "SELECT timestamp, open, high, low, close, volume
             FROM bougies
             WHERE asset = ? AND timeframe = ? AND timestamp >= ?
             ORDER BY timestamp ASC",
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .bind(depuis)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let bougies = rows
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

        Ok(bougies)
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

    /// Récupère les N dernières bougies réelles pour les charts (exclut source='mt5').
    /// À utiliser exclusivement pour l'affichage graphique.
    pub async fn obtenir_bougies_chart(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        limit: i64,
    ) -> Result<Vec<Candle>> {
        let rows = sqlx::query(
            "SELECT timestamp, open, high, low, close, volume
             FROM bougies
             WHERE asset = ? AND timeframe = ? AND source != 'mt5'
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

        bougies.reverse();
        Ok(bougies)
    }

    /// Retourne le timestamp Unix (secondes) de la bougie réelle la plus récente
    /// pour un asset/timeframe (exclut mt5). None si aucune bougie réelle en cache.
    pub async fn timestamp_derniere_bougie_chart(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
    ) -> Result<Option<i64>> {
        let row = sqlx::query(
            "SELECT MAX(timestamp) as max_ts FROM bougies
             WHERE asset = ? AND timeframe = ? AND source != 'mt5'",
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(row.get::<Option<i64>, _>("max_ts"))
    }

    /// Récupère les bougies M1 d'un asset filtrées sur une plage horaire UTC (SQL),
    /// évitant de charger l'intégralité des données M1 en mémoire.
    /// `heure_debut` / `heure_fin` : format "HH:MM" UTC
    pub async fn obtenir_bougies_plage_horaire_m1(
        &self,
        asset: &Asset,
        heure_debut: &str,
        heure_fin: &str,
    ) -> Result<Vec<Candle>> {
        let parse = |s: &str| -> Option<i64> {
            let mut p = s.splitn(2, ':');
            let h: i64 = p.next()?.parse().ok()?;
            let m: i64 = p.next()?.parse().ok()?;
            Some(h * 3600 + m * 60)
        };
        let debut_sec =
            parse(heure_debut).ok_or_else(|| TradingError::Data("heure_debut invalide".into()))?;
        let fin_sec =
            parse(heure_fin).ok_or_else(|| TradingError::Data("heure_fin invalide".into()))?;

        let rows = sqlx::query(
            "SELECT timestamp, open, high, low, close, volume
             FROM bougies
             WHERE asset = ? AND timeframe = 'M1'
               AND (timestamp % 86400) >= ?
               AND (timestamp % 86400) < ?
             ORDER BY timestamp ASC",
        )
        .bind(asset.as_str())
        .bind(debut_sec)
        .bind(fin_sec)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let bougies = rows
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

        Ok(bougies)
    }

}
