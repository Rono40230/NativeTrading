use chrono::Utc;
use common::{Asset, Result, Signal, Timeframe, TradingError};
use sqlx::Row;

use crate::Database;

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
                    "id":         row.get::<String, _>("id"),
                    "asset":      row.get::<String, _>("asset"),
                    "timeframe":  row.get::<String, _>("timeframe"),
                    "direction":  row.get::<String, _>("direction"),
                    "score":      row.get::<f64, _>("score"),
                    "prix_entree":row.get::<f64, _>("prix_entree"),
                    "stop_loss":  row.get::<f64, _>("stop_loss"),
                    "take_profit":row.get::<String, _>("take_profit"),
                    "strategie":  row.get::<String, _>("strategie"),
                    "cree_le":    row.get::<i64, _>("cree_le"),
                })
            })
            .collect();

        Ok(signaux)
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

    /// Retourne les N derniers signaux d'un asset pour injection dans les prompts LLM.
    /// Ne propage pas les erreurs — retourne vec![] si la DB est indisponible.
    pub async fn obtenir_contexte_llm(
        &self,
        asset: &str,
        limit: i64,
    ) -> Vec<serde_json::Value> {
        let rows = sqlx::query(
            "SELECT direction, timeframe, score, prix_entree, statut, cree_le
             FROM signaux WHERE asset = ? ORDER BY cree_le DESC LIMIT ?",
        )
        .bind(asset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(rows) => rows.iter().map(|row| serde_json::json!({
                "direction":  row.get::<String, _>("direction"),
                "timeframe":  row.get::<String, _>("timeframe"),
                "score":      row.get::<f64, _>("score"),
                "prix_entree": row.get::<f64, _>("prix_entree"),
                "statut":     row.get::<String, _>("statut"),
                "cree_le":    row.get::<i64, _>("cree_le"),
            })).collect(),
            Err(_) => vec![]
        }
    }
}
