//! Endpoints API pour le monitoring ML de la stratégie SMC Directionnel.
//!
//! Routes :
//!   GET  /api/smc/monitoring-ml  → stats globales + par catégorie + dérive
//!   GET  /api/smc/calibration    → seuils calibrés par asset/timeframe/catégorie
//!   GET  /api/smc/feedback       → historique des feedbacks SMC filtrés
//!   GET  /api/smc/baremes        → constantes SCORE_MAX_* du moteur SMC

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;


// ── GET /api/smc/monitoring-ml ────────────────────────────────────────────────

pub async fn monitoring_ml(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    let globales = match db::smc_feedback::stats_globales(pool).await {
        Ok(g) => g,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    // Stats par catégorie SMC
    let rows = sqlx::query(
        "SELECT categorie,
                COUNT(*) AS nb_trades,
                SUM(gagnant) AS nb_gagnants,
                AVG(CASE WHEN gagnant = 1 THEN conviction_llm END) AS conv_win,
                AVG(CASE WHEN gagnant = 0 THEN conviction_llm END) AS conv_lose,
                AVG(pnl_r) AS pnl_r_moyen
         FROM smc_feedback
         WHERE verdict IS NOT NULL
         GROUP BY categorie
         ORDER BY nb_trades DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let par_categorie: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            let wr = if nb > 0 { wins as f64 / nb as f64 } else { 0.0 };
            serde_json::json!({
                "categorie":   r.get::<String, _>("categorie"),
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
        "SELECT gagnant FROM smc_feedback
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
    reponse["par_categorie"] = serde_json::Value::Array(par_categorie);
    reponse["derive_detectee"] = serde_json::Value::Bool(derive_detectee);

    HttpResponse::Ok().json(reponse)
}

// ── GET /api/smc/calibration ──────────────────────────────────────────────────

pub async fn get_calibration(state: web::Data<AppState>) -> impl Responder {
    match db::smc_calibration::lister_toutes(state.db.pool()).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/smc/feedback ─────────────────────────────────────────────────────


// ── GET /api/smc/equity ─────────────────────────────────────────────────────


// ── GET /api/smc/baremes ──────────────────────────────────────────────────────

