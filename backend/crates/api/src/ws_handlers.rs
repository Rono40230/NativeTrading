use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMsg};

use crate::state::AppState;
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
/// Proxy WebSocket : Binance klines pour crypto, Finnhub trades pour métaux/forex.
pub async fn stream_market(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset_str = query.get("asset").map(|s| s.as_str()).unwrap_or("BTC");
    let timeframe_str = query.get("timeframe").map(|s| s.as_str()).unwrap_or("M1");

    let asset_label = asset_str.to_uppercase();
    let timeframe_label = timeframe_str.to_string();
    let _ = parse_timeframe(timeframe_str);

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    // Déterminer si c'est un métal/forex → Finnhub
    let asset_enum = crate::utils::parse_asset(&asset_label);
    let est_finnhub = asset_enum.as_ref().map(|a| a.vers_finnhub().is_some()).unwrap_or(false);

    if est_finnhub {
        // Récupérer la clé Finnhub
        let api_key = state.db.lire_config("finnhub_api_key").await
            .ok().flatten().filter(|k| !k.is_empty())
            .unwrap_or_else(|| std::env::var("FINNHUB_API_KEY").unwrap_or_default());

        if api_key.is_empty() {
            let _ = session.close(None).await;
            return Ok(response);
        }

        let symbole = asset_enum
            .and_then(|a| a.vers_finnhub().map(|s| s.to_string()))
            .unwrap_or_default();

        let finnhub_url = format!("wss://ws.finnhub.io?token={}", api_key);

        actix_web::rt::spawn(async move {
            let mut finnhub_ws = match connect_async(&finnhub_url).await {
                Ok((ws, _)) => ws,
                Err(e) => {
                    tracing::error!("Impossible de se connecter à Finnhub WS: {}", e);
                    let _ = session.close(None).await;
                    return;
                }
            };

            // Souscrire au symbole
            let sub = serde_json::json!({"type": "subscribe", "symbol": symbole}).to_string();
            let _ = finnhub_ws.send(WsMsg::Text(sub.into())).await;

            loop {
                tokio::select! {
                    msg = finnhub_ws.next() => {
                        let Some(msg) = msg else { break };
                        match msg {
                            Ok(WsMsg::Text(txt)) => {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&txt) {
                                    // Finnhub envoie trades: {"type":"trade","data":[{"p":price,"t":ts_ms,"s":symbol,"v":vol}]}
                                    if json.get("type").and_then(|v| v.as_str()) == Some("trade") {
                                        if let Some(trades) = json["data"].as_array() {
                                            if let Some(trade) = trades.last() {
                                                let price = trade["p"].as_f64().unwrap_or(0.0);
                                                let ts_ms = trade["t"].as_i64().unwrap_or(0);
                                                let timestamp = chrono::DateTime::from_timestamp_millis(ts_ms)
                                                    .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339())
                                                    .unwrap_or_default();
                                                let volume = trade["v"].as_f64().unwrap_or(0.0);
                                                let event = StreamEvent {
                                                    r#type: "candle",
                                                    asset: asset_label.clone(),
                                                    timeframe: timeframe_label.clone(),
                                                    data: serde_json::json!({
                                                        "timestamp": timestamp,
                                                        "open": price, "high": price,
                                                        "low": price, "close": price,
                                                        "volume": volume,
                                                    }),
                                                };
                                                if let Ok(payload) = serde_json::to_string(&event) {
                                                    if session.text(payload).await.is_err() { break; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => { tracing::warn!("Finnhub WS erreur: {}", e); break; }
                            _ => {}
                        }
                    }
                    Some(msg) = client_stream.recv() => {
                        match msg {
                            Ok(Message::Close(_)) | Err(_) => break,
                            Ok(Message::Ping(bytes)) => { if session.pong(&bytes).await.is_err() { break; } }
                            _ => {}
                        }
                    }
                }
            }
            // Désabonnement propre
            let unsub = serde_json::json!({"type": "unsubscribe", "symbol": symbole}).to_string();
            let _ = finnhub_ws.send(WsMsg::Text(unsub.into())).await;
            let _ = session.close(None).await;
        });
    } else {
        // Crypto → Binance klines (comportement inchangé)
        let symbol = format!("{}usdt", asset_str.to_lowercase());
        let interval = timeframe_vers_binance(timeframe_str);
        let binance_url = format!("wss://stream.binance.com:9443/ws/{}@kline_{}", symbol, interval);

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
                            Ok(Message::Ping(bytes)) => { if session.pong(&bytes).await.is_err() { break; } }
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

