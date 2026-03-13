//! WebSocket streaming temps réel — module racine
//! Dispatche vers Binance (crypto) ou IB Gateway (métaux).

mod binance;
mod ib;
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
    let ib_port = state.ib_port;
    let ib_client_id = state.ib_client_id;

    let (response, session, client_stream) = actix_ws::handle(&req, body)?;

    let crypto = matches!(&asset, Some(common::Asset::BTC) | Some(common::Asset::ETH));

    actix_web::rt::spawn(async move {
        if crypto {
            binance::stream_binance(session, client_stream, asset, timeframe, asset_str, timeframe_str).await;
        } else {
            ib::stream_ib(session, client_stream, asset, timeframe, asset_str, timeframe_str, ib_port, ib_client_id).await;
        }
    });

    Ok(response)
}
