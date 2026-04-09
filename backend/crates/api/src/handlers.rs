use actix_web::{web, HttpResponse, Responder};
use data::{
    providers::{BinanceProvider, IbGatewayProvider},
    DataProvider,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── IB Gateway status ───────────────────────────────────────────────────────

/// GET /api/ib/status — Vérifie la connexion IB Gateway.
/// Résultat mis en cache 90 secondes pour éviter de saturer IB Gateway
/// avec des sessions ibapi non fermées (une par appel = fuites de clients).
pub async fn ib_status(state: web::Data<AppState>) -> impl Responder {
    // — Lecture cache —————————————————————————————
    {
        let cache = state.ib_status_cache.read().await;
        if let Some((t, connecte, adresse, erreur)) = cache.as_ref() {
            if t.elapsed() < std::time::Duration::from_secs(90) {
                let mut resp = serde_json::json!({ "connecte": connecte, "adresse": adresse, "cache": true });
                if let Some(e) = erreur {
                    resp["erreur"] = serde_json::Value::String(e.clone());
                }
                return HttpResponse::Ok().json(resp);
            }
        }
    }

    // — Contrôle réel (une seule fois toutes les 90s) —————————
    let port = match state.db.lire_config("ibgateway_port").await {
        Ok(Some(v)) => v.parse::<u16>().unwrap_or(state.ib_port),
        _ => state.ib_port,
    };
    let adresse = format!("127.0.0.1:{}", port);
    let connexion = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ibapi::Client::connect(&adresse, state.ib_client_id + 10),
    )
    .await;

    let (connecte, erreur) = match connexion {
        Ok(Ok(_client)) => (true, None),
        Ok(Err(e)) => (false, Some(format!("{}", e))),
        Err(_) => (false, Some("Timeout — IB Gateway ne répond pas (>5s)".to_string())),
    };

    // — Mise à jour cache ——————————————————————————
    {
        let mut cache = state.ib_status_cache.write().await;
        *cache = Some((std::time::Instant::now(), connecte, adresse.clone(), erreur.clone()));
    }

    let mut resp = serde_json::json!({ "connecte": connecte, "adresse": adresse });
    if let Some(e) = erreur {
        resp["erreur"] = serde_json::Value::String(e);
    }
    HttpResponse::Ok().json(resp)
}

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
    // Condition : retourner le cache si on a UNE bougie au minimum.
    // On ne conditionne pas sur `len >= limit` car certains assets (ex: CADJPY W1)
    // n'ont jamais assez de bougies IB → boucle infinie de reconnexions IB.
    if !force {
        if let Ok(bougies) = state
            .db
            .obtenir_bougies(&asset, &timeframe, limit as i64)
            .await
        {
            if !bougies.is_empty() {
                return HttpResponse::Ok().json(bougies);
            }
        }
    }

    // 2. Fallback provider : Binance pour crypto, IB Gateway pour métaux/forex/indices
    let resultat = if asset.is_crypto() {
        BinanceProvider
            .fetch_candles(asset.clone(), timeframe, limit)
            .await
    } else {
        IbGatewayProvider::new(state.ib_port, state.ib_client_id)
            .fetch_candles(asset.clone(), timeframe, limit)
            .await
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

// ─── Prix actuel (Binance spot, tout ticker) ──────────────────────────────────

#[derive(Deserialize)]
pub struct PrixActuelQuery {
    pub ticker: String,
}

#[derive(serde::Deserialize)]
struct BinanceTickerPrix {
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

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Création client reqwest: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<BinanceTickerPrix>().await {
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
