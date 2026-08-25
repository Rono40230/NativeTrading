//! Flux graphique des actifs MT5/Axi — protocole identique au flux Binance
//! (`candle`/`historical_start`/`historical_end`) : la bougie EN FORMATION
//! poussée par l'EA chaque seconde est servie telle quelle (vrai OHLC).
//! L'historique reste celui du REST (déjà chargé par le chart) — le batch
//! vide préserve les données du store.

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use std::time::Duration;

use crate::state::AppState;

pub async fn stream_mt5(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset = query.get("asset").cloned().unwrap_or_default().to_uppercase();
    let tf = query.get("timeframe").cloned().unwrap_or_else(|| "M15".into());
    let _ = &state;

    let (response, mut session, mut client) = actix_ws::handle(&req, body)?;
    actix_web::rt::spawn(async move {
        let _ = session.text(r#"{"type":"historical_start"}"#).await;
        let _ = session.text(r#"{"type":"historical_end"}"#).await;

        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if let Some((debut, o, h, l, c, v)) =
                        crate::mt5_collecteur::bougie_en_formation(&asset, &tf)
                    {
                        let msg = serde_json::json!({
                            "type": "candle",
                            "data": { "timestamp": debut, "open": o, "high": h,
                                      "low": l, "close": c, "volume": v }
                        });
                        if session.text(serde_json::to_string(&msg).unwrap_or_default())
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                msg = client.recv() => match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(Message::Ping(b))) => { let _ = session.pong(&b).await; }
                    _ => {}
                }
            }
        }
    });
    Ok(response)
}
