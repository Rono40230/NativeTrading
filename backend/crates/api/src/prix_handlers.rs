//! GET /api/prix?assets=XAUUSD,BTC,EURUSD
//! Retourne les prix actuels depuis Binance (crypto) ou Yahoo Finance (autres).
use actix_web::{web, HttpResponse, Responder};
use futures_util::future::join_all;
use std::collections::HashMap;

use crate::prix_utils;

#[derive(serde::Deserialize)]
pub struct PrixQuery {
    pub assets: String,
}

/// GET /api/prix?assets=XAUUSD,BTC,EURUSD
/// Retourne `{ "XAUUSD": 3200.5, "BTC": 85000.0, … }`.
/// Assets inconnus ou sources inaccessibles sont silencieusement omis.
pub async fn get_prix(query: web::Query<PrixQuery>) -> impl Responder {
    let client = match prix_utils::client_http() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client HTTP /api/prix: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    // Validation : uniquement alphanumériques, max 50 assets, noms ≤ 10 chars
    let assets: Vec<String> = query
        .assets
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(50)
        .collect();

    let futs: Vec<_> = assets
        .iter()
        .map(|asset| {
            let c = client.clone();
            let a = asset.clone();
            async move { (a.clone(), prix_utils::fetch_prix_asset(&c, &a).await) }
        })
        .collect();

    let resultats = join_all(futs).await;

    let map: HashMap<String, f64> = resultats
        .into_iter()
        .filter_map(|(asset, prix)| prix.map(|p| (asset, p)))
        .collect();

    HttpResponse::Ok().json(map)
}
