//! WebSocket streaming temps réel — module racine
//! Dispatche vers Binance (crypto + métaux). Les autres classes d'actifs
//! n'ont pas encore de provider de streaming.

mod binance;
mod types;

use actix_web::{web, HttpRequest, HttpResponse};

use crate::utils::{parse_asset, parse_timeframe};

/// GET /api/stream?asset=BTC&timeframe=M1
pub async fn stream_market(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
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
            // Pas de provider de streaming pour cet asset — fermeture propre.
            let _ = session.close(None).await;
        }
    });

    Ok(response)
}
