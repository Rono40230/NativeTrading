//! Stats détaillées Rockets pour ML Insights (P8).
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketsPhaseStats {
    pub phase: String,
    pub nb_trades: i64,
    pub win_rate: f64,
    pub pnl_r_moyen: f64,
    pub conviction_win: Option<f64>,
    pub conviction_lose: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketsConvictionStats {
    pub tranche: String,
    pub nb_trades: i64,
    pub win_rate: f64,
}

/// Win rate + conviction LLM par phase Rockets.
pub async fn stats_par_phase(pool: &SqlitePool) -> Result<Vec<RocketsPhaseStats>> {
    let rows = sqlx::query(
        "SELECT phase,
                COUNT(*) as nb_trades,
                COALESCE(SUM(CASE WHEN gagnant = 1 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(COALESCE(pnl_r, -1.0)), 0.0) as pnl_r_moyen,
                AVG(CASE WHEN gagnant = 1 THEN conviction_llm END) as conviction_win,
                AVG(CASE WHEN gagnant = 0 THEN conviction_llm END) as conviction_lose
         FROM rockets_feedback
         WHERE verdict IS NOT NULL AND verdict NOT IN ('invalide','expire')
         GROUP BY phase
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
            RocketsPhaseStats {
                phase: r.get("phase"),
                nb_trades: nb,
                win_rate: if nb > 0 { wins as f64 * 100.0 / nb as f64 } else { 0.0 },
                pnl_r_moyen: r.get("pnl_r_moyen"),
                conviction_win: r.get("conviction_win"),
                conviction_lose: r.get("conviction_lose"),
            }
        })
        .collect())
}

/// Win rate par tranche de conviction LLM pour Rockets.
pub async fn stats_conviction_llm(pool: &SqlitePool) -> Result<Vec<RocketsConvictionStats>> {
    let rows = sqlx::query(
        "SELECT CASE
                    WHEN conviction_llm < 60 THEN '<60'
                    WHEN conviction_llm < 70 THEN '60-70'
                    WHEN conviction_llm < 80 THEN '70-80'
                    ELSE '80+'
                END as tranche,
                COUNT(*) as nb_trades,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate
         FROM rockets_feedback
         WHERE verdict IS NOT NULL AND verdict NOT IN ('invalide','expire') AND conviction_llm IS NOT NULL
         GROUP BY tranche
         ORDER BY MIN(conviction_llm)",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| RocketsConvictionStats {
            tranche: r.get("tranche"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
        })
        .collect())
}
