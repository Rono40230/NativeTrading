use actix_web::{web, HttpResponse, Responder};
use data::{providers::{BinanceProvider, IbGatewayProvider}, DataProvider};
use serde::{Deserialize, Serialize};


use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── IB Gateway status ───────────────────────────────────────────────────────

/// GET /api/ib/status — Tente une vraie connexion TCP à IB Gateway (timeout 5s)
pub async fn ib_status(state: web::Data<AppState>) -> impl Responder {
    let adresse = format!("127.0.0.1:{}", state.ib_port);
    let connexion = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ibapi::Client::connect(&adresse, state.ib_client_id + 10),
    )
    .await;

    match connexion {
        Ok(Ok(client)) => {
            let version = client.server_version();
            HttpResponse::Ok().json(serde_json::json!({
                "connecte": true,
                "adresse": adresse,
                "server_version": version
            }))
        }
        Ok(Err(e)) => HttpResponse::Ok().json(serde_json::json!({
            "connecte": false,
            "adresse": adresse,
            "erreur": format!("{}", e)
        })),
        Err(_) => HttpResponse::Ok().json(serde_json::json!({
            "connecte": false,
            "adresse": adresse,
            "erreur": "Timeout — IB Gateway ne répond pas (>5s)"
        })),
    }
}

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
        // Crypto
        serde_json::json!({ "id": "BTC",    "nom": "Bitcoin",      "type": "crypto" }),
        serde_json::json!({ "id": "ETH",    "nom": "Ethereum",     "type": "crypto" }),
        // Métaux
        serde_json::json!({ "id": "XAUUSD", "nom": "Or (Gold)",    "type": "metal" }),
        serde_json::json!({ "id": "XAGUSD", "nom": "Argent (Silver)", "type": "metal" }),
        // Forex
        serde_json::json!({ "id": "EURUSD", "nom": "Euro / Dollar",   "type": "forex" }),
        serde_json::json!({ "id": "GBPJPY", "nom": "Livre / Yen",     "type": "forex" }),
        serde_json::json!({ "id": "CADJPY", "nom": "CAD / Yen",       "type": "forex" }),
        serde_json::json!({ "id": "NZDJPY", "nom": "NZD / Yen",       "type": "forex" }),
        serde_json::json!({ "id": "USDCAD", "nom": "Dollar / CAD",    "type": "forex" }),
        serde_json::json!({ "id": "USDJPY", "nom": "Dollar / Yen",    "type": "forex" }),
        // Indices
        serde_json::json!({ "id": "DAX",    "nom": "DAX 40 (Allemagne)", "type": "indice" }),
        serde_json::json!({ "id": "NAS100", "nom": "Nasdaq 100",     "type": "indice" }),
        serde_json::json!({ "id": "SP500",  "nom": "S&P 500",        "type": "indice" }),
    ];
    HttpResponse::Ok().json(assets)
}

// ─── Candles ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CandlesQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
    /// Si true, ignore le cache DB et force un appel au provider
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
    // Plafond à 5000 : couvre M1×3j (4320), H4×30j (180), D1×90j (90), etc.
    let limit = query.limit.unwrap_or(200).min(5000) as usize;
    let force = query.force.unwrap_or(false);

    // 1. Cache local — ignorer si force=true (polling temps réel)
    if !force {
        if let Ok(bougies) = state.db.obtenir_bougies(&asset, &timeframe, limit as i64).await {
            if bougies.len() >= limit {
                return HttpResponse::Ok().json(bougies);
            }
        }
    }

    // 2. Fallback provider : Binance pour crypto, IB Gateway pour métaux
    let resultat = match &asset {
        common::Asset::BTC | common::Asset::ETH => {
            BinanceProvider.fetch_candles(asset.clone(), timeframe, limit).await
        }
        _ => {
            IbGatewayProvider::new(state.ib_port, state.ib_client_id)
                .fetch_candles(asset.clone(), timeframe, limit)
                .await
        }
    };
    match resultat {
        Ok(bougies) => {
            if let Err(e) = state.db.inserer_bougies(&asset, &timeframe, &bougies).await {
                tracing::warn!("Impossible de mettre en cache les bougies: {}", e);
            }
            HttpResponse::Ok().json(bougies)
        }
        Err(e) => {
            tracing::warn!("get_candles échoué pour {}: {}", query.asset, e);
            HttpResponse::Ok().json(Vec::<serde_json::Value>::new())
        }
    }
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
