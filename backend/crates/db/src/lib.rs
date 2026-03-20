use chrono::{TimeZone, Utc};
use common::{Asset, Candle, Result, Signal, Timeframe, TradingError};
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
            .create_if_missing(true)
            // WAL : plusieurs lecteurs simultanés, réduit les locks entre Signal Engine et collecte
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            // Attend jusqu'à 10s si la DB est verrouillée avant de renvoyer SQLITE_BUSY
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
        let row =
            sqlx::query("SELECT COUNT(*) as n FROM bougies WHERE asset = ? AND timeframe = ?")
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

    /// Lit les annonces du cache si fetched_at >= now - ttl_sec
    pub async fn lire_calendrier_cache(&self, ttl_sec: i64) -> Result<Vec<serde_json::Value>> {
        let seuil = Utc::now().timestamp() - ttl_sec;
        let rows = sqlx::query(
            "SELECT id, date_heure, devise, titre, impact, precedent, prevision
             FROM calendrier_cache
             WHERE fetched_at >= ?
             ORDER BY date_heure ASC",
        )
        .bind(seuil)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id":         r.get::<String, _>("id"),
                    "date_heure": r.get::<String, _>("date_heure"),
                    "devise":     r.get::<String, _>("devise"),
                    "titre":      r.get::<String, _>("titre"),
                    "impact":     r.get::<String, _>("impact"),
                    "precedent":  r.get::<Option<String>, _>("precedent"),
                    "prevision":  r.get::<Option<String>, _>("prevision"),
                })
            })
            .collect())
    }

    /// Vérifie si un signal (même asset/timeframe) existe dans la fenêtre anti-doublon.
    pub async fn signal_recent_existe(
        &self,
        asset: &Asset,
        tf: &Timeframe,
        min: i64,
    ) -> Result<bool> {
        let seuil = Utc::now().timestamp() - min * 60;
        let row = sqlx::query(
            "SELECT COUNT(*) as n FROM signaux WHERE asset = ? AND timeframe = ? AND cree_le >= ?",
        )
        .bind(asset.as_str())
        .bind(tf.as_str())
        .bind(seuil)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.get::<i64, _>("n") > 0)
    }

    /// Compte les signaux générés dans les `minutes` dernières minutes.
    pub async fn compter_signaux_recents(&self, minutes: i64) -> Result<i64> {
        let seuil = Utc::now().timestamp() - minutes * 60;
        let row = sqlx::query("SELECT COUNT(*) as n FROM signaux WHERE cree_le >= ?")
            .bind(seuil)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.get::<i64, _>("n"))
    }

    /// Efface et ré-insère toutes les annonces économiques (mise à jour du cache)
    pub async fn ecrire_calendrier_cache(&self, annonces: &[serde_json::Value]) -> Result<()> {
        let now = Utc::now().timestamp();
        sqlx::query("DELETE FROM calendrier_cache")
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        for a in annonces {
            sqlx::query("INSERT INTO calendrier_cache (id, date_heure, devise, titre, impact, precedent, prevision, fetched_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(a["id"].as_str().unwrap_or("")).bind(a["date_heure"].as_str().unwrap_or(""))
                .bind(a["devise"].as_str().unwrap_or("")).bind(a["titre"].as_str().unwrap_or(""))
                .bind(a["impact"].as_str().unwrap_or("")).bind(a["precedent"].as_str())
                .bind(a["prevision"].as_str()).bind(now).execute(&self.pool).await
                .map_err(|e| TradingError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Couverture données : count + min/max timestamp par asset × timeframe
    pub async fn obtenir_couverture_donnees(&self) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT asset, timeframe, COUNT(*) as n, MIN(timestamp) as min_ts, MAX(timestamp) as max_ts FROM bougies GROUP BY asset, timeframe ORDER BY asset, timeframe",
        ).fetch_all(&self.pool).await.map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows.iter().map(|r| serde_json::json!({
            "asset": r.get::<String, _>("asset"), "timeframe": r.get::<String, _>("timeframe"),
            "count": r.get::<i64, _>("n"), "min_ts": r.get::<i64, _>("min_ts"), "max_ts": r.get::<i64, _>("max_ts"),
        })).collect())
    }
}
