use actix_web::{web, HttpResponse, Responder};
use db::rockets::{self, NouveauRocket};
use serde::Deserialize;
use std::time::Duration;

use crate::rockets_scan;
use crate::state::AppState;

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteSauvegarder {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
}

#[derive(Deserialize)]
pub struct QueryHistorique {
    pub limite: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/rockets/signal — enregistre un signal détecté côté frontend
pub async fn sauvegarder_signal(
    state: web::Data<AppState>,
    body: web::Json<RequeteSauvegarder>,
) -> impl Responder {
    let pool = state.db.pool();
    let nouveau = NouveauRocket {
        ticker:       body.ticker.clone(),
        phase:        body.phase.clone(),
        score:        body.score,
        prix_entree:  body.prix_entree,
        stop_loss:    body.stop_loss,
        target:       body.target,
        target2:      body.target2,
        target3:      body.target3,
        ratio_volume: body.ratio_volume,
        atr_ratio:    body.atr_ratio,
        rsi:          body.rsi,
    };
    match rockets::sauvegarder(pool, &nouveau).await {
        Ok(Some(id)) => HttpResponse::Ok().json(serde_json::json!({ "id": id, "nouveau": true })),
        Ok(None)     => HttpResponse::Ok().json(serde_json::json!({ "nouveau": false })),
        Err(e) => {
            tracing::error!("Sauvegarde rocket: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReponse<'a> {
    signaux: &'a [rockets_scan::ScanResultat],
    total_candidats: usize,
}

/// GET /api/rockets/scan — résultats du dernier scan worker
pub async fn get_scan() -> impl Responder {
    let results  = rockets_scan::get_scan_results();
    let total    = rockets_scan::get_total_candidats();
    let locked   = results.read().await;
    let nb_total = *total.read().await;
    HttpResponse::Ok().json(ScanReponse { signaux: &*locked, total_candidats: nb_total })
}

/// GET /api/rockets/historique?limite=50
pub async fn get_historique(
    state: web::Data<AppState>,
    query: web::Query<QueryHistorique>,
) -> impl Responder {
    let pool = state.db.pool();
    let limite = query.limite.unwrap_or(50);
    match rockets::historique(pool, limite).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique rockets: {}", e);
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

async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

/// Worker lancé au démarrage : toutes les 15min, statue TP/SL/expiration.
pub async fn demarrer_worker_suivi(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => { tracing::error!("Worker rockets HTTP: {}", e); return; }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;

        // 1. Marquer les signaux ouverts depuis plus de 4h comme expirés
        if let Ok(n) = rockets::marquer_expires(&pool).await {
            if n > 0 { tracing::info!("Rockets: {} signal(s) expirés", n); }
        }

        // 2. Vérifier TP / SL sur les signaux encore ouverts
        let signaux = match rockets::lister_ouverts(&pool).await {
            Ok(s) => s,
            Err(e) => { tracing::warn!("Worker rockets liste: {}", e); continue; }
        };

        for s in &signaux {
            let prix = match fetch_prix(&client, &s.ticker).await {
                Some(p) => p,
                None    => continue,
            };
            let verdict = if prix >= s.target {
                Some("confirme")
            } else if prix <= s.stop_loss {
                Some("invalide")
            } else {
                None
            };
            if let Some(v) = verdict {
                if let Err(e) = rockets::maj_verdict(&pool, s.id, v, prix).await {
                    tracing::warn!("Worker rockets verdict: {}", e);
                } else {
                    tracing::info!("Rocket {} → {} @ {:.5}", s.ticker, v, prix);
                }
            }
        }
    }
}
