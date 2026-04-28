//! Streaming WebSocket Binance — BTC / ETH temps réel

use actix_ws::Message;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

use super::types::{BinanceKlineMsg, CandleData, CandleEvent};

pub(super) fn binance_stream_url(asset: &common::Asset, tf: &common::Timeframe) -> String {
    let symbol = match asset {
        common::Asset::BTC => "btcusdt",
        common::Asset::ETH => "ethusdt",
        common::Asset::SOL => "solusdt",
        common::Asset::BNB => "bnbusdt",
        common::Asset::XRP => "xrpusdt",
        common::Asset::ADA => "adausdt",
        common::Asset::DOGE => "dogeusdt",
        common::Asset::AVAX => "avaxusdt",
        common::Asset::LINK => "linkusdt",
        common::Asset::DOT => "dotusdt",
        _ => "btcusdt",
    };
    let interval = match tf {
        common::Timeframe::M1 => "1m",
        common::Timeframe::M5 => "5m",
        common::Timeframe::M15 => "15m",
        common::Timeframe::M30 => "30m",
        common::Timeframe::H1 => "1h",
        common::Timeframe::H4 => "4h",
        common::Timeframe::D1 => "1d",
        common::Timeframe::W1 => "1w",
    };

    if matches!(asset, common::Asset::XAUUSD | common::Asset::XAGUSD) {
        "wss://stream.bybit.com/v5/public/linear".to_string()
    } else {
        format!("wss://stream.binance.com:9443/ws/{}@kline_{}", symbol, interval)
    }
}

pub(super) async fn stream_binance(
    mut session: actix_ws::Session,
    mut client_stream: actix_ws::MessageStream,
    asset: Option<common::Asset>,
    timeframe: common::Timeframe,
    asset_str: String,
    timeframe_str: String,
) {
    let asset = match asset {
        Some(a) => a,
        None => {
            let _ = session.close(None).await;
            return;
        }
    };

    let url = binance_stream_url(&asset, &timeframe);
    let mut ws_stream = match connect_async(&url).await {
        Ok((s, _)) => s,
        Err(e) => {
            let err =
                serde_json::json!({ "type": "error", "message": format!("Binance WS: {}", e) });
            if let Ok(p) = serde_json::to_string(&err) {
                let _ = session.text(p).await;
            }
            let _ = session.close(None).await;
            return;
        }
    };

    let is_bybit = matches!(asset, common::Asset::XAUUSD | common::Asset::XAGUSD);
    if is_bybit {
        let bybit_interval = match timeframe {
            common::Timeframe::M1 => "1",
            common::Timeframe::M5 => "5",
            common::Timeframe::M15 => "15",
            common::Timeframe::M30 => "30",
            common::Timeframe::H1 => "60",
            common::Timeframe::H4 => "240",
            common::Timeframe::D1 => "D",
            common::Timeframe::W1 => "W",
        };
        use futures_util::SinkExt;
        let sym = if asset == common::Asset::XAUUSD { "XAUUSDT" } else { "XAGUSDT" };
        let sub_msg = serde_json::to_string(&serde_json::json!({
            "op": "subscribe",
            "args": [format!("kline.{}.{}", bybit_interval, sym)]
        })).unwrap_or_default();
        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(sub_msg)).await;
    }

    let (_, mut binance_rx) = ws_stream.split();
    let ok =
        serde_json::json!({ "type": "connected", "asset": asset_str, "timeframe": timeframe_str });
    if let Ok(p) = serde_json::to_string(&ok) {
        let _ = session.text(p).await;
    }

    loop {
        tokio::select! {
            msg = binance_rx.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(txt))) => {
                        if is_bybit {
                            tracing::debug!("Bybit WS RX: {}", txt);
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                                // Gérer les messages kline
                                if v.get("topic").and_then(|t| t.as_str()).map_or(false, |t| t.starts_with("kline.")) {
                                    if let Some(data_arr) = v.get("data").and_then(|d| d.as_array()) {
                                        if let Some(k) = data_arr.first() {
                                            tracing::debug!("Bybit kline data block: {:?}", k);
                                            let start = k.get("start").and_then(|s| s.as_u64());
                                            let open = k.get("open").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok());
                                            let high = k.get("high").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok());
                                            let low = k.get("low").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok());
                                            let close = k.get("close").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok());
                                            let volume = k.get("volume").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok());
                                            let confirm = k.get("confirm").and_then(|c| c.as_bool());
                                            
                                            tracing::debug!("Parsed fields: s={:?} o={:?} h={:?} l={:?} c={:?} v={:?} conf={:?}", start, open, high, low, close, volume, confirm);
                                            
                                            if let (
                                                Some(start),
                                                Some(open),
                                                Some(high),
                                                Some(low),
                                                Some(close),
                                                Some(volume),
                                                Some(confirm),
                                            ) = (start, open, high, low, close, volume, confirm) {
                                                let event = CandleEvent {
                                                    r#type: if confirm { "candle" } else { "bar_update" },
                                                    asset: asset_str.clone(),
                                                    timeframe: timeframe_str.clone(),
                                                    data: CandleData {
                                                        timestamp: (start / 1000) as i64,
                                                        open,
                                                        high,
                                                        low,
                                                        close,
                                                        volume,
                                                    },
                                                };
                                                if let Ok(p) = serde_json::to_string(&event) {
                                                    tracing::debug!("Sending to client: {}", p);
                                                    if session.text(p).await.is_err() { break; }
                                                }
                                            } else {
                                                tracing::warn!("Failed to parse all bybit kline fields from {:?}", k);
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            if let Ok(km) = serde_json::from_str::<BinanceKlineMsg>(&txt) {
                                let k = &km.kline;
                                let event = CandleEvent {
                                    r#type: if k.is_closed { "candle" } else { "bar_update" },
                                    asset: asset_str.clone(),
                                    timeframe: timeframe_str.clone(),
                                    data: CandleData {
                                        timestamp: (k.open_time_ms / 1000) as i64,
                                        open: k.open.parse().unwrap_or(0.0),
                                        high: k.high.parse().unwrap_or(0.0),
                                        low: k.low.parse().unwrap_or(0.0),
                                        close: k.close.parse().unwrap_or(0.0),
                                        volume: k.volume.parse().unwrap_or(0.0),
                                    },
                                };
                                if let Ok(p) = serde_json::to_string(&event) {
                                    if session.text(p).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(_)) | None => break,
                    _ => {}
                }
            }
            msg = client_stream.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    Some(Ok(Message::Ping(bytes))) => {
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
}
