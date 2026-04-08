use actix_web::{web, HttpResponse, Responder};
use db::rockets;
use db::rockets_feedback;
use db::rockets_config;
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
            let peak_precedent = s.prix_peak.unwrap_or(s.prix_entree);
            let peak = peak_precedent.max(prix);
            if peak > peak_precedent {
                let _ = rockets::maj_prix_peak(pool, s.id, peak).await;
            }
            match calculer_verdict_rocket(s, prix, peak, peak_precedent) {
                Some(v @ "TP1") | Some(v @ "TP2") => {
                    // Vente partielle ⅓ — position reste ouverte
                    let _ = rockets::enregistrer_tp_partiel(pool, s.id, v, prix).await;
                }
                Some(v) => {
                    if rockets::maj_verdict(pool, s.id, v, prix).await.is_ok() {
                        fermes += 1;
                    }
                }
                None => {}
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
                    reconcilier_feedback(
                        &pool,
                        &s.ticker,
                        s.id,
                        "invalide",
                        s.prix_entree,
                        prix,
                        s.atr14,
                        &s.cree_le,
                    )
                    .await;
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
        let config = rockets_config::lire_config(&pool).await;
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

            let peak_precedent = s.prix_peak.unwrap_or(s.prix_entree);
            let peak = peak_precedent.max(prix);
            if peak > peak_precedent {
                if let Err(e) = rockets::maj_prix_peak(&pool, s.id, peak).await {
                    tracing::warn!("Rocket {} maj peak: {}", s.ticker, e);
                }
            }

            match calculer_verdict_rocket(s, prix, peak, peak_precedent) {
                Some(v @ "TP1") | Some(v @ "TP2") => {
                    if config.vente_partielle {
                        // Option 1 : vente partielle ⅓ — position reste ouverte
                        if let Err(e) = rockets::enregistrer_tp_partiel(&pool, s.id, v, prix).await {
                            tracing::warn!("Rocket {} tp partiel: {}", s.ticker, e);
                        } else {
                            tracing::info!("Rocket {} → {} partiel @ {:.5}", s.ticker, v, prix);
                        }
                    } else {
                        // Option 2 : pas de vente — SL progresse via peak, on logue seulement
                        tracing::info!("Rocket {} → {} (SL progresse, Option 2) @ {:.5}", s.ticker, v, prix);
                    }
                }
                Some(v) => {
                    if let Err(e) = rockets::maj_verdict(&pool, s.id, v, prix).await {
                        tracing::warn!("Worker rockets verdict: {}", e);
                    } else {
                        tracing::info!("Rocket {} → {} @ {:.5}", s.ticker, v, prix);
                        reconcilier_feedback(
                            &pool,
                            &s.ticker,
                            s.id,
                            v,
                            s.prix_entree,
                            prix,
                            s.atr14,
                            &s.cree_le,
                        )
                        .await;
                    }
                }
                None => {}
            }
        }
    }
}

// ── Helper feedback ──────────────────────────────────────────────────────────

/// Réconcilie le feedback Rockets après une clôture TP/SL.
/// `cree_le_str` est au format SQLite `datetime('now')` → "2026-04-06 14:32:00".
#[allow(clippy::too_many_arguments)]
async fn reconcilier_feedback(
    pool: &sqlx::SqlitePool,
    ticker: &str,
    signal_id: i64,
    verdict: &str,
    prix_entree: f64,
    prix_verdict: f64,
    atr14: Option<f64>,
    cree_le_str: &str,
) {
    let timestamp_signal = chrono::NaiveDateTime::parse_from_str(cree_le_str, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp());

    let atr = atr14.unwrap_or(1.0).max(1e-9);
    if let Err(e) = rockets_feedback::maj_feedback_verdict(
        pool,
        signal_id,
        verdict,
        prix_entree,
        prix_verdict,
        atr,
        timestamp_signal,
    )
    .await
    {
        tracing::warn!("Feedback Rockets {} id={}: {}", ticker, signal_id, e);
    }
}

#[cfg(test)]
#[path = "rockets_suivi_tests.rs"]
mod tests;
