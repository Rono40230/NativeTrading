use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use smc::scorer;

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

#[derive(Deserialize)]
pub struct SmcQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

/// GET /api/smc/analyse?asset=BTC&timeframe=M15&limit=200
/// Retourne le score de confluence SMC (0–100) et les détails par composant.
pub async fn analyse_smc(
    query: web::Query<SmcQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": format!("Asset inconnu: {}", query.asset) }))
        }
    };
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200) as i64;

    let bougies = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit)
        .await
        .unwrap_or_default();

    if bougies.len() < 30 {
        return HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": "IB Gateway non connecté — données insuffisantes pour l'analyse SMC"
        }));
    }

    match scorer(&bougies) {
        Some(score) => HttpResponse::Ok().json(score),
        None => HttpResponse::Ok().json(serde_json::json!({
            "total": 0.0,
            "tendance": 0.0,
            "order_block": 0.0,
            "imbalance": 0.0,
            "ifvg": 0.0,
            "fibonacci": 0.0,
            "direction": "Both",
            "confluence": false,
            "message": "Marché indécis ou données insuffisantes"
        })),
    }
}
