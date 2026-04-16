//! Module DB pour la mémoire apprenante de la stratégie SMC Directionnel.
//!
//! Chaque signal SMC est enregistré à sa création (verdict=NULL).
//! Le job `smc_feedback_job` réconcilie le verdict après clôture.
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types publics ─────────────────────────────────────────────────────────────

pub struct NouveauFeedbackSmc<'a> {
    pub signal_id: &'a str,
    pub asset: &'a str,
    pub timeframe: &'a str,
    pub timestamp_signal: i64,
    pub categorie: &'a str,
    pub session_active: &'a str,
    pub score_smc: f64,
    pub confiance_ml: f64,
    pub kill_zone_active: bool,
    pub sweep_detecte: bool,
    pub conviction_llm: i64,
    pub atr14: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmcFeedbackRow {
    pub id: i64,
    pub signal_id: String,
    pub asset: String,
    pub timeframe: String,
    pub timestamp_signal: i64,
    pub categorie: String,
    pub session_active: String,
    pub score_smc: f64,
    pub confiance_ml: f64,
    pub kill_zone_active: i64,
    pub sweep_detecte: i64,
    pub conviction_llm: i64,
    pub atr14: f64,
    pub verdict: Option<String>,
    pub pnl_r: Option<f64>,
    pub gagnant: Option<i64>,
    pub duree_trade_min: Option<i64>,
    pub ferme_le: Option<i64>,
    pub cree_le: i64,
    // P9 — enrichissement
    pub prix_entree_reel: Option<f64>,
    pub prix_sortie_reel: Option<f64>,
    pub session_sortie: Option<String>,
    pub notes_trader: Option<String>,
}

// ── Écriture ─────────────────────────────────────────────────────────────────

/// Insère un feedback initial (signal ouvert). Ignoré si `signal_id` existe déjà.
pub async fn inserer_feedback(pool: &SqlitePool, fb: &NouveauFeedbackSmc<'_>) -> Result<i64> {
    let row = sqlx::query(
        "INSERT OR IGNORE INTO smc_feedback
         (signal_id, asset, timeframe, timestamp_signal, categorie,
          session_active, score_smc, confiance_ml, kill_zone_active,
          sweep_detecte, conviction_llm, atr14)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(fb.signal_id)
    .bind(fb.asset)
    .bind(fb.timeframe)
    .bind(fb.timestamp_signal)
    .bind(fb.categorie)
    .bind(fb.session_active)
    .bind(fb.score_smc)
    .bind(fb.confiance_ml)
    .bind(fb.kill_zone_active as i64)
    .bind(fb.sweep_detecte as i64)
    .bind(fb.conviction_llm)
    .bind(fb.atr14)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.map(|r| r.get::<i64, _>("id")).unwrap_or(0))
}

/// Réconcilie un feedback clôturé : verdict, pnl_r, gagnant, durée.
pub async fn reconcilier_feedback(
    pool: &SqlitePool,
    signal_id: &str,
    verdict: &str,
    prix_entree: f64,
    prix_verdict: f64,
    atr14: f64,
    timestamp_signal: i64,
) -> Result<()> {
    let pnl_r = if atr14 > 0.0 {
        (prix_verdict - prix_entree) / atr14
    } else {
        0.0
    };
    let gagnant: i64 = if pnl_r > 0.0 { 1 } else { 0 };
    let ts_now = chrono::Utc::now().timestamp();
    let duree_min = (ts_now - timestamp_signal) / 60;

    sqlx::query(
        "UPDATE smc_feedback
         SET verdict = ?, pnl_r = ?, gagnant = ?,
             duree_trade_min = ?, ferme_le = unixepoch(),
             prix_entree_reel = ?, prix_sortie_reel = ?, session_sortie = ?
         WHERE signal_id = ? AND verdict IS NULL",
    )
    .bind(verdict)
    .bind(pnl_r)
    .bind(gagnant)
    .bind(duree_min)
    .bind(prix_entree)
    .bind(prix_verdict)
    .bind(crate::session_sortie_courante(ts_now))
    .bind(signal_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(())
}

// ── Lecture ──────────────────────────────────────────────────────────────────

/// Retourne les N feedbacks clôturés les plus récents pour asset+tf+catégorie.
/// Utilisé par le prompt few-shot.
pub async fn lister_feedbacks_asset_categorie(
    pool: &SqlitePool,
    asset: &str,
    timeframe: &str,
    categorie: &str,
    limit: i64,
) -> Result<Vec<SmcFeedbackRow>> {
    let rows = sqlx::query(
        "SELECT id, signal_id, asset, timeframe, timestamp_signal, categorie,
                session_active, score_smc, confiance_ml, kill_zone_active,
                sweep_detecte, conviction_llm, atr14,
                verdict, pnl_r, gagnant, duree_trade_min, ferme_le, cree_le
         FROM smc_feedback
         WHERE asset = ? AND timeframe = ? AND categorie = ? AND verdict IS NOT NULL
         ORDER BY cree_le DESC LIMIT ?",
    )
    .bind(asset)
    .bind(timeframe)
    .bind(categorie)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_row).collect())
}

/// Filtre par asset+tf élargi (pour endpoint feedback historique).
pub async fn lister_feedbacks_like(
    pool: &SqlitePool,
    asset_like: &str,
    tf_like: &str,
    limit: i64,
) -> Result<Vec<SmcFeedbackRow>> {
    let rows = sqlx::query(
        "SELECT id, signal_id, asset, timeframe, timestamp_signal, categorie,
                session_active, score_smc, confiance_ml, kill_zone_active,
                sweep_detecte, conviction_llm, atr14,
                verdict, pnl_r, gagnant, duree_trade_min, ferme_le, cree_le
         FROM smc_feedback
         WHERE asset LIKE ? AND timeframe LIKE ?
         ORDER BY cree_le DESC LIMIT ?",
    )
    .bind(asset_like)
    .bind(tf_like)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_row).collect())
}

/// Statistiques globales pour le monitoring ML.
pub async fn stats_globales(pool: &SqlitePool) -> Result<serde_json::Value> {
    let row = sqlx::query(
        "SELECT COUNT(*) as nb_total,
                SUM(CASE WHEN verdict IS NOT NULL THEN 1 ELSE 0 END) as nb_clos,
                SUM(CASE WHEN gagnant = 1 AND verdict IS NOT NULL THEN 1 ELSE 0 END) as nb_gagnants,
                SUM(CASE WHEN gagnant = 0 AND verdict IS NOT NULL THEN 1 ELSE 0 END) as nb_perdants,
                AVG(CASE WHEN verdict IS NOT NULL THEN gagnant END) as win_rate,
                AVG(CASE WHEN verdict IS NOT NULL THEN pnl_r END) as pnl_moyen_r
         FROM smc_feedback",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let nb_total: i64 = row.get("nb_total");
    let nb_clos: i64 = row.get("nb_clos");
    Ok(serde_json::json!({
        "nb_signals_total":      nb_total,
        "nb_feedbacks_clotures": nb_clos,
        "nb_gagnants":           row.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0),
        "nb_perdants":           row.get::<Option<i64>, _>("nb_perdants").unwrap_or(0),
        "nb_invalides":          nb_total - nb_clos,
        "win_rate_global":       row.get::<Option<f64>, _>("win_rate").unwrap_or(0.0),
        "pnl_moyen_r":           row.get::<Option<f64>, _>("pnl_moyen_r"),
        "derniere_maj":          chrono::Utc::now().timestamp(),
    }))
}

/// Statistiques par catégorie pour le monitoring ML.
pub async fn stats_par_categorie(pool: &SqlitePool) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        "SELECT categorie,
                COUNT(*) as nb_trades,
                SUM(gagnant) as nb_gagnants,
                AVG(CASE WHEN gagnant = 1 THEN conviction_llm END) as conviction_win,
                AVG(CASE WHEN gagnant = 0 THEN conviction_llm END) as conviction_lose,
                AVG(pnl_r) as pnl_r_moyen
         FROM smc_feedback
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
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            serde_json::json!({
                "categorie":       r.get::<String, _>("categorie"),
                "nb_trades":       nb,
                "win_rate":        if nb > 0 { wins as f64 / nb as f64 } else { 0.0 },
                "conviction_win":  r.get::<Option<f64>, _>("conviction_win"),
                "conviction_lose": r.get::<Option<f64>, _>("conviction_lose"),
                "pnl_r_moyen":     r.get::<Option<f64>, _>("pnl_r_moyen"),
            })
        })
        .collect())
}

// ── Helper interne ────────────────────────────────────────────────────────────

fn mapper_row(r: &sqlx::sqlite::SqliteRow) -> SmcFeedbackRow {
    SmcFeedbackRow {
        id: r.get("id"),
        signal_id: r.get("signal_id"),
        asset: r.get("asset"),
        timeframe: r.get("timeframe"),
        timestamp_signal: r.get("timestamp_signal"),
        categorie: r.get("categorie"),
        session_active: r.get("session_active"),
        score_smc: r.get("score_smc"),
        confiance_ml: r.get("confiance_ml"),
        kill_zone_active: r.get("kill_zone_active"),
        sweep_detecte: r.get("sweep_detecte"),
        conviction_llm: r.get("conviction_llm"),
        atr14: r.get("atr14"),
        verdict: r.get("verdict"),
        pnl_r: r.get("pnl_r"),
        gagnant: r.get("gagnant"),
        duree_trade_min: r.get("duree_trade_min"),
        ferme_le: r.get("ferme_le"),
        cree_le: r.get("cree_le"),
        prix_entree_reel: r.get("prix_entree_reel"),
        prix_sortie_reel: r.get("prix_sortie_reel"),
        session_sortie: r.get("session_sortie"),
        notes_trader: r.get("notes_trader"),
    }
}
