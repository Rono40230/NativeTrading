//! WebSocket streaming temps réel — module racine
//! Dispatche vers Binance (crypto) ou IG Markets via Lightstreamer (métaux/forex/indices).

mod binance;
mod ig;
mod types;

use actix_web::{web, HttpRequest, HttpResponse};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

/// GET /api/stream?asset=BTC&timeframe=M1
pub async fn stream_market(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset_str = query
        .get("asset")
        .cloned()
        .unwrap_or_else(|| "BTC".to_string())
        .to_uppercase();
    let timeframe_str = query
        .get("timeframe")
        .cloned()
        .unwrap_or_else(|| "M1".to_string());

    let asset = parse_asset(&asset_str);
    let timeframe = parse_timeframe(&timeframe_str);
    let ls = state.ig_lightstreamer.clone();
    let ig_session = state.ig_session.clone();
    let db = state.db.clone();

    let (response, session, client_stream) = actix_ws::handle(&req, body)?;

    let crypto = asset.as_ref().map(|a| a.is_crypto()).unwrap_or(false);

    actix_web::rt::spawn(async move {
        if crypto {
            binance::stream_binance(
                session,
                client_stream,
                asset,
                timeframe,
                asset_str,
                timeframe_str,
            )
            .await;
        } else {
            ig::stream_ig(
                session,
                client_stream,
                asset,
                timeframe,
                asset_str,
                timeframe_str,
                ls,
                ig_session,
                db,
            )
            .await;
        }
    });

    Ok(response)
}
