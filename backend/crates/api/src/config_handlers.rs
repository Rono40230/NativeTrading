use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state::AppState;

/// Clés de configuration autorisées (whitelist de sécurité)
const CLES_AUTORISEES: &[&str] = &[
    "capital_depart",
    "risque_trade",
    "anthropic_api_key",
    "telegram_bot_token",
    "telegram_chat_id",
    "seuil_confiance_rockets",
    "seuil_confiance_straddle",
    "seuil_confiance_smc",
    "tiingo_api_key",
];

#[derive(Deserialize)]
pub struct ConfigQuery {
    pub cle: String,
}

#[derive(Deserialize)]
pub struct ConfigUpdate {
    pub cle: String,
    pub valeur: String,
}

/// GET /api/config?cle=... — lit une valeur de configuration depuis SQLite
pub async fn get_config(
    state: web::Data<AppState>,
    query: web::Query<ConfigQuery>,
) -> impl Responder {
    if !CLES_AUTORISEES.contains(&query.cle.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Clé de configuration non autorisée" }));
    }
    match state.db.lire_config(&query.cle).await {
        Ok(Some(val)) => HttpResponse::Ok().json(serde_json::json!({
            "cle": query.cle,
            "valeur": val
        })),
        // Clé absente → 200 avec valeur null (pas encore configurée, valeur par défaut côté frontend)
        Ok(None) => {
            HttpResponse::Ok().json(serde_json::json!({ "cle": query.cle, "valeur": null }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

/// POST /api/config — enregistre une valeur de configuration dans SQLite
pub async fn post_config(
    state: web::Data<AppState>,
    body: web::Json<ConfigUpdate>,
) -> impl Responder {
    if !CLES_AUTORISEES.contains(&body.cle.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Clé de configuration non autorisée" }));
    }
    match state.db.ecrire_config(&body.cle, &body.valeur).await {
        Ok(()) => {
            tracing::info!("Config mise à jour: {}", body.cle);

            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}
