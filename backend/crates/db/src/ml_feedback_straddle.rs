//! Stats détaillées Straddle pour ML Insights (P8).
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StraddleCategorieStats {
    pub categorie: String,
    pub nb_trades: i64,
    pub win_rate: f64,
    pub pnl_r_moyen: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StraddleConvictionStats {
    pub tranche: String,
    pub nb_trades: i64,
    pub win_rate: f64,
}

/// Win rate par catégorie Straddle (Annonce, Volatilité, Kill Zone…).
pub async fn stats_par_categorie(pool: &SqlitePool) -> Result<Vec<StraddleCategorieStats>> {
    let rows = sqlx::query(
        "SELECT categorie,
                COUNT(*) as nb_trades,
                COALESCE(SUM(CASE WHEN gagnant = 1 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(pnl_r), 0.0) as pnl_r_moyen
         FROM straddle_feedback
         WHERE verdict IS NOT NULL
         GROUP BY categorie
         ORDER BY nb_trades DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get("nb_gagnants");
            StraddleCategorieStats {
                categorie: r.get("categorie"),
                nb_trades: nb,
                win_rate: if nb > 0 { wins as f64 * 100.0 / nb as f64 } else { 0.0 },
                pnl_r_moyen: r.get("pnl_r_moyen"),
            }
        })
        .collect())
}

/// Win rate par tranche de score LLM pour Straddle.
pub async fn stats_score_llm(pool: &SqlitePool) -> Result<Vec<StraddleConvictionStats>> {
    let rows = sqlx::query(
        "SELECT CASE
                    WHEN score_llm < 60 THEN '<60'
                    WHEN score_llm < 70 THEN '60-70'
                    WHEN score_llm < 80 THEN '70-80'
                    ELSE '80+'
                END as tranche,
                COUNT(*) as nb_trades,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate
         FROM straddle_feedback
         WHERE verdict IS NOT NULL AND score_llm IS NOT NULL
         GROUP BY tranche
         ORDER BY MIN(score_llm)",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| StraddleConvictionStats {
            tranche: r.get("tranche"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
        })
        .collect())
}
