use crate::{state::AppState, utils};
use actix_web::{web, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueryPatterns {
    pub asset: Option<String>,
    pub timeframe: Option<String>,
}

/// GET /api/volatility/patterns?asset=BTC&timeframe=M15
/// Retourne les patterns de volatilité ATR agrégés par heure/jour,
/// classifiés en 4 clusters + le seuil Straddle calibré (P85).
pub async fn get_patterns(
    state: web::Data<AppState>,
    query: web::Query<QueryPatterns>,
) -> impl actix_web::Responder {
    let asset = match utils::parse_asset(query.asset.as_deref().unwrap_or("BTC")) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Asset inconnu" }))
        }
    };
    let timeframe = utils::parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));

    match state.db.obtenir_patterns_horaires(&asset, &timeframe).await {
        Ok(rep) => HttpResponse::Ok().json(rep),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
