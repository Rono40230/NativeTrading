use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::Serialize;
use tokio_tungstenite::{connect_async, tungstenite::Message as BinanceMsg};

use crate::utils::parse_timeframe;

#[derive(Serialize)]
struct StreamEvent {
    r#type: &'static str,
    asset: String,
    timeframe: String,
    data: serde_json::Value,
}

/// Convertit un timeframe interne en intervalle Binance (ex: M1 → "1m")
fn timeframe_vers_binance(tf: &str) -> &'static str {
    match tf.to_uppercase().as_str() {
        "M1"  => "1m",
        "M5"  => "5m",
        "M15" => "15m",
        "H1"  => "1h",
        "H4"  => "4h",
        "D1"  => "1d",
        "W1"  => "1w",
        _     => "1m",
    }
}

/// GET /api/stream?asset=BTC&timeframe=M1
/// WebSocket proxy vers le stream kline Binance (push ~1-2s).
pub async fn stream_market(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset_str = query.get("asset").map(|s| s.as_str()).unwrap_or("BTC");
    let timeframe_str = query.get("timeframe").map(|s| s.as_str()).unwrap_or("M1");

    let asset_label = asset_str.to_uppercase();
    let timeframe_label = timeframe_str.to_string();
    let _ = parse_timeframe(timeframe_str); // validation

    let symbol = format!("{}usdt", asset_str.to_lowercase());
    let interval = timeframe_vers_binance(timeframe_str);
    let binance_url = format!("wss://stream.binance.com:9443/ws/{}@kline_{}", symbol, interval);

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        let mut binance_ws = match connect_async(&binance_url).await {
            Ok((ws, _)) => ws,
            Err(e) => {
                tracing::error!("Impossible de se connecter à Binance WS: {}", e);
                let _ = session.close(None).await;
                return;
            }
        };

        loop {
            tokio::select! {
                // Messages entrants depuis Binance
                msg = binance_ws.next() => {
                    let Some(msg) = msg else { break };
                    match msg {
                        Ok(BinanceMsg::Text(txt)) => {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&txt) {
                                if let Some(k) = json.get("k") {
                                    let ts_ms = k["t"].as_i64().unwrap_or(0);
                                    let timestamp = chrono::DateTime::from_timestamp_millis(ts_ms)
                                        .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339())
                                        .unwrap_or_default();
                                    let event = StreamEvent {
                                        r#type: "candle",
                                        asset: asset_label.clone(),
                                        timeframe: timeframe_label.clone(),
                                        data: serde_json::json!({
                                            "timestamp": timestamp,
                                            "open":   k["o"].as_str().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                                            "high":   k["h"].as_str().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                                            "low":    k["l"].as_str().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                                            "close":  k["c"].as_str().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                                            "volume": k["v"].as_str().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                                        }),
                                    };
                                    if let Ok(payload) = serde_json::to_string(&event) {
                                        if session.text(payload).await.is_err() {
                                            break; // client déconnecté
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Binance WS erreur: {}", e);
                            break;
                        }
                        _ => {} // Ping/Pong/Binary gérés automatiquement par tungstenite
                    }
                }
                // Messages entrants depuis le client frontend
                Some(msg) = client_stream.recv() => {
                    match msg {
                        Ok(Message::Close(_)) | Err(_) => break,
                        Ok(Message::Ping(bytes)) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let _ = session.close(None).await;
    });

    Ok(response)
}
