//! GET /api/prix?assets=XAUUSD,BTC,EURUSD
//! Retourne les prix actuels depuis Binance (crypto) ou IG Markets (autres).
//!
//! GET /api/marche/klines   — proxy Binance OHLCV (sparklines)
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
    let client = match prix_utils::client_http() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client HTTP /api/prix: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    let assets: Vec<String> = query
        .assets
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(50)
        .collect();

    let db = state.db.clone();
    let ig_session = state.ig_session.clone();
    let mut map: HashMap<String, f64> = HashMap::new();

    let mut ig_assets = Vec::new();
    let mut crypto_assets = Vec::new();
    for a in &assets {
        if let Some(epic) = prix_utils::ig_epic_str(a) {
            ig_assets.push((a.clone(), epic.to_string()));
        } else {
            crypto_assets.push(a.clone());
        }
    }

    if !ig_assets.is_empty() {
        let epics: Vec<&str> = ig_assets.iter().map(|(_, e)| e.as_str()).collect();
        let result_ig = prix_utils::fetch_ig_multi(&client, &ig_session, &db, &epics).await;
        
        for (asset, epic) in &ig_assets {
            if let Some(&p) = result_ig.get(epic) {
                map.insert(asset.clone(), p);
            } else if let Some(prix) = prix_utils::dernier_prix_db(asset, &db).await {
                map.insert(asset.clone(), prix);
            }
        }
    }

    let futs: Vec<_> = crypto_assets
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
            map.insert(asset, p);
        } else if let Some(prix_db) = prix_utils::dernier_prix_db(&asset, &db).await {
            map.insert(asset, prix_db);
        }
    }

    HttpResponse::Ok().json(map)
}

const INTERVALS_AUTORISES: &[&str] = &[
    "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w", "1M",
];

#[derive(serde::Deserialize)]
pub struct KlinesQuery {
    pub symbol: String,
    pub interval: String,
    pub limit: Option<u32>,
}

pub async fn get_klines_crypto(query: web::Query<KlinesQuery>) -> impl Responder {
    let symbol = query.symbol.trim().to_uppercase();
    if symbol.is_empty() || symbol.len() > 10 || !symbol.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "symbol invalide" }));
    }
    if !INTERVALS_AUTORISES.contains(&query.interval.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "interval invalide" }));
    }

    let limit = query.limit.unwrap_or(100).min(1000);
    let symbol_usdt = format!("{}USDT", symbol);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }))
        }
    };

    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
        symbol_usdt, query.interval, limit
    );

    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => {
            let data: Vec<Vec<serde_json::Value>> = match r.json().await {
                Ok(d) => d,
                Err(_) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": "Parse JSON" }))
                }
            };
            HttpResponse::Ok().json(data)
        }
        Ok(r) => HttpResponse::BadGateway()
            .json(serde_json::json!({ "error": format!("Binance {}", r.status()) })),
        Err(e) => HttpResponse::BadGateway().json(serde_json::json!({ "error": e.to_string() })),
    }
}
