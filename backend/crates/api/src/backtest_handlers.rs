use actix_web::{web, HttpResponse, Responder};
use backtest::BacktestEngine;
use db::strategies_params::StraddleParams;
use serde::Deserialize;
use strategies::{
    smc_directional::SmcDirectionalStrategy,
    straddle::{StraddleCreneauStrategy, StraddleStrategy},
};

use crate::ollama::formater_contexte_backtest;
use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

#[derive(Deserialize)]
pub struct BacktestRequest {
    pub asset: String,
    pub timeframe: Option<String>,
    pub capital: Option<f64>,
    /// Nombre de jours à remonter (filtre par date réelle, pas par nombre de bougies)
    pub nb_jours: Option<u32>,
    /// "straddle" (défaut) ou "smc"
    pub strategie: Option<String>,
    /// Filtre créneau Straddle : timing précis du pic ("HH:MM", ex: "14:32").
    pub timing_optimal: Option<String>,
    /// Tolérance ± en minutes autour du pic (défaut : 10 min).
    pub fenetre_min: Option<u32>,
    pub jour_semaine: Option<i64>,
    // ── Paramètres StraddleParams injectables (optimisation LLM) ──────────────
    pub tp_mult_1: Option<f64>,
    pub tp_mult_2: Option<f64>,
    pub tp_mult_3: Option<f64>,
    pub sl_mult: Option<f64>,
    pub seuil_atr: Option<f64>,
    pub atr_periode: Option<i64>,
    pub horizon_bougies: Option<i64>,
    pub trailing_atr: Option<f64>,
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
    let nb_jours = body.nb_jours.unwrap_or(90);
    let strategie = body.strategie.as_deref().unwrap_or("straddle");

    tracing::info!(
        "Backtest: {} {:?} capital={} strategie={} nb_jours={}",
        body.asset,
        timeframe,
        capital,
        strategie,
        nb_jours
    );

    let bougies = match state
        .db
        .obtenir_bougies_depuis_jours(&asset, &timeframe, nb_jours)
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

    // Horizon adapté au timeframe : Straddle = 30 min, SMC = 240 min (4h)
    let horizon_minutes: u64 = if strategie == "smc" { 240 } else { 30 };
    let horizon_bougies = body
        .horizon_bougies
        .map(|h| h as usize)
        .unwrap_or_else(|| ((horizon_minutes / timeframe.minutes()) as usize).max(2));
    let engine = BacktestEngine {
        horizon_bougies,
        ..BacktestEngine::new(capital)
    };

    let result = match strategie {
        "smc" => engine.run(&bougies, &SmcDirectionalStrategy::default()),
        _ => {
            let params = straddle_params_from_body(&body);
            match &body.timing_optimal {
                Some(timing) => {
                    let fenetre = body.fenetre_min.unwrap_or(10);
                    let strat = StraddleCreneauStrategy::avec_params(
                        timing,
                        fenetre,
                        body.jour_semaine,
                        params,
                    );
                    engine.run(&bougies, &strat)
                }
                None => engine.run(&bougies, &StraddleStrategy::avec_params(params)),
            }
        }
    };

    match result {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

/// POST /api/backtest/raffiner-ml
///
/// Lance le backtest avec feedback et raffine le pipeline ML depuis les outcomes réels.
/// Stocke aussi le contexte backtest dans AppState pour enrichir les analyses Ollama.
pub async fn raffiner_ml(
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
    let nb_jours = body.nb_jours.unwrap_or(90);
    let strategie = body.strategie.as_deref().unwrap_or("straddle");

    let bougies = match state
        .db
        .obtenir_bougies_depuis_jours(&asset, &timeframe, nb_jours)
        .await
    {
        Ok(b) if b.len() >= 62 => b,
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Données insuffisantes — IB Gateway requis"
            }));
        }
    };

    let horizon_minutes: u64 = if strategie == "smc" { 240 } else { 30 };
    let horizon_bougies = ((horizon_minutes / timeframe.minutes()) as usize).max(2);
    let engine = BacktestEngine {
        horizon_bougies,
        ..BacktestEngine::new(capital)
    };
    let (results, feedback) = match strategie {
        "smc" => match engine.run_avec_feedback(&bougies, &SmcDirectionalStrategy::default()) {
            Ok(r) => r,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": format!("{}", e) }))
            }
        },
        _ => {
            let params = straddle_params_from_body(&body);
            let res = match &body.timing_optimal {
                Some(timing) => {
                    let fenetre = body.fenetre_min.unwrap_or(10);
                    let strat = StraddleCreneauStrategy::avec_params(
                        timing,
                        fenetre,
                        body.jour_semaine,
                        params,
                    );
                    engine.run_avec_feedback(&bougies, &strat)
                }
                None => engine.run_avec_feedback(&bougies, &StraddleStrategy::avec_params(params)),
            };
            match res {
                Ok(r) => r,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": format!("{}", e) }))
                }
            }
        }
    };

    // Raffinement ML depuis les outcomes réels
    let indices: Vec<(usize, bool)> = feedback
        .iter()
        .map(|f| (f.indice_entree, f.gagne))
        .collect();
    let mut pipeline = state.pipeline_ml.lock().await;
    let nb_raffines = pipeline
        .raffiner_depuis_backtest(&bougies, &indices)
        .unwrap_or(0);
    drop(pipeline);

    // Stockage du contexte backtest pour les analyses Ollama
    let ctx = formater_contexte_backtest(
        results.win_rate,
        results.roi_pct,
        results.sharpe_ratio,
        results.max_drawdown_pct,
        results.profit_factor,
        results.total_trades,
        &body.asset,
        strategie,
    );
    *state.contexte_backtest.write().await = Some(ctx);

    tracing::info!(
        "Raffinement ML depuis backtest: {} trades → {} échantillons",
        feedback.len(),
        nb_raffines
    );

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "backtest": results,
        "ml_raffine": nb_raffines >= 50,
        "nb_echantillons_raffines": nb_raffines,
        "contexte_backtest_stocke": true,
    }))
}

/// Construit un `StraddleParams` depuis les champs optionnels du `BacktestRequest`.
/// Les champs absents utilisent la valeur par défaut DB (atr_periode, horizon_bougies, trailing_atr).
fn straddle_params_from_body(body: &BacktestRequest) -> StraddleParams {
    let def = StraddleParams::default();
    StraddleParams {
        atr_periode: body.atr_periode.unwrap_or(def.atr_periode),
        atr_seuil: body.seuil_atr.unwrap_or(def.atr_seuil),
        tp_mult_1: body.tp_mult_1.unwrap_or(def.tp_mult_1),
        tp_mult_2: body.tp_mult_2.unwrap_or(def.tp_mult_2),
        tp_mult_3: body.tp_mult_3.unwrap_or(def.tp_mult_3),
        sl_mult: body.sl_mult.unwrap_or(def.sl_mult),
        horizon_bougies: body.horizon_bougies.unwrap_or(def.horizon_bougies),
        trailing_atr: body.trailing_atr.unwrap_or(def.trailing_atr),
    }
}
