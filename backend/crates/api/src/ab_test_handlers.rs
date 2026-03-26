use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

/// GET /api/ia/ab-test
/// Retourne les stats de performance par variante de stratégie (SMC Directionnel vs SMC+IA, etc.)
/// pour comparer l'impact de l'enrichissement LLM sur le win rate.
pub async fn get_ab_test(state: web::Data<AppState>) -> impl Responder {
    match state.db.stats_ab_test().await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            tracing::error!("Stats A/B test: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
