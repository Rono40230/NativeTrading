//! WS broadcast prix ticker — GET /api/prix/stream?assets=XAUUSD,BTC,EURUSD
//!
//! Une seule connexion WS pour tous les assets demandés.
//! Rafraîchissement sûr : 2 secondes (REST groupé).

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::future::join_all;
use std::time::Duration;

use crate::prix_utils;
use crate::state::AppState;

pub async fn stream_prix(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let assets_raw = query.get("assets").cloned().unwrap_or_default();

    let assets: Vec<String> = assets_raw
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(50)
        .collect();

    let client = &*crate::http_client::HTTP_CLIENT;

    let db = state.db.clone();

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        // Rafraichissement : Intervalle de 2s
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let mut map = serde_json::Map::new();

                    // Prix par REST groupé (Bybit/Binance, fallback DB)
                    let resultats = join_all(
                        assets.iter().map(|asset| {
                            let client = client.clone();
                            let asset = asset.clone();
                            async move {
                                // MT5/Axi d'abord (bougie en formation, à la
                                // seconde) ; Binance pour la crypto ; DB en
                                // dernier recours (marché fermé).
                                let prix = crate::mt5_collecteur::prix_live(&asset)
                                    .or_else(|| None::<f64>);
                                let prix = match prix {
                                    Some(p) => Some(p),
                                    None => prix_utils::fetch_binance(&client, &asset).await,
                                };
                                (asset, prix)
                            }
                        })
                    ).await;

                    for (asset, prix) in resultats {
                        if let Some(p) = prix {
                            map.insert(asset.clone(), serde_json::json!(p));
                        } else if let Some(prix_db) = prix_utils::dernier_prix_db(&asset, &db).await {
                            // Fallback DB (week-end ou provider indisponible)
                            map.insert(asset, serde_json::json!(prix_db));
                        }
                    }

                    if !map.is_empty() {
                        let msg = serde_json::Value::Object(map);
                        if let Ok(txt) = serde_json::to_string(&msg) {
                            if session.text(txt).await.is_err() {
                                return;
                            }
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
    });

    Ok(response)
}
