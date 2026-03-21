//! Streaming WebSocket Binance — BTC / ETH temps réel

use actix_ws::Message;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

use super::types::{BinanceKlineMsg, CandleData, CandleEvent};

pub(super) fn binance_stream_url(asset: &common::Asset, tf: &common::Timeframe) -> String {
    let symbol = match asset {
        common::Asset::BTC  => "btcusdt",
        common::Asset::ETH  => "ethusdt",
        common::Asset::SOL  => "solusdt",
        common::Asset::BNB  => "bnbusdt",
        common::Asset::XRP  => "xrpusdt",
        common::Asset::ADA  => "adausdt",
        common::Asset::DOGE => "dogeusdt",
        common::Asset::AVAX => "avaxusdt",
        common::Asset::LINK => "linkusdt",
        common::Asset::DOT  => "dotusdt",
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
    format!(
        "wss://stream.binance.com:9443/ws/{}@kline_{}",
        symbol, interval
    )
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
    let ws_stream = match connect_async(&url).await {
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
                        if let Ok(km) = serde_json::from_str::<BinanceKlineMsg>(&txt) {
                            let k = &km.kline;
                            let event = CandleEvent {
                                r#type: "candle",
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
