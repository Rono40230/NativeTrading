use actix_web::{web, HttpResponse, Responder};
use data::{providers::BinanceProvider, DataProvider};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── Health ───────────────────────────────────────────────────────────────────

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ─── Candles ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CandlesQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
    pub force: Option<bool>,
}

pub async fn get_candles(
    state: web::Data<AppState>,
    query: web::Query<CandlesQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Asset non supporté. Voir GET /api/assets pour la liste complète." })),
    };

    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200).min(50_000) as usize;
    let force = query.force.unwrap_or(false);

    // 1. Cache DB — toutes sources : c'est LA source du chart. Le backfill
    //    profond y a déposé l'historique (BTC M15 : ~2 ans) et le WS la
    //    maintient à jour — le provider REST (plafonné 1000/requête) ne
    //    servirait qu'à tronquer la fenêtre demandée. `force` ne contourne
    //    plus la DB : il ne sert qu'au rattrapage si la DB est vide (voir 2).
    let _ = force;
    if let Ok(bougies) = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit as i64)
        .await
    {
        if !bougies.is_empty() {
            return HttpResponse::Ok().json(bougies);
        }
    }

    // 2. Pour les crypto : fallback Binance REST si cache vide (ou si l'option force a échoué mais le cache est vide)
    if asset.est_cotable_bybit() {
        let resultat = BinanceProvider
            .fetch_candles(asset.clone(), timeframe, limit)
            .await;
        match resultat {
            Ok(bougies) => {
                if let Err(e) = state.db.inserer_bougies(&asset, &timeframe, &bougies).await {
                    tracing::warn!("Impossible de mettre en cache les bougies crypto: {}", e);
                }
                return HttpResponse::Ok().json(bougies);
            }
            Err(e) => {
                tracing::warn!("get_candles Binance échoué pour {}: {}", query.asset, e);
            }
        }
    }
    // Pour les assets non-crypto sans cache : pas encore de provider REST.

    HttpResponse::Ok().json(Vec::<serde_json::Value>::new())
}

// ─── Prédiction ML ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PredictQuery {
    pub asset: String,
    pub timeframe: Option<String>,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/PredictionML.ts")]
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
            // Pas de données en cache → modèle non prêt, retourner 200
            return HttpResponse::Ok().json(ReponsePrediction {
                asset: query.asset.clone(),
                direction: "inconnu".to_string(),
                confiance: 0.0,
                est_confiant: false,
                modele_pret: false,
            });
        }
    };

    let pipeline = state.pipeline_ml.read().await;

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

// ─── Prix actuel (Binance spot, tout ticker) ──────────────────────────────────



