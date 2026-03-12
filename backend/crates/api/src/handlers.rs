use actix_web::{web, HttpResponse, Responder};
use backtest::BacktestEngine;
use serde::{Deserialize, Serialize};
use strategies::{smc_directional::SmcDirectionalStrategy, straddle::StraddleStrategy};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── Health ───────────────────────────────────────────────────────────────────

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ─── Assets ───────────────────────────────────────────────────────────────────

pub async fn get_assets() -> impl Responder {
    let assets = vec![
        serde_json::json!({ "id": "BTC", "nom": "Bitcoin", "type": "crypto" }),
        serde_json::json!({ "id": "ETH", "nom": "Ethereum", "type": "crypto" }),
        serde_json::json!({ "id": "XAUUSD", "nom": "Or (Gold)", "type": "metal" }),
        serde_json::json!({ "id": "XAGUSD", "nom": "Argent (Silver)", "type": "metal" }),
    ];
    HttpResponse::Ok().json(assets)
}

// ─── Candles ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CandlesQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

pub async fn get_candles(
    state: web::Data<AppState>,
    query: web::Query<CandlesQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Asset non supporté: BTC, ETH, XAUUSD ou XAGUSD." })),
    };

    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200).min(1000) as usize;

    // Essayer d'abord la base de données locale
    let bougies_db = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit as i64)
        .await;

    let bougies = match bougies_db {
        Ok(b) if b.len() >= 60 => b,
        _ => {
            // IB Gateway sera le seul provider — stub jusqu'à l'intégration
            return HttpResponse::ServiceUnavailable().json(
                serde_json::json!({ "error": "IB Gateway non connecté — données non disponibles" }),
            );
        }
    };

    HttpResponse::Ok().json(bougies)
}

// ─── Signaux ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SignauxQuery {
    pub limit: Option<i64>,
}

pub async fn get_signaux(
    state: web::Data<AppState>,
    query: web::Query<SignauxQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20).min(100);
    match state.db.obtenir_signaux(limit).await {
        Ok(signaux) => HttpResponse::Ok().json(signaux),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ─── Prédiction ML ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PredictQuery {
    pub asset: String,
    pub timeframe: Option<String>,
}

#[derive(Serialize)]
pub struct ReponsePrediction {
    pub asset: String,
    pub direction: String,
    pub confiance: f64,
    pub est_confiant: bool,
    pub modele_pret: bool,
}

pub async fn predict_ml(
    state: web::Data<AppState>,
    query: web::Query<PredictQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté." }));
        }
    };

    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));

    let bougies = match state.db.obtenir_bougies(&asset, &timeframe, 100).await {
        Ok(b) if !b.is_empty() => b,
        _ => {
            return HttpResponse::ServiceUnavailable()
                .json(serde_json::json!({ "error": "IB Gateway non connecté — données non disponibles" }));
        }
    };

    let pipeline = state.pipeline_ml.lock().await;

    if !pipeline.est_pret() {
        return HttpResponse::Ok().json(ReponsePrediction {
            asset: query.asset.clone(),
            direction: "inconnu".to_string(),
            confiance: 0.0,
            est_confiant: false,
            modele_pret: false,
        });
    }

    match pipeline.predire(&bougies) {
        Ok(pred) => HttpResponse::Ok().json(ReponsePrediction {
            asset: query.asset.clone(),
            direction: format!("{:?}", pred.direction),
            confiance: pred.confiance,
            est_confiant: pred.est_confiant,
            modele_pret: true,
        }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ─── Backtest ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BacktestRequest {
    pub asset: String,
    pub timeframe: Option<String>,
    pub capital: Option<f64>,
    pub limit: Option<u32>,
    /// "straddle" (défaut) ou "smc"
    pub strategie: Option<String>,
}

pub async fn run_backtest(
    state: web::Data<AppState>,
    body: web::Json<BacktestRequest>,
) -> impl Responder {
    let asset = match parse_asset(&body.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté." }));
        }
    };

    let timeframe = parse_timeframe(body.timeframe.as_deref().unwrap_or("M15"));
    let capital = body.capital.unwrap_or(2000.0);
    let limit = body.limit.unwrap_or(500).min(1000) as usize;
    let strategie = body.strategie.as_deref().unwrap_or("straddle");

    tracing::info!(
        "Backtest: {} {:?} capital={} strategie={}",
        body.asset,
        timeframe,
        capital,
        strategie
    );

    let bougies = match state.db.obtenir_bougies(&asset, &timeframe, limit as i64).await {
        Ok(b) if b.len() >= 30 => b,
        _ => {
            return HttpResponse::ServiceUnavailable()
                .json(serde_json::json!({ "error": "IB Gateway non connecté — données insuffisantes pour le backtest" }));
        }
    };

    let engine = BacktestEngine::new(capital);

    let result = match strategie {
        "smc" => {
            let strat = SmcDirectionalStrategy;
            engine.run(&bougies, &strat)
        }
        _ => {
            let strat = StraddleStrategy::new();
            engine.run(&bougies, &strat)
        }
    };

    match result {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}
