use common::{Result, TradingError};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

#[derive(Serialize, Clone)]
pub struct RocketSignal {
    pub id: i64,
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
    pub verdict: Option<String>,
    pub prix_verdict: Option<f64>,
    pub cree_le: String,
    pub maj_le: Option<String>,
}

pub struct NouveauRocket {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
}

fn row_to_signal(row: &sqlx::sqlite::SqliteRow) -> RocketSignal {
    RocketSignal {
        id:           row.get("id"),
        ticker:       row.get("ticker"),
        phase:        row.get("phase"),
        score:        row.get("score"),
        prix_entree:  row.get("prix_entree"),
        stop_loss:    row.get("stop_loss"),
        target:       row.get("target"),
        target2:      row.get("target2"),
        target3:      row.get("target3"),
        ratio_volume: row.get("ratio_volume"),
        atr_ratio:    row.get("atr_ratio"),
        rsi:          row.get("rsi"),
        verdict:      row.get("verdict"),
        prix_verdict: row.get("prix_verdict"),
        cree_le:      row.get("cree_le"),
        maj_le:       row.get("maj_le"),
    }
}

/// Insère uniquement si aucun signal identique (ticker+phase) dans les 6 dernières heures.
pub async fn sauvegarder(pool: &SqlitePool, s: &NouveauRocket) -> Result<Option<i64>> {
    let id = sqlx::query(
        "INSERT INTO rockets_signaux
         (ticker, phase, score, prix_entree, stop_loss, target, target2, target3, ratio_volume, atr_ratio, rsi)
         SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
         WHERE NOT EXISTS (
           SELECT 1 FROM rockets_signaux
           WHERE ticker = ? AND phase = ? AND cree_le >= datetime('now', '-6 hours')
         )",
    )
    .bind(&s.ticker)
    .bind(&s.phase)
    .bind(s.score)
    .bind(s.prix_entree)
    .bind(s.stop_loss)
    .bind(s.target)
    .bind(s.target2)
    .bind(s.target3)
    .bind(s.ratio_volume)
    .bind(s.atr_ratio)
    .bind(s.rsi)
    .bind(&s.ticker)
    .bind(&s.phase)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?
    .last_insert_rowid();

    Ok(if id > 0 { Some(id) } else { None })
}

pub async fn lister_ouverts(pool: &SqlitePool) -> Result<Vec<RocketSignal>> {
    let rows = sqlx::query(
        "SELECT id, ticker, phase, score, prix_entree, stop_loss, target, target2, target3,
                ratio_volume, atr_ratio, rsi, verdict, prix_verdict, cree_le, maj_le
         FROM rockets_signaux WHERE verdict IS NULL ORDER BY cree_le DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(row_to_signal).collect())
}

pub async fn maj_verdict(pool: &SqlitePool, id: i64, verdict: &str, prix: f64) -> Result<()> {
    sqlx::query(
        "UPDATE rockets_signaux
         SET verdict = ?, prix_verdict = ?, maj_le = datetime('now')
         WHERE id = ?",
    )
    .bind(verdict)
    .bind(prix)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn marquer_expires(pool: &SqlitePool) -> Result<u64> {
    let res = sqlx::query(
        "UPDATE rockets_signaux SET verdict = 'expire', maj_le = datetime('now')
         WHERE verdict IS NULL AND cree_le <= datetime('now', '-4 hours')",
    )
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(res.rows_affected())
}

pub async fn historique(pool: &SqlitePool, limite: i64) -> Result<Vec<RocketSignal>> {
    let rows = sqlx::query(
        "SELECT id, ticker, phase, score, prix_entree, stop_loss, target, target2, target3,
                ratio_volume, atr_ratio, rsi, verdict, prix_verdict, cree_le, maj_le
         FROM rockets_signaux ORDER BY cree_le DESC LIMIT ?",
    )
    .bind(limite)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(row_to_signal).collect())
}
