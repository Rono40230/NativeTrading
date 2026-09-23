//! Endpoints API pour le monitoring ML de la stratégie Rockets.
//!
//! Routes :
//!   GET  /api/rockets/monitoring-ml      → stats globales + par phase + dérive
//!   GET  /api/rockets/calibration        → seuils calibrés par phase+session
//!   GET  /api/rockets/feedback           → historique des feedbacks filtrés
//!   POST /api/rockets/feedback/trader    → saisie résultat trader
//!   GET  /api/rockets/equity             → courbe equity simulée

use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;



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


// ── POST /api/rockets/feedback/trader ────────────────────────────────────────


// ── GET /api/rockets/equity ───────────────────────────────────────────────────


// ── GET /api/rockets/seuils-effectifs ────────────────────────────────────────


