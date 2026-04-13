//! Endpoints API pour le monitoring ML de la stratégie Rockets.
//!
//! Routes :
//!   GET  /api/rockets/monitoring-ml      → stats globales + par phase + dérive
//!   GET  /api/rockets/calibration        → seuils calibrés par phase+session
//!   GET  /api/rockets/feedback           → historique des feedbacks filtrés
//!   POST /api/rockets/feedback/trader    → saisie résultat trader
//!   GET  /api/rockets/equity             → courbe equity simulée

use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct QueryFeedback {
    pub ticker: Option<String>,
    pub phase: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct QueryEquity {
    pub capital: Option<f64>,
    pub risk_pct: Option<f64>,
}

#[derive(Deserialize)]
pub struct BodyFeedbackTrader {
    pub signal_id: i64,
    /// "tp1" | "tp2" | "tp3" | "sl" | "ignore"
    pub verdict: String,
    pub prix_entree_reel: Option<f64>,
    pub prix_sortie_reel: Option<f64>,
    pub notes: Option<String>,
}


// ── GET /api/rockets/monitoring-ml ───────────────────────────────────────────

pub async fn monitoring_ml(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    let globales = match db::rockets_feedback_stats::stats_globales(pool).await {
        Ok(g) => g,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    // Stats par phase (conviction moyenne win vs lose + pnl R)
    let rows = sqlx::query(
        "SELECT phase,
                COUNT(*) AS nb_trades,
                SUM(gagnant) AS nb_gagnants,
                AVG(CASE WHEN gagnant = 1 THEN conviction_llm END) AS conv_win,
                AVG(CASE WHEN gagnant = 0 THEN conviction_llm END) AS conv_lose,
                AVG(pnl_r) AS pnl_r_moyen
         FROM rockets_feedback
         WHERE verdict IS NOT NULL
         GROUP BY phase
         ORDER BY nb_trades DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let par_phase: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            let wr = if nb > 0 { wins as f64 / nb as f64 } else { 0.0 };
            serde_json::json!({
                "phase":       r.get::<String, _>("phase"),
                "nb_trades":   nb,
                "win_rate":    wr,
                "conv_win":    r.get::<Option<f64>, _>("conv_win"),
                "conv_lose":   r.get::<Option<f64>, _>("conv_lose"),
                "pnl_r_moyen": r.get::<Option<f64>, _>("pnl_r_moyen"),
            })
        })
        .collect();

    // Détection de dérive : win rate des 20 derniers trades < 45 %
    let recents = sqlx::query(
        "SELECT gagnant FROM rockets_feedback
         WHERE verdict IS NOT NULL ORDER BY ferme_le DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let nb_rec = recents.len() as f64;
    let derive_detectee = if nb_rec >= 10.0 {
        let wins: f64 = recents
            .iter()
            .filter(|r| r.get::<Option<i64>, _>("gagnant").unwrap_or(0) == 1)
            .count() as f64;
        wins / nb_rec < 0.45
    } else {
        false
    };

    let mut reponse = globales;
    reponse["par_phase"] = serde_json::Value::Array(par_phase);
    reponse["derive_detectee"] = serde_json::Value::Bool(derive_detectee);

    HttpResponse::Ok().json(reponse)
}

// ── GET /api/rockets/calibration ─────────────────────────────────────────────

pub async fn get_calibration(state: web::Data<AppState>) -> impl Responder {
    match db::rockets_calibration::lister_toutes(state.db.pool()).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/rockets/feedback ─────────────────────────────────────────────────

pub async fn get_feedback(
    state: web::Data<AppState>,
    query: web::Query<QueryFeedback>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(500);
    let ticker = query.ticker.as_deref().unwrap_or("%");
    let phase = query.phase.as_deref().unwrap_or("%");
    let pool = state.db.pool();

    match db::rockets_feedback::lister_recents_ticker_phase_like(pool, ticker, phase, limit).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── POST /api/rockets/feedback/trader ────────────────────────────────────────

pub async fn post_feedback_trader(
    state: web::Data<AppState>,
    body: web::Json<BodyFeedbackTrader>,
) -> impl Responder {
    let pool = state.db.pool();

    let verdicts_valides = ["tp1", "tp2", "tp3", "sl", "ignore"];
    if !verdicts_valides.contains(&body.verdict.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "verdict invalide (tp1|tp2|tp3|sl|ignore)" }));
    }

    match db::rockets_feedback_trader::saisir_verdict_trader(
        pool,
        body.signal_id,
        &body.verdict,
        body.prix_entree_reel.unwrap_or(0.0),
        body.prix_sortie_reel,
        body.notes.as_deref(),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/rockets/equity ───────────────────────────────────────────────────

pub async fn get_equity(
    state: web::Data<AppState>,
    query: web::Query<QueryEquity>,
) -> impl Responder {
    let capital = query.capital.unwrap_or(10_000.0);
    let risk_pct = query.risk_pct.unwrap_or(0.015);
    let risk_montant = capital * risk_pct;
    let pool = state.db.pool();

    match db::rockets_feedback_stats::courbe_equity(pool, capital, risk_montant).await {
        Ok(points) => {
            let nb_trades = points.len() as i64;
            HttpResponse::Ok().json(serde_json::json!({
                "capital_initial": capital,
                "risk_pct": risk_pct,
                "nb_trades_saisis": nb_trades,
                "points": points,
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

