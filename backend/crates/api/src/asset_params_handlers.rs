use actix_web::{web, HttpResponse, Responder};
use db::asset_params::{lire_tous, sauvegarder_tous, AssetParams};

use crate::state::AppState;

pub async fn get_asset_params(state: web::Data<AppState>) -> impl Responder {
    match lire_tous(state.db.pool()).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

pub async fn put_asset_params(
    state: web::Data<AppState>,
    body: web::Json<Vec<AssetParams>>,
) -> impl Responder {
    match sauvegarder_tous(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}
