//! Streaming IB Gateway — métaux, paires Forex et indices
//! 1 connexion TCP, 2 subscriptions multiplexées :
//!   - historical_data_streaming → bougies chart
//!   - tick_by_tick_bid_ask      → prix live (Forex/métaux — None pour indices)

use actix_ws::Message;
use ibapi::contracts::{Contract, SecurityType};
use ibapi::market_data::historical::{BarSize, Duration, HistoricalBarUpdate, WhatToShow};
use ibapi::market_data::TradingHours;
use ibapi::Client;

use super::types::{CandleData, CandleEvent};

// ─── Helpers contrats IB ──────────────────────────────────────────────────────

/// Contrat IB pour données historiques (chart).
/// Métaux → Commodity SMART | Forex → ForexPair IDEALPRO | Indices → CFD SMART
pub(super) fn ib_contrat_hist(asset: &common::Asset) -> Contract {
    match asset {
        // ── Métaux précieux ──────────────────────────────────────────────
        common::Asset::XAUUSD => Contract {
            symbol: "XAUUSD".into(),
            security_type: SecurityType::Commodity,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        common::Asset::XAGUSD => Contract {
            symbol: "XAGUSD".into(),
            security_type: SecurityType::Commodity,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        // ── Paires Forex (ForexPair IDEALPRO) ────────────────────────────
        common::Asset::EURUSD => ib_forex_pair("EUR", "USD"),
        common::Asset::GBPJPY => ib_forex_pair("GBP", "JPY"),
        common::Asset::CADJPY => ib_forex_pair("CAD", "JPY"),
        common::Asset::NZDJPY => ib_forex_pair("NZD", "JPY"),
        common::Asset::USDCAD => ib_forex_pair("USD", "CAD"),
        common::Asset::USDJPY => ib_forex_pair("USD", "JPY"),
        // Indices / CFD (CFD SMART) ────────────────────────────────────────────────
        // IB Index symbols : DAX → EUREX | SPX → CBOE | NAS100 → NQ contfut CME
        common::Asset::DAX => Contract {
            symbol: "DAX".into(),
            security_type: SecurityType::Index,
            exchange: "EUREX".into(),
            currency: "EUR".into(),
            ..Default::default()
        },
        common::Asset::NAS100 => Contract {
            symbol: "NQ".into(),
            security_type: SecurityType::ContinuousFuture,
            exchange: "CME".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        common::Asset::SP500 => Contract {
            symbol: "SPX".into(),
            security_type: SecurityType::Index,
            exchange: "CBOE".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        // BTC/ETH ne passent pas par IB — cas impossible en production
        common::Asset::BTC | common::Asset::ETH => Contract {
            symbol: asset.as_str().into(),
            security_type: SecurityType::CFD,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        },
    }
}

/// Contrat pour tick-by-tick bid/ask.
/// Métaux → ForexPair IDEALPRO (Commodity ne supporte pas tick-by-tick)
/// Forex  → même ForexPair IDEALPRO que pour l'historique
/// Indices → None (tick-by-tick non supporté sur CFD)
fn ib_contrat_tick(asset: &common::Asset) -> Option<Contract> {
    match asset {
        common::Asset::XAUUSD => Some(ib_forex_pair("XAU", "USD")),
        common::Asset::XAGUSD => Some(ib_forex_pair("XAG", "USD")),
        common::Asset::EURUSD => Some(ib_forex_pair("EUR", "USD")),
        common::Asset::GBPJPY => Some(ib_forex_pair("GBP", "JPY")),
        common::Asset::CADJPY => Some(ib_forex_pair("CAD", "JPY")),
        common::Asset::NZDJPY => Some(ib_forex_pair("NZD", "JPY")),
        common::Asset::USDCAD => Some(ib_forex_pair("USD", "CAD")),
        common::Asset::USDJPY => Some(ib_forex_pair("USD", "JPY")),
        // Indices et crypto : pas de tick-by-tick
        _ => None,
    }
}

/// Helper : construit un contrat ForexPair IDEALPRO
fn ib_forex_pair(symbole: &str, devise: &str) -> Contract {
    Contract {
        symbol: symbole.into(),
        security_type: SecurityType::ForexPair,
        exchange: "IDEALPRO".into(),
        currency: devise.into(),
        ..Default::default()
    }
}

/// WhatToShow selon le type d'asset.
/// Indices / Futures continus → Trades | Forex, métaux → MidPoint
fn what_to_show_hist(asset: &common::Asset) -> WhatToShow {
    match asset {
        common::Asset::DAX | common::Asset::NAS100 | common::Asset::SP500 => WhatToShow::Trades,
        _ => WhatToShow::MidPoint,
    }
}

fn ib_bar_size(tf: &common::Timeframe) -> BarSize {
    match tf {
        common::Timeframe::M1 => BarSize::Min,
        common::Timeframe::M5 => BarSize::Min5,
        common::Timeframe::M15 => BarSize::Min15,
        common::Timeframe::H1 => BarSize::Hour,
        common::Timeframe::H4 => BarSize::Hour4,
        common::Timeframe::D1 => BarSize::Day,
        common::Timeframe::W1 => BarSize::Week,
    }
}

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

    // Retry avec des client_id différents si [326] "déjà utilisé"
    let client = {
        let mut conn_result: Option<Client> = None;
        let mut last_err = String::new();
        for offset in 1i32..=20 {
            match Client::connect(&adresse, ib_client_id + offset).await {
                Ok(c) => {
                    tracing::info!("IB Gateway connecté avec client_id={}", ib_client_id + offset);
                    conn_result = Some(c);
                    break;
                }
                Err(e) => {
                    last_err = e.to_string();
                    if !last_err.contains("eof") && !last_err.contains("326") {
                        break;
                    }
                    tracing::debug!(
                        "client_id={} indisponible ({}), essai suivant",
                        ib_client_id + offset,
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
            tracing::debug!("Tick-by-tick non disponible pour {} (indice/CFD)", asset_str);
            None
        }
    };

    let ok = serde_json::json!({ "type": "connected", "asset": asset_str, "timeframe": timeframe_str });
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
