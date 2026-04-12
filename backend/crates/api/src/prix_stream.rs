//! WS broadcast prix ticker — GET /api/prix/stream?assets=XAUUSD,BTC,EURUSD
//!
//! Une seule connexion WS pour tous les assets demandés.
//! Envoie `{ "XAUUSD": 4758.5, "BTC": 95000.0, … }` toutes les 2s.
//! Tous les assets sont fetchés en PARALLÈLE (join_all) comme le handler REST.
//! Utilise try_lock sur ig_session pour ne jamais bloquer le login Lightstreamer.

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::future::join_all;
use std::time::Duration;

use crate::prix_utils;
use crate::state::AppState;

/// GET /api/prix/stream?assets=XAUUSD,BTC,EURUSD
pub async fn stream_prix(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let assets_raw = query.get("assets").cloned().unwrap_or_default();

    // Validation : alphanumérique, max 50 assets, noms ≤ 10 chars
    let assets: Vec<String> = assets_raw
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(50)
        .collect();

    let ig_session = state.ig_session.clone();
    let db = state.db.clone();

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        let client = match prix_utils::client_http() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("prix_stream: client HTTP: {}", e);
                let _ = session.close(None).await;
                return;
            }
        };

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Tous les assets en PARALLÈLE — même pattern que le handler REST get_prix
                    let resultats = join_all(
                        assets.iter().map(|asset| {
                            let client = client.clone();
                            let ig = ig_session.clone();
                            let db = db.clone();
                            let asset = asset.clone();
                            async move {
                                let mut prix = prix_utils::fetch_prix_asset(&client, &asset, &ig, &db).await;
                                // Fallback DB pour les assets IG quand le mutex est bloqué par LS
                                if prix.is_none() && prix_utils::est_asset_ig(&asset) {
                                    prix = prix_utils::dernier_prix_db(&asset, &db).await;
                                }
                                (asset, prix)
                            }
                        })
                    ).await;

                    let mut map = serde_json::Map::new();
                    for (asset, prix) in resultats {
                        if let Some(p) = prix {
                            map.insert(asset, serde_json::json!(p));
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
