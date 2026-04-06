//! Module DB pour la mémoire apprenante de la stratégie Straddle.
//!
//! Chaque signal Straddle est enregistré ici à sa création (verdict=NULL).
//! Le job `straddle_feedback_job` met à jour le verdict lors de la réconciliation.
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types publics ─────────────────────────────────────────────────────────────

/// Données initiales insérées à la création du signal.
pub struct NouveauFeedback<'a> {
    pub signal_id: &'a str,
    pub pic_id: Option<i64>,
    pub asset: &'a str,
    pub timeframe: &'a str,
    pub timestamp_signal: i64,
    pub categorie: &'a str,
    pub evenement_nom: Option<&'a str>,
    pub session_active: Option<&'a str>,
    pub ratio_atr: f64,
    pub score_llm: f64,
}

/// Lecture complète — pour le prompt few-shot et le monitoring ML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StraddleFeedbackRow {
    pub id: i64,
    pub signal_id: String,
    pub pic_id: Option<i64>,
    pub asset: String,
    pub timeframe: String,
    pub timestamp_signal: i64,
    pub categorie: String,
    pub evenement_nom: Option<String>,
    pub session_active: Option<String>,
    pub ratio_atr: f64,
    pub score_llm: f64,
    pub verdict: Option<String>,
    pub amplitude_reelle_pct: Option<f64>,
    pub duree_trade_min: Option<i64>,
    pub pnl_r: Option<f64>,
    pub gagnant: Option<i64>,
    pub cree_le: i64,
    pub ferme_le: Option<i64>,
}

// ── Écriture ─────────────────────────────────────────────────────────────────

/// Insère un feedback vide (signal ouvert). Ignoré si `signal_id` existe déjà.
pub async fn inserer_feedback(pool: &SqlitePool, fb: &NouveauFeedback<'_>) -> Result<i64> {
    let row = sqlx::query(
        "INSERT OR IGNORE INTO straddle_feedback
         (signal_id, pic_id, asset, timeframe, timestamp_signal,
          categorie, evenement_nom, session_active, ratio_atr, score_llm)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(fb.signal_id)
    .bind(fb.pic_id)
    .bind(fb.asset)
    .bind(fb.timeframe)
    .bind(fb.timestamp_signal)
    .bind(fb.categorie)
    .bind(fb.evenement_nom)
    .bind(fb.session_active)
    .bind(fb.ratio_atr)
    .bind(fb.score_llm)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.map(|r| r.get::<i64, _>("id")).unwrap_or(0))
}

/// Met à jour le feedback avec le verdict du trade (appelé par le job de réconciliation).
pub async fn maj_feedback_verdict(
    pool: &SqlitePool,
    signal_id: &str,
    verdict: &str,
    prix_entree: f64,
    prix_verdict: f64,
    risque: f64,
    timestamp_signal: i64,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let duree_min = (now - timestamp_signal) / 60;
    let amplitude_pct = if prix_entree > 0.0 {
        (prix_verdict - prix_entree).abs() / prix_entree * 100.0
    } else {
        0.0
    };
    let pnl_r = if risque > 0.0 {
        let delta = prix_verdict - prix_entree;
        delta / risque
    } else {
        0.0
    };
    let gagnant: i64 = if verdict.starts_with("tp") { 1 } else { 0 };

    sqlx::query(
        "UPDATE straddle_feedback
         SET verdict = ?, amplitude_reelle_pct = ?, duree_trade_min = ?,
             pnl_r = ?, gagnant = ?, ferme_le = ?
         WHERE signal_id = ?",
    )
    .bind(verdict)
    .bind(amplitude_pct)
    .bind(duree_min)
    .bind(pnl_r)
    .bind(gagnant)
    .bind(now)
    .bind(signal_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(())
}

// ── Lecture ──────────────────────────────────────────────────────────────────

/// Retourne les N feedbacks clôturés les plus récents pour un asset+catégorie.
/// Utilisé par le prompt few-shot (Étape 4).
pub async fn lister_recents_asset_categorie(
    pool: &SqlitePool,
    asset: &str,
    categorie: &str,
    limit: i64,
) -> Result<Vec<StraddleFeedbackRow>> {
    let rows = sqlx::query(
        "SELECT id, signal_id, pic_id, asset, timeframe, timestamp_signal,
                categorie, evenement_nom, session_active, ratio_atr, score_llm,
                verdict, amplitude_reelle_pct, duree_trade_min, pnl_r, gagnant,
                cree_le, ferme_le
         FROM straddle_feedback
         WHERE asset = ? AND categorie = ? AND verdict IS NOT NULL
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(asset)
    .bind(categorie)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_row).collect())
}

/// Statistiques agrégées par catégorie pour un asset — pour le monitoring ML.
/// Retourne le nombre de trades, win rate, score LLM moyen, pnl_r moyen.
pub async fn stats_monitoring(pool: &SqlitePool, asset: &str) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        "SELECT categorie,
                COUNT(*) as nb_trades,
                SUM(gagnant) as nb_gagnants,
                AVG(CASE WHEN gagnant = 1 THEN score_llm END) as score_moyen_win,
                AVG(CASE WHEN gagnant = 0 THEN score_llm END) as score_moyen_lose,
                AVG(pnl_r) as pnl_r_moyen
         FROM straddle_feedback
         WHERE asset = ? AND verdict IS NOT NULL
         GROUP BY categorie
         ORDER BY nb_trades DESC",
    )
    .bind(asset)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            serde_json::json!({
                "categorie":          r.get::<String, _>("categorie"),
                "nb_trades":          nb,
                "win_rate":           if nb > 0 { wins as f64 / nb as f64 } else { 0.0 },
                "score_llm_win":      r.get::<Option<f64>, _>("score_moyen_win"),
                "score_llm_lose":     r.get::<Option<f64>, _>("score_moyen_lose"),
                "pnl_r_moyen":        r.get::<Option<f64>, _>("pnl_r_moyen"),
            })
        })
        .collect())
}

/// Statistiques globales (tous assets) — pour le endpoint `/api/straddle/monitoring-ml`.
pub async fn stats_globales(pool: &SqlitePool) -> Result<serde_json::Value> {
    let row = sqlx::query(
        "SELECT COUNT(*) as nb_total,
                SUM(CASE WHEN verdict IS NOT NULL THEN 1 ELSE 0 END) as nb_clos,
                AVG(CASE WHEN verdict IS NOT NULL THEN gagnant END) as win_rate,
                AVG(CASE WHEN verdict IS NOT NULL THEN pnl_r END) as pnl_moyen_r
         FROM straddle_feedback",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(serde_json::json!({
        "nb_signals_total":     row.get::<i64, _>("nb_total"),
        "nb_feedbacks_clotures": row.get::<i64, _>("nb_clos"),
        "win_rate_global":      row.get::<Option<f64>, _>("win_rate").unwrap_or(0.0),
        "pnl_moyen_r":          row.get::<Option<f64>, _>("pnl_moyen_r").unwrap_or(0.0),
        "derniere_maj":         chrono::Utc::now().timestamp(),
    }))
}

// ── Helper interne ────────────────────────────────────────────────────────────

fn mapper_row(r: &sqlx::sqlite::SqliteRow) -> StraddleFeedbackRow {
    StraddleFeedbackRow {
        id: r.get("id"),
        signal_id: r.get("signal_id"),
        pic_id: r.get("pic_id"),
        asset: r.get("asset"),
        timeframe: r.get("timeframe"),
        timestamp_signal: r.get("timestamp_signal"),
        categorie: r.get("categorie"),
        evenement_nom: r.get("evenement_nom"),
        session_active: r.get("session_active"),
        ratio_atr: r.get("ratio_atr"),
        score_llm: r.get("score_llm"),
        verdict: r.get("verdict"),
        amplitude_reelle_pct: r.get("amplitude_reelle_pct"),
        duree_trade_min: r.get("duree_trade_min"),
        pnl_r: r.get("pnl_r"),
        gagnant: r.get("gagnant"),
        cree_le: r.get("cree_le"),
        ferme_le: r.get("ferme_le"),
    }
}
