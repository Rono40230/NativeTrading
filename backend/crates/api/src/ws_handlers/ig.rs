//! Streaming IG Markets via Lightstreamer.
//!
//! Au lieu de poller l'API REST toutes les 5s,
//! on s'abonne au canal broadcast IgLightstreamer et on relaie les bougies
//! vers le client WebSocket frontend.
//!
//! Charge initiale : bougies historiques depuis le cache DB.
//! Puis : mises à jour live via broadcast Lightstreamer.

use actix_ws::Message;
use common::{Asset, Timeframe};
use std::sync::Arc;

use crate::ig_lightstreamer::{IgLightstreamer, LsCandle};
use super::types::{CandleData, CandleEvent};

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
    db: Arc<db::Database>,
) {
    let asset = match asset {
        Some(a) => a,
        None => {
            let _ = session.close(None).await;
            return;
        }
    };

    // ── 1. Charge historique depuis le cache DB ────────────────────────────────
    let historique = db
        .obtenir_bougies(&asset, &timeframe, 200)
        .await
        .unwrap_or_default();

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
                timestamp: b.timestamp.timestamp_millis(),
                open:   b.open,
                high:   b.high,
                low:    b.low,
                close:  b.close,
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

    // ── 2. S'abonner au Lightstreamer pour cet asset+timeframe ────────────────
    if let Err(e) = ls.subscribe(&asset, timeframe).await {
        let err = serde_json::json!({ "type": "error", "message": format!("LS subscribe: {}", e) });
        if let Ok(p) = serde_json::to_string(&err) {
            let _ = session.text(p).await;
        }
        let _ = session.close(None).await;
        return;
    }

    let ok = serde_json::json!({
        "type": "connected",
        "asset": asset_str,
        "timeframe": timeframe_str
    });
    if let Ok(p) = serde_json::to_string(&ok) {
        if session.text(p).await.is_err() {
            ls.unsubscribe(&asset, timeframe).await.ok();
            return;
        }
    }

    // ── 3. Recevoir les bougies Lightstreamer et les relayer ──────────────────
    let mut rx = ls.subscribe_broadcast();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(ls_candle) => {
                        if !is_same_sub(&ls_candle, &asset, timeframe) {
                            continue;
                        }
                        let b = &ls_candle.candle;
                        let evt_type = if ls_candle.closed { "candle" } else { "bar_update" };
                        let evt = CandleEvent {
                            r#type: evt_type,
                            asset: asset_str.clone(),
                            timeframe: timeframe_str.clone(),
                            data: CandleData {
                                timestamp: b.timestamp.timestamp_millis(),
                                open:   b.open,
                                high:   b.high,
                                low:    b.low,
                                close:  b.close,
                                volume: b.volume,
                            },
                        };
                        if let Ok(p) = serde_json::to_string(&evt) {
                            if session.text(p).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        // Canal broadcast en retard : reprendre
                        rx = ls.subscribe_broadcast();
                    }
                }
            }

            msg = client_stream.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(bytes))) => {
                        let _ = session.pong(&bytes).await;
                    }
                    _ => {}
                }
            }
        }
    }

    ls.unsubscribe(&asset, timeframe).await.ok();
}

fn is_same_sub(ls_candle: &LsCandle, asset: &Asset, timeframe: Timeframe) -> bool {
    ls_candle.asset == *asset && ls_candle.timeframe == timeframe
}
