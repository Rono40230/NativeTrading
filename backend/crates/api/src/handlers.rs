use actix_web::{web, HttpResponse, Responder};
use data::{providers::BinanceProvider, DataProvider};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── IG Markets status ────────────────────────────────────────────────────────

/// GET /api/ig/status — Force un re-login IG (bouton "Tester" dans Settings).
pub async fn ig_status(state: web::Data<AppState>) -> impl Responder {
    match state
        .ig_session
        .lock()
        .await
        .tester_connexion(&state.db)
        .await
    {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "connecte": true,
            "source": "ig_markets"
        })),
        Err(e) => HttpResponse::Ok().json(serde_json::json!({
            "connecte": false,
            "source": "ig_markets",
            "erreur": format!("{}", e)
        })),
    }
}

/// GET /api/ig/statut-local — Retourne l'état de la session IG sans appel réseau.
/// Utilisé par le Dashboard pour afficher le badge sans provoquer de re-login.
pub async fn ig_statut_local(state: web::Data<AppState>) -> impl Responder {
    let connecte = state.ig_session.lock().await.est_connecte();
    HttpResponse::Ok().json(serde_json::json!({
        "connecte": connecte,
        "source": "ig_markets"
    }))
}

/// GET /api/ig/search?q=EURUSD
/// Recherche les marchés disponibles sur IG pour un terme donné.
/// Utilisé pour découvrir les epics valides pour le compte connecté.
pub async fn ig_search_markets(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let terme = match query.get("q") {
        Some(t)
            if !t.is_empty() && t.len() <= 20 && t.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            t.to_uppercase()
        }
        _ => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Paramètre q requis (ex: EURUSD)" }))
        }
    };

    let (url_base, headers, client) = {
        let mut session = state.ig_session.lock().await;
        let url_base = session.url();
        let headers = match session.headers(&state.db).await {
            Ok(h) => h,
            Err(e) => {
                return HttpResponse::ServiceUnavailable()
                    .json(serde_json::json!({ "error": format!("Session IG: {}", e) }))
            }
        };
        let client = session.client().clone();
        (url_base, headers, client)
    };

    let url = format!("{}/markets?searchTerm={}", url_base, terme);
    match client
        .get(&url)
        .headers(headers)
        .header("Version", "1")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(data) => HttpResponse::Ok().json(data),
            Err(e) => HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Parse: {}", e) })),
        },
        Ok(r) => HttpResponse::BadGateway()
            .json(serde_json::json!({ "error": format!("IG {}", r.status()) })),
        Err(e) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
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

    // 2. Pour les crypto : fallback Binance REST si cache vide
    if asset.is_crypto() {
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
    // Pour les assets IG sans cache : le WebSocket stream_ig gère l'initialisation
    // (fetch_historique avec protection anti-403 intégrée)

    HttpResponse::Ok().json(Vec::<serde_json::Value>::new())
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
