use actix_web::{web, HttpResponse, Responder};
use db::signaux;
use serde::Deserialize;
use std::time::Duration;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct QuerySignaux {
    pub limit: Option<i64>,
}

/// GET /api/signaux?limit=N — historique avec verdict inclus
pub async fn get_signaux(
    state: web::Data<AppState>,
    query: web::Query<QuerySignaux>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(500);
    match state.db.obtenir_signaux(limit).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique signaux: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

fn calculer_verdict(
    direction: &str,
    stop_loss: f64,
    take_profit: &[f64],
    prix: f64,
) -> Option<&'static str> {
    let long = direction.to_uppercase().contains("LONG");
    if long {
        if prix <= stop_loss {
            return Some("SL");
        }
        if take_profit.get(2).is_some_and(|&t| prix >= t) {
            return Some("TP3");
        }
        if take_profit.get(1).is_some_and(|&t| prix >= t) {
            return Some("TP2");
        }
        if take_profit.first().is_some_and(|&t| prix >= t) {
            return Some("TP1");
        }
    } else {
        if prix >= stop_loss {
            return Some("SL");
        }
        if take_profit.get(2).is_some_and(|&t| prix <= t) {
            return Some("TP3");
        }
        if take_profit.get(1).is_some_and(|&t| prix <= t) {
            return Some("TP2");
        }
        if take_profit.first().is_some_and(|&t| prix <= t) {
            return Some("TP1");
        }
    }
    None
}

/// Worker lancé au démarrage : toutes les 5min, vérifie TP/SL des signaux SMC/Straddle.
pub async fn demarrer_worker_suivi_signaux(pool: sqlx::SqlitePool) {
    let client = &*crate::http_client::HTTP_CLIENT;

    loop {
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;

        if let Ok(n) = signaux::expirer_anciens(&pool).await {
            if n > 0 {
                tracing::info!("Signaux: {} expiré(s)", n);
            }
        }

        let actifs = match signaux::lister_actifs(&pool).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Worker signaux liste: {}", e);
                continue;
            }
        };

        for s in &actifs {
            let prix = match crate::prix_utils::fetch_prix_asset(&client, &s.asset).await {
                Some(p) => p,
                None => continue,
            };
            if let Some(v) = calculer_verdict(&s.direction, s.stop_loss, &s.take_profit, prix) {
                match signaux::maj_verdict(&pool, &s.id, v, prix).await {
                    Ok(_) => {
                        tracing::info!("Signal {} {} → {} @ {:.4}", s.asset, s.direction, v, prix)
                    }
                    Err(e) => tracing::warn!("Worker signaux verdict: {}", e),
                }
            }
        }
    }
}
