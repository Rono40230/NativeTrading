//! WS broadcast prix ticker — GET /api/prix/stream?assets=XAUUSD,BTC,EURUSD
//!
//! Une seule connexion WS pour tous les assets demandés.
//! Rafraîchissement sûr : 2 secondes (REST groupé anti-ban IG).

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

    let client = match prix_utils::client_http() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("prix_stream: client HTTP: {}", e);
            return Err(actix_web::error::ErrorInternalServerError(e));
        }
    };

    let ig_session = state.ig_session.clone();
    let db = state.db.clone();

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        // Rafraichissement : Intervalle de 1s pour baisser la latence sur crypto/métaux
        // On gardera un compteur pour ne requêter IG que toutes les 2s (anti-spammeur IG)
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Map Asset > Epic pour optimiser les appels groupés IG
        let mut ig_assets = Vec::new();
        let mut crypto_assets = Vec::new();
        for a in &assets {
            if let Some(epic) = prix_utils::ig_epic_str(a) {
                ig_assets.push((a.clone(), epic.to_string()));
            } else {
                crypto_assets.push(a.clone());
            }
        }

        let mut tick_counter: u64 = 0;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    tick_counter = tick_counter.wrapping_add(1);
                    let mut map = serde_json::Map::new();
                    
                    // 1. IG Multi-markets Request (Tout d'un coup, 1 requête = SAFE)
                    // Seulement 1 fois sur 2 (toutes les 2 secondes)
                    if !ig_assets.is_empty() && (tick_counter.is_multiple_of(2)) {
                        let epics: Vec<&str> = ig_assets.iter().map(|(_, e)| e.as_str()).collect();
                        let result_ig = prix_utils::fetch_ig_multi(&client, &ig_session, &db, &epics).await;
                        
                        for (asset, epic) in &ig_assets {
                            if let Some(&p) = result_ig.get(epic) {
                                map.insert(asset.clone(), serde_json::json!(p));
                            } else if let Some(prix) = prix_utils::dernier_prix_db(asset, &db).await {
                                // Fallback DB (week-end ou login en cours)
                                map.insert(asset.clone(), serde_json::json!(prix));
                            }
                        }
                    }

                    // 2. Crypto Requests (Binance n'a pas de rate limit aussi strict pour ces requêtes, join_all sûr)
                    let resultats_crypto = join_all(
                        crypto_assets.iter().map(|asset| {
                            let client = client.clone();
                            let asset = asset.clone();
                            async move {
                                let prix = prix_utils::fetch_binance(&client, &asset).await;
                                (asset, prix)
                            }
                        })
                    ).await;

                    for (asset, prix) in resultats_crypto {
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
