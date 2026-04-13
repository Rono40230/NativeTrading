//! Module DB pour la mémoire apprenante de la stratégie Rockets.
//!
//! Chaque signal Rockets est enregistré ici à sa création (verdict=NULL).
//! Le worker `rockets_suivi` met à jour le verdict lors de la clôture TP/SL.
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types publics ─────────────────────────────────────────────────────────────

/// Données initiales insérées à la création du signal.
pub struct NouveauFeedbackRocket {
    pub signal_id: i64,
    pub ticker: String,
    pub phase: String,
    pub session_active: String,
    pub timestamp_signal: i64,
    pub score_scan: i64,
    pub conviction_llm: i64,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
}

/// Lecture complète — pour le prompt few-shot et le monitoring ML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketsFeedbackRow {
    pub id: i64,
    pub signal_id: i64,
    pub ticker: String,
    pub phase: String,
    pub session_active: String,
    pub timestamp_signal: i64,
    pub score_scan: i64,
    pub conviction_llm: i64,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
    pub verdict: Option<String>,
    pub pnl_r: Option<f64>,
    pub gagnant: Option<i64>,
    pub duree_trade_min: Option<i64>,
    pub ferme_le: Option<i64>,
    pub cree_le: i64,
}

// ── Écriture ─────────────────────────────────────────────────────────────────

/// Insère un feedback vide (signal ouvert). Ignoré si `signal_id` existe déjà.
pub async fn inserer_feedback(pool: &SqlitePool, fb: &NouveauFeedbackRocket) -> Result<i64> {
    let row = sqlx::query(
        "INSERT OR IGNORE INTO rockets_feedback
         (signal_id, ticker, phase, session_active, timestamp_signal,
          score_scan, conviction_llm, ratio_volume, atr_ratio, rsi)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(fb.signal_id)
    .bind(&fb.ticker)
    .bind(&fb.phase)
    .bind(&fb.session_active)
    .bind(fb.timestamp_signal)
    .bind(fb.score_scan)
    .bind(fb.conviction_llm)
    .bind(fb.ratio_volume)
    .bind(fb.atr_ratio)
    .bind(fb.rsi)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.map(|r| r.get::<i64, _>("id")).unwrap_or(0))
}

/// Met à jour le feedback avec le verdict du trade.
/// Appelé par `rockets_suivi` lors de la clôture TP3/SL ou d'un TP partiel final.
pub async fn maj_feedback_verdict(
    pool: &SqlitePool,
    signal_id: i64,
    verdict: &str,
    prix_entree: f64,
    prix_verdict: f64,
    atr14: f64,
    timestamp_signal: i64,
) -> Result<()> {
    // Normaliser en minuscules pour garantir la cohérence en DB
    let verdict_lc = verdict.to_lowercase();
    let now = chrono::Utc::now().timestamp();
    let duree_min = (now - timestamp_signal) / 60;

    // Ne pas calculer pnl_r pour les signaux jamais entrés en position
    let (pnl_r, gagnant): (Option<f64>, i64) = if verdict_lc == "invalide" || verdict_lc == "expire" {
        (None, 0)
    } else {
        let p = if atr14 > 0.0 {
            (prix_verdict - prix_entree) / atr14
        } else {
            0.0
        };
        let g = if verdict_lc.starts_with("tp") { 1 } else { 0 };
        (Some(p), g)
    };

    sqlx::query(
        "UPDATE rockets_feedback
         SET verdict = ?, pnl_r = ?, gagnant = ?,
             duree_trade_min = ?, ferme_le = ?
         WHERE signal_id = ?",
    )
    .bind(&verdict_lc)
    .bind(pnl_r)
    .bind(gagnant)
    .bind(duree_min)
    .bind(now)
    .bind(signal_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(())
}

// ── Lecture ──────────────────────────────────────────────────────────────────

/// Retourne les N feedbacks clôturés les plus récents pour un ticker+phase.
/// Utilisé par le prompt few-shot.
pub async fn lister_recents_ticker_phase(
    pool: &SqlitePool,
    ticker: &str,
    phase: &str,
    limit: i64,
) -> Result<Vec<RocketsFeedbackRow>> {
    let rows = sqlx::query(
        "SELECT id, signal_id, ticker, phase, session_active, timestamp_signal,
                score_scan, conviction_llm, ratio_volume, atr_ratio, rsi,
                verdict, pnl_r, gagnant, duree_trade_min, ferme_le, cree_le
         FROM rockets_feedback
         WHERE ticker = ? AND phase = ? AND verdict IS NOT NULL
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(ticker)
    .bind(phase)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_row).collect())
}

/// Retourne un pool de feedbacks clôturés sur une phase, tous tickers confondus,
/// pour permettre une sélection par similarité de profil côté appelant.
pub async fn lister_pool_phase(
    pool: &SqlitePool,
    phase: &str,
    limit: i64,
) -> Result<Vec<RocketsFeedbackRow>> {
    let rows = sqlx::query(
        "SELECT id, signal_id, ticker, phase, session_active, timestamp_signal,
                score_scan, conviction_llm, ratio_volume, atr_ratio, rsi,
                verdict, pnl_r, gagnant, duree_trade_min, ferme_le, cree_le
         FROM rockets_feedback
         WHERE phase = ? AND verdict IS NOT NULL
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(phase)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_row).collect())
}

// ── Helper interne ────────────────────────────────────────────────────────────

fn mapper_row(r: &sqlx::sqlite::SqliteRow) -> RocketsFeedbackRow {
    RocketsFeedbackRow {
        id: r.get("id"),
        signal_id: r.get("signal_id"),
        ticker: r.get("ticker"),
        phase: r.get("phase"),
        session_active: r.get("session_active"),
        timestamp_signal: r.get("timestamp_signal"),
        score_scan: r.get("score_scan"),
        conviction_llm: r.get("conviction_llm"),
        ratio_volume: r.get("ratio_volume"),
        atr_ratio: r.get("atr_ratio"),
        rsi: r.get("rsi"),
        verdict: r.get("verdict"),
        pnl_r: r.get("pnl_r"),
        gagnant: r.get("gagnant"),
        duree_trade_min: r.get("duree_trade_min"),
        ferme_le: r.get("ferme_le"),
        cree_le: r.get("cree_le"),
    }
}

// ── GET /api/rockets/feedback (liste filtrée avec LIKE) ───────────────────────

pub async fn lister_recents_ticker_phase_like(
    pool: &sqlx::SqlitePool,
    ticker_like: &str,
    phase_like: &str,
    limit: i64,
) -> Result<Vec<RocketsFeedbackRow>> {
    let rows = sqlx::query(
        "SELECT * FROM rockets_feedback
         WHERE ticker LIKE ? AND phase LIKE ?
         ORDER BY cree_le DESC LIMIT ?",
    )
    .bind(ticker_like)
    .bind(phase_like)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_row).collect())
}


