/// Méthodes de lecture sur la table `signaux`.
/// Séparé de signaux.rs pour respecter la limite de 300 lignes par fichier.
use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Récupère les derniers signaux enregistrés (avec verdict si disponible).
    /// Pour les signaux Straddle (direction=Both), inclut sl_short et take_profit_short.
    pub async fn obtenir_signaux(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT id, asset, timeframe, direction, score, prix_entree,
                    stop_loss, take_profit, strategie, statut,
                    verdict, prix_verdict, r_realise, cree_le, ferme_le,
                    llm_conviction, llm_raison,
                    sl_short, take_profit_short,
                    sl_long_effectif, sl_short_effectif,
                    tps_long_atteints, tps_short_atteints,
                    heure_entree
             FROM signaux ORDER BY cree_le DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let signaux: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                let tp_raw = row.get::<String, _>("take_profit");
                let tp_arr: Vec<f64> = serde_json::from_str(&tp_raw).unwrap_or_default();

                let tp_short_arr: Vec<f64> = row
                    .get::<Option<String>, _>("take_profit_short")
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                serde_json::json!({
                    "id":                    row.get::<String, _>("id"),
                    "asset":                 row.get::<String, _>("asset"),
                    "timeframe":             row.get::<String, _>("timeframe"),
                    "direction":             row.get::<String, _>("direction"),
                    "score":                 row.get::<f64, _>("score"),
                    "prix_entree":           row.get::<f64, _>("prix_entree"),
                    "stop_loss":             row.get::<f64, _>("stop_loss"),
                    "take_profit":           tp_arr,
                    "strategie":             row.get::<String, _>("strategie"),
                    "statut":                row.get::<String, _>("statut"),
                    "verdict":               row.get::<Option<String>, _>("verdict"),
                    "prix_verdict":          row.get::<Option<f64>, _>("prix_verdict"),
                    "r_realise":             row.get::<Option<f64>, _>("r_realise"),
                    "llm_conviction":        row.get::<Option<i64>, _>("llm_conviction"),
                    "llm_raison":            row.get::<Option<String>, _>("llm_raison"),
                    "cree_le":               row.get::<i64, _>("cree_le"),
                    "ferme_le":              row.get::<Option<i64>, _>("ferme_le"),
                    "sl_short":              row.get::<Option<f64>, _>("sl_short"),
                    "take_profit_short":     tp_short_arr,
                    "sl_long_effectif":      row.get::<Option<f64>, _>("sl_long_effectif"),
                    "sl_short_effectif":     row.get::<Option<f64>, _>("sl_short_effectif"),
                    "tps_long_atteints":     serde_json::from_str::<Vec<String>>(
                        &row.get::<Option<String>, _>("tps_long_atteints").unwrap_or_else(|| "[]".into())
                    ).unwrap_or_default(),
                    "tps_short_atteints":    serde_json::from_str::<Vec<String>>(
                        &row.get::<Option<String>, _>("tps_short_atteints").unwrap_or_else(|| "[]".into())
                    ).unwrap_or_default(),
                    "heure_entree":          row.get::<Option<i64>, _>("heure_entree"),
                })
            })
            .collect();

        Ok(signaux)
    }

    /// Retourne les N derniers signaux d'un asset pour injection dans les prompts LLM.
    /// Ne propage pas les erreurs — retourne vec![] si la DB est indisponible.
    pub async fn obtenir_contexte_llm(&self, asset: &str, limit: i64) -> Vec<serde_json::Value> {
        let rows = sqlx::query(
            "SELECT direction, timeframe, score, prix_entree, statut, cree_le
             FROM signaux WHERE asset = ? ORDER BY cree_le DESC LIMIT ?",
        )
        .bind(asset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(rows) => rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "direction":   row.get::<String, _>("direction"),
                        "timeframe":   row.get::<String, _>("timeframe"),
                        "score":       row.get::<f64, _>("score"),
                        "prix_entree": row.get::<f64, _>("prix_entree"),
                        "statut":      row.get::<String, _>("statut"),
                        "cree_le":     row.get::<i64, _>("cree_le"),
                    })
                })
                .collect(),
            Err(_) => vec![],
        }
    }

    /// Retourne les N derniers signaux SMC d'un asset pour le filtre LLM pré-sauvegarde.
    /// Sans propagation d'erreur — retourne vec![] si la DB est indisponible.
    pub async fn obtenir_historique_smc(
        &self,
        asset: &str,
        limit: i64,
    ) -> Vec<(String, String, f64, String)> {
        let rows = sqlx::query(
            "SELECT direction, timeframe, score, statut
             FROM signaux WHERE asset = ?
             ORDER BY cree_le DESC LIMIT ?",
        )
        .bind(asset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(rows) => rows
                .iter()
                .map(|row| {
                    (
                        row.get::<String, _>("direction"),
                        row.get::<String, _>("timeframe"),
                        row.get::<f64, _>("score"),
                        row.get::<String, _>("statut"),
                    )
                })
                .collect(),
            Err(_) => vec![],
        }
    }
}
