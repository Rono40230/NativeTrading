//! Streaming IG Markets via polling REST /markets/{epic}.
//!
//! Fallback REST polling : toutes les 2s, `GET /markets/{epic}` est appelé pour
//! obtenir le prix spot (bid+offer)/2. Les ticks sont agrégés en bougies OHLC
//! alignées sur la période du timeframe et envoyées au client WebSocket.
//!
//! Charge initiale : bougies historiques depuis le cache DB ou REST IG.

use actix_ws::Message;
use common::{Asset, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use super::types::{CandleData, CandleEvent};
use crate::ig_lightstreamer::IgLightstreamer;
use crate::ig_session::IgSession;
use crate::prix_utils;

// ─── Durée en secondes d'un timeframe ────────────────────────────────────────

fn tf_secs(tf: Timeframe) -> i64 {
    match tf {
        Timeframe::M1 => 60,
        Timeframe::M5 => 300,
        Timeframe::M15 => 900,
        Timeframe::M30 => 1_800,
        Timeframe::H1 => 3_600,
        Timeframe::H4 => 14_400,
        Timeframe::D1 => 86_400,
        Timeframe::W1 => 604_800,
    }
}

// ─── Streaming principal ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) async fn stream_ig(
    mut session: actix_ws::Session,
    mut client_stream: actix_ws::MessageStream,
    asset: Option<Asset>,
    timeframe: Timeframe,
    asset_str: String,
    timeframe_str: String,
    ls: Arc<IgLightstreamer>,
    ig_session: Arc<Mutex<IgSession>>,
    db: Arc<Database>,
) {
    let asset = match asset {
        Some(a) => a,
        None => {
            let _ = session.close(None).await;
            return;
        }
    };

    // ── 1. Historique : cache DB frais ou seed REST IG ─────────────────────────
    let historique = ls.fetch_historique(&asset, timeframe, 200).await;

    let start = serde_json::json!({ "type": "historical_start" });
    if let Ok(p) = serde_json::to_string(&start) {
        if session.text(p).await.is_err() {
            return;
        }
    }

    for b in &historique {
        let evt = CandleEvent {
            r#type: "candle",
            asset: asset_str.clone(),
            timeframe: timeframe_str.clone(),
            data: CandleData {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            },
        };
        if let Ok(p) = serde_json::to_string(&evt) {
            if session.text(p).await.is_err() {
                return;
            }
        }
    }

    let hist_end = serde_json::json!({ "type": "historical_end" });
    if let Ok(p) = serde_json::to_string(&hist_end) {
        if session.text(p).await.is_err() {
            return;
        }
    }

    let connected = serde_json::json!({
        "type": "connected",
        "asset": asset_str,
        "timeframe": timeframe_str
    });
    if let Ok(p) = serde_json::to_string(&connected) {
        if session.text(p).await.is_err() {
            return;
        }
    }

    // ── 2. Polling REST /markets/{epic} toutes les 2s ─────────────────────────
    let http_client = match prix_utils::client_http() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("stream_ig: client HTTP: {}", e);
            let _ = session.close(None).await;
            return;
        }
    };

    let bar_dur = tf_secs(timeframe);
    let mut bar_ts: i64 = 0;
    let mut bar_open: f64 = 0.0;
    let mut bar_high: f64 = f64::NEG_INFINITY;
    let mut bar_low: f64 = f64::INFINITY;

    let mut interval = tokio::time::interval(Duration::from_secs(2));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let price = match prix_utils::fetch_prix_asset(
                    &http_client,
                    &asset_str,
                    &ig_session,
                    &db,
                ).await {
                    Some(p) => p,
                    None => continue,
                };

                let now_ts = chrono::Utc::now().timestamp();
                let new_bar_ts = (now_ts / bar_dur) * bar_dur;

                if new_bar_ts != bar_ts {
                    // Fermer la barre précédente si elle existe
                    if bar_ts > 0 {
                        let closed = CandleEvent {
                            r#type: "candle",
                            asset: asset_str.clone(),
                            timeframe: timeframe_str.clone(),
                            data: CandleData {
                                timestamp: bar_ts,
                                open:   bar_open,
                                high:   bar_high,
                                low:    bar_low,
                                close:  price,
                                volume: 0.0,
                            },
                        };
                        if let Ok(p) = serde_json::to_string(&closed) {
                            if session.text(p).await.is_err() {
                                return;
                            }
                        }
                    }
                    // Démarrer une nouvelle barre
                    bar_ts   = new_bar_ts;
                    bar_open = price;
                    bar_high = price;
                    bar_low  = price;
                } else {
                    bar_high = bar_high.max(price);
                    bar_low  = bar_low.min(price);
                }

                // Mise à jour temps réel de la barre en cours
                let update = CandleEvent {
                    r#type: "bar_update",
                    asset: asset_str.clone(),
                    timeframe: timeframe_str.clone(),
                    data: CandleData {
                        timestamp: bar_ts,
                        open:   bar_open,
                        high:   bar_high,
                        low:    bar_low,
                        close:  price,
                        volume: 0.0,
                    },
                };
                if let Ok(p) = serde_json::to_string(&update) {
                    if session.text(p).await.is_err() {
                        return;
                    }
                }
            }

            msg = client_stream.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(Message::Ping(bytes))) => {
                        let _ = session.pong(&bytes).await;
                    }
                    _ => {}
                }
            }
        }
    }
}
