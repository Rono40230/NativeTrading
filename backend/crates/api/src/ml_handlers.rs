use actix_web::{web, HttpResponse, Responder};
use common::Asset;
use data::{providers::binance::BinanceProvider, DataProvider};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

#[derive(Deserialize)]
pub struct EntrainementQuery {
    pub asset: Option<String>,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ReponseEntrainement {
    pub success: bool,
    pub accuracy_rf: f64,
    pub accuracy_lstm: f64,
    pub nb_echantillons: usize,
    pub duree_ms: u128,
    pub message: String,
}

/// POST /api/ml/train?asset=BTC&timeframe=M15&limit=1000
/// Lance l'entraînement RF + LSTM sur les données Binance. Retour synchrone (~30–90s sur CPU).
pub async fn entrainer_ml(
    query: web::Query<EntrainementQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    let asset = parse_asset(query.asset.as_deref().unwrap_or("BTC"))
        .unwrap_or(Asset::BTC);
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(1000).min(2000) as usize;

    tracing::info!(
        "Entraînement ML demandé: {:?} {:?} limit={}",
        asset,
        timeframe,
        limit
    );
    let debut = Instant::now();

    // Récupération des bougies
    let provider = BinanceProvider::new();
    let bougies = match provider.fetch_candles(asset, timeframe, limit).await {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": format!("Données Binance: {}", e)
            }));
        }
    };

    let nb = bougies.len();
    if nb < 100 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Données insuffisantes: {} bougies (min 100)", nb)
        }));
    }

    let mut pipeline = state.pipeline_ml.lock().await;
    match pipeline.entrainer_sur_historique(&bougies, 5, 0.002) {
        Ok((acc_rf, acc_lstm)) => {
            let duree_ms = debut.elapsed().as_millis();
            tracing::info!(
                "Entraînement terminé en {}ms: RF={:.1}% LSTM={:.1}%",
                duree_ms,
                acc_rf * 100.0,
                acc_lstm * 100.0
            );
            HttpResponse::Ok().json(ReponseEntrainement {
                success: true,
                accuracy_rf: (acc_rf * 1000.0).round() / 1000.0,
                accuracy_lstm: (acc_lstm * 1000.0).round() / 1000.0,
                nb_echantillons: nb,
                duree_ms,
                message: format!(
                    "RF: {:.1}% | LSTM: {:.1}% ({} bougies en {}ms)",
                    acc_rf * 100.0,
                    acc_lstm * 100.0,
                    nb,
                    duree_ms
                ),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Échec entraînement: {}", e)
        })),
    }
}

/// GET /api/ml/status — état du pipeline ML
pub async fn statut_ml(state: web::Data<AppState>) -> impl Responder {
    let pipeline = state.pipeline_ml.lock().await;
    HttpResponse::Ok().json(serde_json::json!({
        "modele_pret": pipeline.est_pret(),
        "lstm_pret": pipeline.lstm.est_pret(),
    }))
}
