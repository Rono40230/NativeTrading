use actix_web::{web, HttpResponse, Responder};
use db::signaux;
use serde::Deserialize;
use std::time::Duration;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct QuerySignaux {
    pub limit: Option<i64>,
}

/// GET /api/signaux?limit=N — historique avec verdict inclus
pub async fn get_signaux(
    state: web::Data<AppState>,
    query: web::Query<QuerySignaux>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(500);
    match state.db.obtenir_signaux(limit).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique signaux: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

async fn fetch_prix_binance(client: &reqwest::Client, asset: &str) -> Option<f64> {
    let symbole = match asset {
        "BTC" => "BTCUSDT",
        "ETH" => "ETHUSDT",
        "SOL" => "SOLUSDT",
        _     => return None,
    };
    let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", symbole);
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

fn calculer_verdict(
    direction: &str,
    stop_loss: f64,
    take_profit: &[f64],
    prix: f64,
) -> Option<&'static str> {
    let long = direction.to_uppercase().contains("LONG");
    if long {
        if prix <= stop_loss                                  { return Some("SL"); }
        if take_profit.get(2).map_or(false, |&t| prix >= t)  { return Some("TP3"); }
        if take_profit.get(1).map_or(false, |&t| prix >= t)  { return Some("TP2"); }
        if take_profit.first().map_or(false, |&t| prix >= t) { return Some("TP1"); }
    } else {
        if prix >= stop_loss                                  { return Some("SL"); }
        if take_profit.get(2).map_or(false, |&t| prix <= t)  { return Some("TP3"); }
        if take_profit.get(1).map_or(false, |&t| prix <= t)  { return Some("TP2"); }
        if take_profit.first().map_or(false, |&t| prix <= t) { return Some("TP1"); }
    }
    None
}

/// Worker lancé au démarrage : toutes les 5min, vérifie TP/SL des signaux SMC/Straddle.
pub async fn demarrer_worker_suivi_signaux(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => { tracing::error!("Worker signaux HTTP: {}", e); return; }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;

        if let Ok(n) = signaux::expirer_anciens(&pool).await {
            if n > 0 { tracing::info!("Signaux: {} expiré(s)", n); }
        }

        let actifs = match signaux::lister_actifs(&pool).await {
            Ok(s) => s,
            Err(e) => { tracing::warn!("Worker signaux liste: {}", e); continue; }
        };

        for s in &actifs {
            let prix = match fetch_prix_binance(&client, &s.asset).await {
                Some(p) => p,
                None    => continue,
            };
            if let Some(v) = calculer_verdict(&s.direction, s.stop_loss, &s.take_profit, prix) {
                match signaux::maj_verdict(&pool, &s.id, v, prix).await {
                    Ok(_)  => tracing::info!("Signal {} {} → {} @ {:.4}", s.asset, s.direction, v, prix),
                    Err(e) => tracing::warn!("Worker signaux verdict: {}", e),
                }
            }
        }
    }
}
