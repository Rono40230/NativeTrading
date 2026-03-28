use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct MarquerLuPayload {
    pub url: String,
}

#[derive(Serialize)]
struct LusResponse {
    urls: Vec<String>,
}

/// POST /api/news/lu — marque un article comme lu dans la DB.
pub async fn marquer_lu(
    state: web::Data<AppState>,
    body: web::Json<MarquerLuPayload>,
) -> impl Responder {
    match state.db.marquer_article_lu(&body.url).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            tracing::warn!("marquer_lu DB error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "erreur": e.to_string() }))
        }
    }
}

/// GET /api/news/lus — retourne la liste des URLs lues.
pub async fn lire_lus(state: web::Data<AppState>) -> impl Responder {
    match state.db.lire_articles_lus().await {
        Ok(urls) => HttpResponse::Ok().json(LusResponse { urls }),
        Err(e) => {
            tracing::warn!("lire_lus DB error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "erreur": e.to_string() }))
        }
    }
}
