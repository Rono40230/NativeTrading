use actix_web::{web, HttpResponse, Responder};
use backtest::BacktestEngine;
use serde::Deserialize;
use strategies::{smc_directional::SmcDirectionalStrategy, straddle::StraddleStrategy};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

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

    let bougies = match state
        .db
        .obtenir_bougies(&asset, &timeframe, limit as i64)
        .await
    {
        Ok(b) if b.len() >= 30 => b,
        _ => {
            return HttpResponse::Ok().json(serde_json::json!({
                "total_trades": 0,
                "winning_trades": 0,
                "losing_trades": 0,
                "win_rate": 0.0,
                "capital_initial": capital,
                "capital_final": capital,
                "roi_pct": 0.0,
                "profit_net": 0.0,
                "sharpe_ratio": 0.0,
                "max_drawdown_pct": 0.0,
                "profit_factor": 0.0,
                "message": "IB Gateway non connecté — lancez IB Gateway pour obtenir des données"
            }));
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
