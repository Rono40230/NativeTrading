use actix_web::{web, HttpResponse, Responder};
use db::strategies_params::{
    lire_smc_params, lire_straddle_params, sauvegarder_smc_params, sauvegarder_straddle_params,
    SmcParams, StraddleParams,
};

use crate::state::AppState;

// ── Straddle ─────────────────────────────────────────────────────────────────

pub async fn get_straddle_params(state: web::Data<AppState>) -> impl Responder {
    let params = lire_straddle_params(state.db.pool()).await;
    HttpResponse::Ok().json(params)
}

pub async fn put_straddle_params(
    state: web::Data<AppState>,
    body: web::Json<StraddleParams>,
) -> impl Responder {
    match sauvegarder_straddle_params(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ── SMC ──────────────────────────────────────────────────────────────────────

pub async fn get_smc_params(state: web::Data<AppState>) -> impl Responder {
    let params = lire_smc_params(state.db.pool()).await;
    HttpResponse::Ok().json(params)
}

pub async fn put_smc_params(
    state: web::Data<AppState>,
    body: web::Json<SmcParams>,
) -> impl Responder {
    match sauvegarder_smc_params(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}
