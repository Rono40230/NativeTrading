//! Bibliothèque des essais du laboratoire de simulation (api::simulation).
//! Chaque simulation lancée y est conservée : paramètres virtuels +
//! résultats agrégés — pour comparer plusieurs réglages d'un coup.

use crate::{Database, TradingError};
use sqlx::Row;

/// Un essai persisté (déserialisé par l'appelant via serde_json).
#[derive(Debug)]
pub struct EssaiSimulation {
    pub id: String,
    pub strategie: String,
    pub params_json: String,
    pub resultats_json: String,
    pub cree_le: i64,
}

impl Database {
    /// Enregistre un essai (INSERT OR REPLACE — même id = même essai).
    pub async fn enregistrer_essai_simulation(
        &self,
        id: &str,
        strategie: &str,
        params_json: &str,
        resultats_json: &str,
        cree_le: i64,
    ) -> Result<(), TradingError> {
        sqlx::query(
            "INSERT OR REPLACE INTO simulation_essais
             (id, strategie, params_json, resultats_json, cree_le)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(strategie)
        .bind(params_json)
        .bind(resultats_json)
        .bind(cree_le)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| TradingError::Database(e.to_string()))
    }

    /// Les `limite` derniers essais d'une stratégie (plus récent d'abord).
    pub async fn lister_essais_simulation(
        &self,
        strategie: &str,
        limite: i64,
    ) -> Result<Vec<EssaiSimulation>, TradingError> {
        let rows = sqlx::query(
            "SELECT id, strategie, params_json, resultats_json, cree_le
             FROM simulation_essais
             WHERE strategie = ?
             ORDER BY cree_le DESC LIMIT ?",
        )
        .bind(strategie)
        .bind(limite)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| EssaiSimulation {
                id: r.get("id"),
                strategie: r.get("strategie"),
                params_json: r.get("params_json"),
                resultats_json: r.get("resultats_json"),
                cree_le: r.get("cree_le"),
            })
            .collect())
    }

    /// Supprime un essai.
    pub async fn supprimer_essai_simulation(&self, id: &str) -> Result<(), TradingError> {
        sqlx::query("DELETE FROM simulation_essais WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| TradingError::Database(e.to_string()))
    }
}
