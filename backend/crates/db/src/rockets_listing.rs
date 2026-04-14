use common::{Result, TradingError};
use sqlx::SqlitePool;

use crate::rockets::{row_to_signal, RocketSignal};

const SELECT_COLS: &str =
    "SELECT id, ticker, phase, score, prix_entree, stop_loss, target, target2, target3,
            ratio_volume, atr_ratio, atr14, rsi, statut, prix_peak, verdict, prix_verdict, cree_le, maj_le,
            llm_valide, llm_conviction, llm_raison,
            trailing_coeff, pct_tp1, pct_tp2, pct_trailing
     FROM rockets_signaux";

pub async fn lister_ouverts(pool: &SqlitePool) -> Result<Vec<RocketSignal>> {
    let sql = format!("{} WHERE statut = 'ouvert' ORDER BY cree_le DESC", SELECT_COLS);
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(row_to_signal).collect())
}

pub async fn lister_en_attente(pool: &SqlitePool) -> Result<Vec<RocketSignal>> {
    let sql = format!("{} WHERE statut = 'attente' ORDER BY cree_le DESC", SELECT_COLS);
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(row_to_signal).collect())
}

/// Trades actifs (ouvert ou attente) — utilisé par le frontend pour la vue "Trades en cours".
pub async fn lister_actifs(pool: &SqlitePool) -> Result<Vec<RocketSignal>> {
    let sql = format!(
        "{} WHERE statut IN ('ouvert', 'attente') ORDER BY cree_le DESC",
        SELECT_COLS
    );
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(row_to_signal).collect())
}
