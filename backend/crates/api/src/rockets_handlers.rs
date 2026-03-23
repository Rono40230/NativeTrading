use actix_web::{web, HttpResponse, Responder};
use db::rockets::{self, NouveauRocket};
use serde::Deserialize;
use std::time::Duration;

use crate::rockets_scan;
use crate::state::AppState;

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteSauvegarder {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub atr14: Option<f64>,
    pub rsi: f64,
}

#[derive(Deserialize)]
pub struct QueryHistorique {
    pub limite: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/rockets/signal — enregistre un signal détecté côté frontend
pub async fn sauvegarder_signal(
    state: web::Data<AppState>,
    body: web::Json<RequeteSauvegarder>,
) -> impl Responder {
    let pool = state.db.pool();
    let nouveau = NouveauRocket {
        ticker:       body.ticker.clone(),
        phase:        body.phase.clone(),
        score:        body.score,
        prix_entree:  body.prix_entree,
        stop_loss:    body.stop_loss,
        target:       body.target,
        target2:      body.target2,
        target3:      body.target3,
        ratio_volume: body.ratio_volume,
        atr_ratio:    body.atr_ratio,
        atr14:        body.atr14,
        rsi:          body.rsi,
    };
    match rockets::sauvegarder(pool, &nouveau).await {
        Ok(Some(id)) => HttpResponse::Ok().json(serde_json::json!({ "id": id, "nouveau": true })),
        Ok(None)     => HttpResponse::Ok().json(serde_json::json!({ "nouveau": false })),
        Err(e) => {
            tracing::error!("Sauvegarde rocket: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReponse<'a> {
    signaux: &'a [rockets_scan::ScanResultat],
    total_candidats: usize,
}

/// GET /api/rockets/scan — résultats du dernier scan worker
pub async fn get_scan() -> impl Responder {
    let results  = rockets_scan::get_scan_results();
    let total    = rockets_scan::get_total_candidats();
    let locked   = results.read().await;
    let nb_total = *total.read().await;
    HttpResponse::Ok().json(ScanReponse { signaux: &*locked, total_candidats: nb_total })
}

/// GET /api/rockets/historique?limite=50
pub async fn get_historique(
    state: web::Data<AppState>,
    query: web::Query<QueryHistorique>,
) -> impl Responder {
    let pool = state.db.pool();
    let limite = query.limite.unwrap_or(50);
    match rockets::historique(pool, limite).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique rockets: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

/// Worker lancé au démarrage : toutes les 15min, gère cycle de vie complet.
pub async fn demarrer_worker_suivi(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => { tracing::error!("Worker rockets HTTP: {}", e); return; }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;

        // 1. Expirer les signaux EN ATTENTE depuis >6h (position jamais ouverte)
        if let Ok(n) = rockets::marquer_expires(&pool).await {
            if n > 0 { tracing::info!("Rockets: {} signal(s) expirés (jamais entrés)", n); }
        }

        // 2. Signaux en attente : vérifier si prix_entree atteint → ouvrir position
        let en_attente = match rockets::lister_en_attente(&pool).await {
            Ok(s) => s,
            Err(e) => { tracing::warn!("Worker rockets attente: {}", e); continue; }
        };
        for s in &en_attente {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else { continue };
            if prix >= s.prix_entree {
                if let Err(e) = rockets::entrer_position(&pool, s.id).await {
                    tracing::warn!("Rocket {} entrée position: {}", s.ticker, e);
                } else {
                    tracing::info!("Rocket {} → ouvert @ {:.5}", s.ticker, prix);
                }
            }
        }

        // 3. Signaux OUVERTS : TP pyramidal + trailing TP3 + SL
        let signaux = match rockets::lister_ouverts(&pool).await {
            Ok(s) => s,
            Err(e) => { tracing::warn!("Worker rockets ouverts: {}", e); continue; }
        };
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else { continue };

            // Mettre à jour le prix peak
            let peak = s.prix_peak.unwrap_or(s.prix_entree).max(prix);
            if peak > s.prix_peak.unwrap_or(0.0) {
                if let Err(e) = rockets::maj_prix_peak(&pool, s.id, peak).await {
                    tracing::warn!("Rocket {} maj peak: {}", s.ticker, e);
                }
            }

            let verdict = calculer_verdict_rocket(&s, prix, peak);
            if let Some(v) = verdict {
                if let Err(e) = rockets::maj_verdict(&pool, s.id, v, prix).await {
                    tracing::warn!("Worker rockets verdict: {}", e);
                } else {
                    tracing::info!("Rocket {} → {} @ {:.5}", s.ticker, v, prix);
                }
            }
        }
    }
}

fn calculer_verdict_rocket(
    s: &db::rockets::RocketSignal,
    prix: f64,
    peak: f64,
) -> Option<&'static str> {
    let atr14 = s.atr14.unwrap_or(s.prix_entree * 0.01);
    let trailing_stop = peak - atr14 * 1.5;

    // SL effectif progressif selon le niveau TP atteint (break-even)
    let sl_effectif = match (s.target2, s.target3) {
        (Some(tp2), Some(tp3)) if peak >= tp3 => {
            // TP3 en route : trailing stop
            return if prix <= trailing_stop { Some("TP3") } else { None };
        }
        (Some(tp2), _) if peak >= tp2 => s.target,       // BE = TP1
        _ if peak >= s.target          => s.prix_entree,  // BE = entrée
        _                              => s.stop_loss,    // SL original
    };

    if prix <= sl_effectif {
        return Some("invalide");
    }
    // TP2 : fermeture immédiate si prix >= TP2 et pas encore en zone TP3
    if let Some(tp2) = s.target2 {
        if prix >= tp2 {
            return Some("TP2");
        }
    }
    // TP1 : fermeture si prix >= TP1 et pas encore de TP2
    if prix >= s.target && s.target2.map_or(true, |tp2| peak < tp2) {
        return Some("TP1");
    }
    None
}
