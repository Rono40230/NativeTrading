//! Streaming IB Gateway — métaux, paires Forex et indices
//! 1 connexion TCP, 2 subscriptions multiplexées :
//!   - historical_data_streaming → bougies chart
//!   - tick_by_tick_bid_ask      → prix live (Forex/métaux — None pour indices)

use actix_ws::Message;
use ibapi::market_data::historical::{Duration, HistoricalBarUpdate};
use ibapi::market_data::TradingHours;
use ibapi::Client;
use std::sync::atomic::{AtomicI32, Ordering};

use super::ib_contracts::{ib_bar_size, ib_contrat_hist, ib_contrat_tick, what_to_show_hist};
use super::types::{CandleData, CandleEvent};

/// Compteur global : chaque connexion IB reçoit un offset unique croissant,
/// ce qui évite les conflits [326] "client_id déjà utilisé" entre reconnexions.
static IB_OFFSET_COUNTER: AtomicI32 = AtomicI32::new(1);

// ─── Stream principal ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) async fn stream_ib(
    mut session: actix_ws::Session,
    mut client_stream: actix_ws::MessageStream,
    asset: Option<common::Asset>,
    timeframe: common::Timeframe,
    asset_str: String,
    timeframe_str: String,
    ib_port: u16,
    ib_client_id: i32,
) {
    let asset = match asset {
        Some(a) => a,
        None => {
            let _ = session.close(None).await;
            return;
        }
    };

    let adresse = format!("127.0.0.1:{}", ib_port);

    // Compteur atomique : chaque connexion obtient un offset unique croissant,
    // ce qui garantit qu'on ne réutilise pas un client_id encore enregistré
    // côté IB Gateway (timeout ~30s).
    let client = {
        let mut conn_result: Option<Client> = None;
        let mut last_err = String::new();
        for _ in 0..10 {
            let offset = IB_OFFSET_COUNTER.fetch_add(1, Ordering::Relaxed);
            let candidate_id = ib_client_id + offset;
            match Client::connect(&adresse, candidate_id).await {
                Ok(c) => {
                    tracing::info!("IB Gateway connecté avec client_id={}", candidate_id);
                    conn_result = Some(c);
                    break;
                }
                Err(e) => {
                    last_err = e.to_string();
                    if !last_err.contains("eof") && !last_err.contains("326") {
                        break;
                    }
                    tracing::debug!(
                        "client_id={} indisponible ({}), prochain",
                        candidate_id,
                        last_err
                    );
                }
            }
        }
        match conn_result {
            Some(c) => c,
            None => {
                let err = serde_json::json!({
                    "type": "error",
                    "message": format!("IB Gateway: {}", last_err)
                });
                if let Ok(p) = serde_json::to_string(&err) {
                    let _ = session.text(p).await;
                }
                let _ = session.close(None).await;
                return;
            }
        }
    };

    let contrat = ib_contrat_hist(&asset);
    let what_to_show = what_to_show_hist(&asset);

    // Subscription 1 : bougies historiques + updates (chart)
    let mut sub_hist = match client
        .historical_data_streaming(
            &contrat,
            Duration::days(2),
            ib_bar_size(&timeframe),
            Some(what_to_show),
            TradingHours::Extended,
            true,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let err =
                serde_json::json!({ "type": "error", "message": format!("IB stream: {}", e) });
            if let Ok(p) = serde_json::to_string(&err) {
                let _ = session.text(p).await;
            }
            let _ = session.close(None).await;
            return;
        }
    };

    // Subscription 2 : tick-by-tick bid/ask (Forex + métaux seulement)
    let mut sub_tick_opt = match ib_contrat_tick(&asset) {
        Some(contrat_tick) => match client.tick_by_tick_bid_ask(&contrat_tick, 0, true).await {
            Ok(s) => {
                tracing::info!("IB tick_by_tick_bid_ask actif pour {}", asset_str);
                Some(s)
            }
            Err(e) => {
                tracing::warn!("IB tick indisponible pour {} — erreur: {:?}", asset_str, e);
                None
            }
        },
        None => {
            tracing::debug!(
                "Tick-by-tick non disponible pour {} (indice/CFD)",
                asset_str
            );
            None
        }
    };

    let ok =
        serde_json::json!({ "type": "connected", "asset": asset_str, "timeframe": timeframe_str });
    if let Ok(p) = serde_json::to_string(&ok) {
        let _ = session.text(p).await;
    }

    loop {
        tokio::select! {
            // Bougies historiques → chart
            update = sub_hist.next() => {
                match update {
                    Some(HistoricalBarUpdate::Historical(hist)) => {
                        let start = serde_json::json!({ "type": "historical_start" });
                        if let Ok(p) = serde_json::to_string(&start) {
                            if session.text(p).await.is_err() { break; }
                        }
                        for bar in &hist.bars {
                            #[allow(deprecated)]
                            let ts = chrono::DateTime::from_timestamp(bar.date.unix_timestamp(), 0)
                                .unwrap_or_default().timestamp();
                            let event = CandleEvent {
                                r#type: "candle",
                                asset: asset_str.clone(),
                                timeframe: timeframe_str.clone(),
                                data: CandleData {
                                    timestamp: ts,
                                    open: bar.open,
                                    high: bar.high,
                                    low: bar.low,
                                    close: bar.close,
                                    volume: bar.volume,
                                },
                            };
                            if let Ok(p) = serde_json::to_string(&event) {
                                if session.text(p).await.is_err() { break; }
                            }
                        }
                        let end = serde_json::json!({ "type": "historical_end" });
                        if let Ok(p) = serde_json::to_string(&end) {
                            if session.text(p).await.is_err() { break; }
                        }
                    }
                    Some(HistoricalBarUpdate::Update(bar)) => {
                        #[allow(deprecated)]
                        let ts = chrono::DateTime::from_timestamp(bar.date.unix_timestamp(), 0)
                            .unwrap_or_default().timestamp();
                        let event = CandleEvent {
                            r#type: "candle",
                            asset: asset_str.clone(),
                            timeframe: timeframe_str.clone(),
                            data: CandleData {
                                timestamp: ts,
                                open: bar.open,
                                high: bar.high,
                                low: bar.low,
                                close: bar.close,
                                volume: bar.volume,
                            },
                        };
                        if let Ok(p) = serde_json::to_string(&event) {
                            if session.text(p).await.is_err() { break; }
                        }
                    }
                    Some(HistoricalBarUpdate::End { .. }) => {
                        let end = serde_json::json!({ "type": "historical_end" });
                        if let Ok(p) = serde_json::to_string(&end) {
                            if session.text(p).await.is_err() { break; }
                        }
                    }
                    None => break,
                }
            }
            // tick_by_tick_bid_ask → prix live (pending si non disponible)
            tick = async {
                match sub_tick_opt.as_mut() {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                match tick {
                    Some(Ok(ba)) => {
                        let mid = (ba.bid_price + ba.ask_price) / 2.0;
                        tracing::debug!(
                            "bid_ask tick — bid={:.4} ask={:.4} mid={:.4}",
                            ba.bid_price, ba.ask_price, mid
                        );
                        let event = serde_json::json!({
                            "type": "price",
                            "asset": asset_str,
                            "price": mid,
                            "timestamp": chrono::Utc::now().timestamp(),
                        });
                        if let Ok(p) = serde_json::to_string(&event) {
                            if session.text(p).await.is_err() { break; }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("IB tick Forex erreur: {:?} — désactivation", e);
                        sub_tick_opt = None;
                    }
                    None => {
                        tracing::warn!("IB tick Forex subscription terminée — désactivation");
                        sub_tick_opt = None;
                    }
                }
            }
            msg = client_stream.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        sub_hist.cancel().await;
                        if let Some(ref mut s) = sub_tick_opt { s.cancel().await; }
                        break;
                    }
                    Some(Err(_)) => {
                        sub_hist.cancel().await;
                        if let Some(ref mut s) = sub_tick_opt { s.cancel().await; }
                        break;
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        if session.pong(&bytes).await.is_err() {
                            sub_hist.cancel().await;
                            if let Some(ref mut s) = sub_tick_opt { s.cancel().await; }
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
