//! GET /api/prix?assets=XAUUSD,BTC,EURUSD
//! Retourne les prix actuels depuis Bybit/Binance (crypto + métaux),
//! avec fallback sur le dernier close en base pour les autres assets.
//!
//! GET /api/marche/klines   — proxy Bybit OHLCV (sparklines)
use actix_web::{web, HttpResponse, Responder};
use futures_util::future::join_all;
use std::collections::HashMap;

use crate::prix_utils;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct PrixQuery {
    pub assets: String,
}

pub async fn get_prix(state: web::Data<AppState>, query: web::Query<PrixQuery>) -> impl Responder {
    let client = &*crate::http_client::HTTP_CLIENT;

    let assets: Vec<String> = query
        .assets
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(50)
        .collect();

    let db = state.db.clone();
    let mut map: HashMap<String, f64> = HashMap::new();

    let futs: Vec<_> = assets
        .into_iter()
        .map(|asset| {
            let c = client.clone();
            async move {
                (asset.clone(), prix_utils::fetch_binance(&c, &asset).await)
            }
        })
        .collect();

    let resultats = join_all(futs).await;
    for (asset, prix) in resultats {
        if let Some(p) = prix {
            map.insert(asset.clone(), p);
        } else if let Some(prix_db) = prix_utils::dernier_prix_db(&asset, &db).await {
            map.insert(asset, prix_db);
        }
    }

    HttpResponse::Ok().json(map)
}


