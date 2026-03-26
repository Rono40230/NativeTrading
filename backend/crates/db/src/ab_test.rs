use crate::Database;
use common::{Result, TradingError};
use serde::Serialize;
use sqlx::Row;

#[derive(Debug, Serialize)]
pub struct StatVariante {
    pub strategie: String,
    pub nb_total: i64,
    pub nb_wins: i64,
    pub nb_pertes: i64,
    pub win_rate: f64,
    pub conviction_moy: f64,
    pub score_moy: f64,
}

impl Database {
    /// Agrège les signaux clôturés par stratégie pour comparer les variantes A/B.
    /// Win = verdict IN (TP1, TP2, TP3) / Perte = verdict = 'SL'.
    pub async fn stats_ab_test(&self) -> Result<Vec<StatVariante>> {
        let rows = sqlx::query(
            r#"
            SELECT
                strategie,
                COUNT(*) as nb_total,
                SUM(CASE WHEN verdict IN ('TP1','TP2','TP3') THEN 1 ELSE 0 END) as nb_wins,
                SUM(CASE WHEN verdict = 'SL' THEN 1 ELSE 0 END) as nb_pertes,
                AVG(COALESCE(CAST(llm_conviction AS REAL), 0)) as conviction_moy,
                AVG(score) as score_moy
            FROM signaux
            WHERE verdict IS NOT NULL AND verdict != 'expire'
            GROUP BY strategie
            ORDER BY nb_total DESC
            "#,
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                let nb_wins: i64 = r.get("nb_wins");
                let nb_pertes: i64 = r.get("nb_pertes");
                let win_rate = if nb_wins + nb_pertes > 0 {
                    nb_wins as f64 / (nb_wins + nb_pertes) as f64 * 100.0
                } else {
                    0.0
                };
                StatVariante {
                    strategie: r.get("strategie"),
                    nb_total: r.get("nb_total"),
                    nb_wins,
                    nb_pertes,
                    win_rate,
                    conviction_moy: r.get::<f64, _>("conviction_moy"),
                    score_moy: r.get::<f64, _>("score_moy"),
                }
            })
            .collect())
    }
}
