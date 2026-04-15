//! Statistiques SMC feedback — courbe equity simulée.

use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

#[derive(Debug, serde::Serialize)]
pub struct EquityPoint {
    pub asset: String,
    pub verdict: String,
    pub pnl_r: f64,
    pub equity_cumulee: f64,
    pub ferme_le: i64,
}

/// Retourne la série equity simulée depuis `smc_feedback` (trades clôturés avec pnl_r).
pub async fn courbe_equity(
    pool: &SqlitePool,
    capital_initial: f64,
    risk_montant: f64,
) -> Result<Vec<EquityPoint>> {
    let rows = sqlx::query(
        "SELECT asset, LOWER(verdict) as verdict, pnl_r, ferme_le
         FROM smc_feedback
         WHERE verdict IS NOT NULL
           AND pnl_r IS NOT NULL
           AND ferme_le IS NOT NULL
         ORDER BY ferme_le ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let mut equity = capital_initial;
    let mut points = Vec::with_capacity(rows.len());

    for r in &rows {
        let pnl_r: f64 = r.get::<Option<f64>, _>("pnl_r").unwrap_or(0.0);
        equity += pnl_r * risk_montant;
        points.push(EquityPoint {
            asset: r.get("asset"),
            verdict: r.get("verdict"),
            pnl_r,
            equity_cumulee: equity,
            ferme_le: r.get::<Option<i64>, _>("ferme_le").unwrap_or(0),
        });
    }

    Ok(points)
}
