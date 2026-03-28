use actix_web::{web, HttpResponse, Responder};
use db::rockets;
use std::time::Duration;

use crate::state::AppState;

pub use strategies::rockets_indicateurs::calculer_verdict_rocket;

// ── Helpers HTTP ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

pub async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

// ── POST /api/rockets/sync ───────────────────────────────────────────────────

/// Force un cycle de suivi immédiat (SL/TP attente + ouverts)
pub async fn sync_verdicts(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("HTTP client: {}", e) }))
        }
    };

    let mut fermes = 0u32;
    let mut ouverts_nouveaux = 0u32;

    // Attente : SL avant entrée, ou ouvrir si prix atteint
    if let Ok(en_attente) = rockets::lister_en_attente(pool).await {
        for s in &en_attente {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            if prix <= s.stop_loss
                && rockets::maj_verdict(pool, s.id, "invalide", prix)
                    .await
                    .is_ok()
            {
                fermes += 1;
            } else if prix >= s.prix_entree && rockets::entrer_position(pool, s.id).await.is_ok() {
                ouverts_nouveaux += 1;
            }
        }
    }

    // Ouverts : SL/TP
    if let Ok(signaux) = rockets::lister_ouverts(pool).await {
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            let peak = s.prix_peak.unwrap_or(s.prix_entree).max(prix);
            if peak > s.prix_peak.unwrap_or(0.0) {
                let _ = rockets::maj_prix_peak(pool, s.id, peak).await;
            }
            if let Some(v) = calculer_verdict_rocket(s, prix, peak) {
                if rockets::maj_verdict(pool, s.id, v, prix).await.is_ok() {
                    fermes += 1;
                }
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "fermes": fermes,
        "ouverts_nouveaux": ouverts_nouveaux
    }))
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

/// Worker lancé au démarrage : toutes les 3min, gère cycle de vie complet.
pub async fn demarrer_worker_suivi(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Worker rockets HTTP: {}", e);
            return;
        }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(3 * 60)).await;

        // 1. Expirer les signaux EN ATTENTE depuis >6h (position jamais ouverte)
        if let Ok(n) = rockets::marquer_expires(&pool).await {
            if n > 0 {
                tracing::info!("Rockets: {} signal(s) expirés (jamais entrés)", n);
            }
        }

        // 2. Signaux en attente : SL touché avant entrée → invalide, sinon ouvrir si prix atteint
        let en_attente = match rockets::lister_en_attente(&pool).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Worker rockets attente: {}", e);
                continue;
            }
        };
        for s in &en_attente {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            if prix <= s.stop_loss {
                if let Err(e) = rockets::maj_verdict(&pool, s.id, "invalide", prix).await {
                    tracing::warn!("Rocket {} SL avant entrée: {}", s.ticker, e);
                } else {
                    tracing::info!(
                        "Rocket {} → invalide (SL avant entrée) @ {:.5}",
                        s.ticker,
                        prix
                    );
                }
            } else if prix >= s.prix_entree {
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
            Err(e) => {
                tracing::warn!("Worker rockets ouverts: {}", e);
                continue;
            }
        };
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };

            let peak = s.prix_peak.unwrap_or(s.prix_entree).max(prix);
            if peak > s.prix_peak.unwrap_or(0.0) {
                if let Err(e) = rockets::maj_prix_peak(&pool, s.id, peak).await {
                    tracing::warn!("Rocket {} maj peak: {}", s.ticker, e);
                }
            }

            let verdict = calculer_verdict_rocket(s, prix, peak);
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

#[cfg(test)]
#[path = "rockets_suivi_tests.rs"]
mod tests;
