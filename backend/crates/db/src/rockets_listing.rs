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

/// N derniers signaux clôturés hors expire pour un ticker donné.
/// Utilisé par le filtre LLM pour contextualiser chaque nouveau signal.
pub async fn historique_ticker(pool: &SqlitePool, ticker: &str, limite: i64) -> Vec<RocketSignal> {
    let sql = format!(
        "{} WHERE ticker = ? AND statut = 'ferme' AND verdict IS NOT NULL AND verdict != 'expire'
         ORDER BY cree_le DESC LIMIT ?",
        SELECT_COLS
    );
    let rows = sqlx::query(&sql)
        .bind(ticker)
        .bind(limite)
        .fetch_all(pool)
        .await;
    match rows {
        Ok(rows) => rows.iter().map(row_to_signal).collect(),
        Err(_) => vec![],
    }
}

/// Supprime un signal actif (statut attente ou ouvert).
/// Retourne true si un signal a été supprimé, false si introuvable ou déjà clôturé.
pub async fn supprimer(pool: &SqlitePool, id: i64) -> Result<bool> {
    let res = sqlx::query(
        "DELETE FROM rockets_signaux WHERE id = ? AND statut IN ('attente', 'ouvert')",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(res.rows_affected() > 0)
}

/// Enregistre l'atteinte d'un TP partiel (TP1/TP2) sans clôturer la position.
/// Met à jour le prix peak et la date de mise à jour.
pub async fn enregistrer_tp_partiel(
    pool: &SqlitePool,
    id: i64,
    _verdict: &str,
    prix: f64,
) -> Result<()> {
    sqlx::query(
        "UPDATE rockets_signaux
         SET prix_peak = MAX(COALESCE(prix_peak, 0.0), ?), maj_le = datetime('now')
         WHERE id = ? AND statut = 'ouvert'",
    )
    .bind(prix)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
