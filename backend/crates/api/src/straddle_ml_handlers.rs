use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

// Re-exports des handlers volumineux extraits dans des modules dédiés
pub use crate::straddle_monitoring_handlers::monitoring_ml;

pub async fn get_calibration(state: web::Data<AppState>) -> impl Responder {
    match db::straddle_calibration::lister_toutes(state.db.pool()).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/straddle/seuils-effectifs ────────────────────────────────────────

