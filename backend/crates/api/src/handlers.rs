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
    let limit = query.limit.unwrap_or(200).min(5000) as usize;
    let force = query.force.unwrap_or(false);

    // Si on a forcé une mise à jour cryptos, on passe l'étape cache et on demande à l'API
    if force && asset.est_cotable_bybit() {
        let resultat = BinanceProvider
            .fetch_candles(asset.clone(), timeframe, limit)
            .await;
        if let Ok(bougies) = resultat {
            if !bougies.is_empty() {
                let _ = state.db.inserer_bougies(&asset, &timeframe, &bougies).await;
                return HttpResponse::Ok().json(bougies);
            }
        }
    }

    // 1. Cache DB — toutes sources (MT5 inclus pour avoir l'historique)
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

#[derive(Deserialize)]
pub struct PrixActuelQuery {
    pub ticker: String,
}

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

/// GET /api/prix-actuel?ticker=SNX
/// Retourne le prix spot Binance pour n'importe quel ticker USDT,
/// sans passer par parse_asset (pas de whitelist).
pub async fn get_prix_actuel(query: web::Query<PrixActuelQuery>) -> impl Responder {
    // Validation : uniquement lettres/chiffres (protection injection)
    let ticker = query.ticker.to_uppercase();
    if ticker.is_empty() || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Ticker invalide" }));
    }

    let symbole = format!("{}USDT", ticker);
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}",
        symbole
    );

    let client = &*crate::http_client::HTTP_CLIENT;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<BinancePrix>().await {
            Ok(data) => match data.price.parse::<f64>() {
                Ok(prix) => HttpResponse::Ok().json(serde_json::json!({
                    "ticker": ticker,
                    "prix": prix
                })),
                Err(_) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": "Prix non parsable" })),
            },
            Err(e) => {
                tracing::warn!("Décodage réponse Binance pour {}: {}", symbole, e);
                HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": "Réponse Binance invalide" }))
            }
        },
        Ok(resp) => {
            tracing::warn!("Binance {} HTTP {}", symbole, resp.status());
            HttpResponse::NotFound().json(
                serde_json::json!({ "error": format!("Ticker {} non trouvé sur Binance", ticker) }),
            )
        }
        Err(e) => {
            tracing::warn!("Requête Binance prix {}: {}", symbole, e);
            HttpResponse::ServiceUnavailable()
                .json(serde_json::json!({ "error": "Binance inaccessible" }))
        }
    }
}
