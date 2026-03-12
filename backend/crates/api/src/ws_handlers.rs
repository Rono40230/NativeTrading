use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use common::{Asset, Timeframe};
use data::{providers::yahoo::YahooFinanceProvider, DataProvider};
use futures_util::StreamExt;
use serde::Serialize;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMsg};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

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
/// Proxy WebSocket : Binance klines pour crypto, polling Yahoo Finance pour métaux.
pub async fn stream_market(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    _state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset_str = query.get("asset").map(|s| s.as_str()).unwrap_or("BTC");
    let timeframe_str = query.get("timeframe").map(|s| s.as_str()).unwrap_or("M1");

    let asset_label = asset_str.to_uppercase();
    let timeframe_label = timeframe_str.to_string();

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    let asset_enum = parse_asset(&asset_label);
    let est_metal = asset_enum
        .as_ref()
        .map(|a| YahooFinanceProvider::vers_symbole(a).is_some())
        .unwrap_or(false);

    if est_metal {
        // Métaux → polling Yahoo Finance toutes les 10 secondes
        let asset_for_poll = asset_enum.unwrap_or(Asset::XAUUSD);
        let tf_for_poll = parse_timeframe(&timeframe_label);

        actix_web::rt::spawn(async move {
            let provider = YahooFinanceProvider::new();
            let mut ticker = interval(Duration::from_secs(10));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        match provider.fetch_candles(asset_for_poll.clone(), tf_for_poll, 2).await {
                            Ok(bougies) if !bougies.is_empty() => {
                                let b = bougies.last().unwrap();
                                let event = StreamEvent {
                                    r#type: "candle",
                                    asset: asset_label.clone(),
                                    timeframe: timeframe_label.clone(),
                                    data: serde_json::json!({
                                        "timestamp": b.timestamp.to_rfc3339(),
                                        "open":   b.open,
                                        "high":   b.high,
                                        "low":    b.low,
                                        "close":  b.close,
                                        "volume": b.volume,
                                    }),
                                };
                                if let Ok(payload) = serde_json::to_string(&event) {
                                    if session.text(payload).await.is_err() { break; }
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!("Yahoo Finance poll: {}", e);
                            }
                        }
                    }
                    Some(msg) = client_stream.recv() => {
                        match msg {
                            Ok(Message::Close(_)) | Err(_) => break,
                            Ok(Message::Ping(bytes)) => {
                                if session.pong(&bytes).await.is_err() { break; }
                            }
                            _ => {}
                        }
                    }
                }
            }
            let _ = session.close(None).await;
        });
    } else {
        // Crypto → Binance klines WebSocket
        let symbol = format!("{}usdt", asset_str.to_lowercase());
        let interval_binance = timeframe_vers_binance(timeframe_str);
        let binance_url = format!(
            "wss://stream.binance.com:9443/ws/{}@kline_{}",
            symbol, interval_binance
        );

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
                    msg = binance_ws.next() => {
                        let Some(msg) = msg else { break };
                        match msg {
                            Ok(WsMsg::Text(txt)) => {
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
                                            if session.text(payload).await.is_err() { break; }
                                        }
                                    }
                                }
                            }
                            Err(e) => { tracing::warn!("Binance WS erreur: {}", e); break; }
                            _ => {}
                        }
                    }
                    Some(msg) = client_stream.recv() => {
                        match msg {
                            Ok(Message::Close(_)) | Err(_) => break,
                            Ok(Message::Ping(bytes)) => {
                                if session.pong(&bytes).await.is_err() { break; }
                            }
                            _ => {}
                        }
                    }
                }
            }
            let _ = session.close(None).await;
        });
    }

    Ok(response)
}

