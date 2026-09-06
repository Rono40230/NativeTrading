//! Config Rockets (carte Paramètres › Rockets). Les ex-handlers v1
//! (historique/actifs/suppression sur `rockets_signaux`, table vide) ont été
//! supprimés le 05/09 (§10) : l'historique vit dans `signaux` via la table
//! partagée, le suivi dans la verticale.

use actix_web::{web, HttpResponse, Responder};
use db::rockets;

use crate::state::AppState;

/// GET /api/rockets/config
pub async fn get_config(state: web::Data<AppState>) -> impl Responder {
    let cfg = rockets::lire_config(state.db.pool()).await;
    HttpResponse::Ok().json(cfg)
}

/// PUT /api/rockets/config
pub async fn put_config(
    state: web::Data<AppState>,
    body: web::Json<rockets::RocketsConfig>,
) -> impl Responder {
    match rockets::sauvegarder_config(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
