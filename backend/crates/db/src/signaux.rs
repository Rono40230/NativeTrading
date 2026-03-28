use chrono::Utc;
use common::{Asset, Result, Signal, Timeframe, TradingError};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::Database;

/// Signal actif retourné par le worker de suivi
#[derive(Debug, Serialize)]
pub struct SignalActif {
    pub id: String,
    pub asset: String,
    pub direction: String,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: Vec<f64>,
    pub cree_le: i64,
}

// ── Fonctions libres sur SqlitePool (utilisées par le worker) ────────────────

pub async fn lister_actifs(pool: &SqlitePool) -> Result<Vec<SignalActif>> {
    let rows = sqlx::query(
        "SELECT id, asset, direction, prix_entree, stop_loss, take_profit, cree_le
         FROM signaux WHERE statut = 'Actif' ORDER BY cree_le DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|row| {
            let tp_raw = row.get::<String, _>("take_profit");
            let take_profit: Vec<f64> = serde_json::from_str(&tp_raw).unwrap_or_default();
            SignalActif {
                id: row.get("id"),
                asset: row.get("asset"),
                direction: row.get("direction"),
                prix_entree: row.get("prix_entree"),
                stop_loss: row.get("stop_loss"),
                take_profit,
                cree_le: row.get("cree_le"),
            }
        })
        .collect())
}

pub async fn maj_verdict(pool: &SqlitePool, id: &str, verdict: &str, prix: f64) -> Result<()> {
    let now = Utc::now().timestamp();
    sqlx::query(
        "UPDATE signaux SET statut='Fermé', verdict=?, prix_verdict=?, ferme_le=? WHERE id=?",
    )
    .bind(verdict)
    .bind(prix)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn expirer_anciens(pool: &SqlitePool) -> Result<i64> {
    let seuil = Utc::now().timestamp() - 24 * 3600;
    let res = sqlx::query(
        "UPDATE signaux SET statut='Fermé', verdict='expire', ferme_le=?
         WHERE statut='Actif' AND cree_le < ?",
    )
    .bind(Utc::now().timestamp())
    .bind(seuil)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(res.rows_affected() as i64)
}

impl Database {
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

    /// Enregistre un signal Straddle (Direction::Both) avec les niveaux des deux jambes.
    /// - `stop_loss`         = SL jambe LONG  (< prix_entree)
    /// - `take_profit`       = [tp1, tp2, tp3] jambe LONG  (> prix_entree)
    /// - `sl_short`          = SL jambe SHORT (> prix_entree)
    /// - `take_profit_short` = [tp1, tp2, tp3] jambe SHORT (< prix_entree)
    pub async fn inserer_signal_straddle_complet(
        &self,
        signal: &Signal,
        sl_short: f64,
        take_profit_short: &[f64],
    ) -> Result<()> {
        let tp_long_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;
        let tp_short_json = serde_json::to_string(take_profit_short)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le,
              sl_short, take_profit_short)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal.id.to_string())
        .bind(signal.asset.as_str())
        .bind(signal.timeframe.as_str())
        .bind(format!("{:?}", signal.direction))
        .bind(signal.score)
        .bind(signal.prix_entree)
        .bind(signal.stop_loss)
        .bind(tp_long_json)
        .bind(&signal.strategie)
        .bind(signal.cree_le.timestamp())
        .bind(sl_short)
        .bind(tp_short_json)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }

    /// Liste les signaux encore actifs (pour le worker de suivi).
    pub async fn lister_signaux_actifs(&self) -> Result<Vec<SignalActif>> {
        lister_actifs(&self.pool).await
    }

    /// Expire les signaux actifs depuis plus de 24h sans verdict.
    pub async fn expirer_signaux_anciens(&self) -> Result<i64> {
        expirer_anciens(&self.pool).await
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
            "SELECT COUNT(*) as n FROM signaux
             WHERE asset = ? AND timeframe = ? AND cree_le >= ?",
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

    /// Enregistre un signal avec les métadonnées LLM du filtre pré-sauvegarde.
    pub async fn inserer_signal_avec_llm(
        &self,
        signal: &Signal,
        llm_valide: i64,
        llm_conviction: i64,
        llm_raison: &str,
        llm_sl_suggere: Option<f64>,
        llm_tp1_suggere: Option<f64>,
    ) -> Result<()> {
        let tp_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le,
              llm_valide, llm_conviction, llm_raison, llm_sl_suggere, llm_tp1_suggere)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .bind(llm_valide)
        .bind(llm_conviction)
        .bind(llm_raison)
        .bind(llm_sl_suggere)
        .bind(llm_tp1_suggere)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }
}
