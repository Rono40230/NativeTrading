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
    let symbol_usdt = if symbol == "XAUUSD" || symbol == "XAUUSDT" {
        "XAUUSDT".to_string()
    } else if symbol == "XAGUSD" || symbol == "XAGUSDT" {
        "XAGUSDT".to_string()
    } else {
        format!("{}USDT", symbol)
    };

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

    let category = if symbol_usdt == "XAUUSDT" || symbol_usdt == "XAGUSDT" {
        "linear"
    } else {
        "spot"
    };

    // Mapping d'intervalle générique (Bybit requiert '1', '5', '15', '60', 'D')
    let interval = match query.interval.as_str() {
        "1m" => "1", "3m" => "3", "5m" => "5", "15m" => "15", "30m" => "30",
        "1h" => "60", "2h" => "120", "4h" => "240", "6h" => "360", "12h" => "720",
        "1d" => "D", "1w" => "W", "1M" => "M",
        other => other,
    };

    let url = format!(
        "https://api.bybit.com/v5/market/kline?category={}&symbol={}&interval={}&limit={}",
        category, symbol_usdt, interval, limit
    );

    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => {
            #[derive(serde::Deserialize)]
            struct BybitKlineResult { list: Vec<Vec<String>> }
            #[derive(serde::Deserialize)]
            struct BybitKlineResp { result: BybitKlineResult }
            
            let data: BybitKlineResp = match r.json().await {
                Ok(d) => d,
                Err(_) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": "Parse JSON Bybit" }))
                }
            };
            
            // Format sparklines générique type Binance Kline (listes de valeurs)
            let mut formatted: Vec<Vec<serde_json::Value>> = data.result.list.into_iter().map(|row| {
                vec![
                    serde_json::Value::Number(row.first().unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0).into()), // Ts
                    serde_json::Value::String(row.get(1).unwrap_or(&"0".to_string()).clone()), // Open
                    serde_json::Value::String(row.get(2).unwrap_or(&"0".to_string()).clone()), // High
                    serde_json::Value::String(row.get(3).unwrap_or(&"0".to_string()).clone()), // Low
                    serde_json::Value::String(row.get(4).unwrap_or(&"0".to_string()).clone()), // Close
                    serde_json::Value::String(row.get(5).unwrap_or(&"0".to_string()).clone()), // Volume
                ]
            }).collect();
            
            formatted.reverse(); // Plus vieux au plus récent (Binance standard)
            HttpResponse::Ok().json(formatted)
        }
        Ok(r) => HttpResponse::BadGateway()
            .json(serde_json::json!({ "error": format!("Binance {}", r.status()) })),
        Err(e) => HttpResponse::BadGateway().json(serde_json::json!({ "error": e.to_string() })),
    }
}
