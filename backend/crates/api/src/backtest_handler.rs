//! Handlers HTTP pour le moteur de backtest.
//!
//! POST /api/backtest/lancer   — rejoue une stratégie sur les données historiques
//! GET  /api/backtest/recommandations — recommandations basées sur le dernier résultat

use actix_web::{web, HttpResponse, Responder};
use backtest::{
    recommandations::analyser_recommandations, BacktestConfig, ParamsSmc, ParamsStraddle,
    StrategieParams, StrategieType,
};
use chrono::{DateTime, Utc};
use db::strategies_params;
use serde::Deserialize;

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RequeteBacktest {
    /// Ex: "BTC", "XAUUSD"
    pub asset: String,
    /// Ex: "M15", "H1"
    pub timeframe: String,
    /// Stratégie: "straddle" | "smc" | "rockets"
    pub strategie: String,
    /// ISO 8601 ex: "2025-01-01T00:00:00Z"
    pub debut: Option<DateTime<Utc>>,
    /// ISO 8601 ex: "2025-12-31T23:59:59Z"
    pub fin: Option<DateTime<Utc>>,
    /// Capital initial en USD (défaut 10 000)
    pub capital: Option<f64>,
    /// Risque par trade en fraction (défaut 0.02 = 2%)
    pub risque: Option<f64>,
    /// Nombre de jours de données à charger (défaut 90)
    pub nb_jours: Option<u32>,
}

// ── POST /api/backtest/lancer ─────────────────────────────────────────────────

pub async fn lancer_backtest(
    state: web::Data<AppState>,
    body: web::Json<RequeteBacktest>,
) -> impl Responder {
    let asset = match parse_asset(body.asset.trim()) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté" }))
        }
    };

    let timeframe = parse_timeframe(body.timeframe.trim());

    let strategie =
        match body.strategie.to_lowercase().as_str() {
            "straddle" => StrategieType::Straddle,
            "smc" => StrategieType::Smc,
            "rockets" => StrategieType::Rockets,
            autre => return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Stratégie inconnue : '{autre}'. Valeurs: straddle, smc, rockets")
            })),
        };

    let nb_jours = body.nb_jours.unwrap_or(90);
    let bougies = match state
        .db
        .obtenir_bougies_depuis_jours(&asset, &timeframe, nb_jours)
        .await
    {
        Ok(b) if b.len() >= 30 => b,
        Ok(b) => {
            return HttpResponse::UnprocessableEntity().json(serde_json::json!({
                "error": format!("Données insuffisantes : {} bougies (min 30)", b.len())
            }))
        }
        Err(e) => {
            tracing::error!("DB bougies backtest: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur chargement données" }));
        }
    };

    let debut = body
        .debut
        .unwrap_or_else(|| bougies.first().map(|b| b.timestamp).unwrap_or(Utc::now()));
    let fin = body
        .fin
        .unwrap_or_else(|| bougies.last().map(|b| b.timestamp).unwrap_or(Utc::now()));

    // Lecture des paramètres depuis la DB pour fidélité live
    let params = match strategie {
        StrategieType::Straddle => {
            let p = strategies_params::lire_straddle_params(state.db.pool()).await;
            StrategieParams::Straddle(ParamsStraddle {
                atr_periode:      p.atr_periode as usize,
                atr_seuil:        p.atr_seuil,
                tp_mult_1:        p.tp_mult_1,
                tp_mult_2:        p.tp_mult_2,
                tp_mult_3:        p.tp_mult_3,
                sl_mult:          p.sl_mult,
                trailing_atr:     p.trailing_atr,
                vente_partielle:  p.vente_partielle,
                pct_cloture_tp1:  p.pct_cloture_tp1,
                pct_cloture_tp2:  p.pct_cloture_tp2,
            })
        }
        StrategieType::Smc => {
            let p = strategies_params::lire_smc_params(state.db.pool()).await;
            StrategieParams::Smc(ParamsSmc {
                atr_periode:      p.atr_periode as usize,
                score_min:        p.score_min as f64,
                atr_tp1:          p.atr_tp1,
                atr_tp2:          p.atr_tp2,
                atr_tp3:          p.atr_tp3,
                atr_sl:           p.atr_sl,
                vente_partielle:  p.vente_partielle,
                kill_zone_filtre: p.kill_zone_filtre,
                pct_cloture_tp1:  p.pct_cloture_tp1,
                pct_cloture_tp2:  p.pct_cloture_tp2,
            })
        }
        StrategieType::Rockets => StrategieParams::Rockets,
    };

    let config = BacktestConfig {
        asset,
        timeframe,
        debut,
        fin,
        strategie,
        capital_initial: body.capital.unwrap_or(10_000.0),
        risque_par_trade: body.risque.unwrap_or(0.02).clamp(0.001, 0.05),
        params,
    };

    let debut_calcul = std::time::Instant::now();
    let result = match backtest::engine::rejouer(&bougies, config) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Erreur backtest: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Erreur moteur: {e}") }));
        }
    };
    let duree_ms = debut_calcul.elapsed().as_millis();

    tracing::info!(
        "Backtest {:?}/{} terminé : {} trades, win_rate={:.1}%, sharpe={:.2} ({} ms)",
        result.config.strategie,
        result.config.timeframe.as_str(),
        result.nb_trades,
        result.win_rate * 100.0,
        result.sharpe,
        duree_ms,
    );

    let recommandations = analyser_recommandations(&result);

    HttpResponse::Ok().json(serde_json::json!({
        "result": result,
        "recommandations": recommandations,
        "duree_ms": duree_ms,
    }))
}
