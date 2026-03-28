use actix_web::{web, HttpResponse, Responder};
use std::time::Duration;

use crate::state::AppState;

/// GET /api/news/fear-greed
/// Retourne le Fear & Greed Index Bitcoin depuis alternative.me.
/// Cache mémoire TTL 1h — dégradation silencieuse si API indisponible.
pub async fn get_fear_greed(state: web::Data<AppState>) -> impl Responder {
    // Lecture cache (scope limité pour libérer le lock rapidement)
    {
        let cache = state.fear_greed_cache.read().await;
        if let Some((fetched_at, data)) = cache.as_ref() {
            if fetched_at.elapsed() < Duration::from_secs(3600) {
                return HttpResponse::Ok().json(data);
            }
        }
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("NativeTrading/1.0")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Fear&Greed – création client HTTP: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    let resp = match client
        .get("https://api.alternative.me/fng/?limit=1")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Fear&Greed – fetch échoué (fallback neutre): {e}");
            return HttpResponse::Ok().json(serde_json::json!({
                "valeur": 50,
                "label": "Neutral",
                "source": "fallback"
            }));
        }
    };

    let raw: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Fear&Greed – parse JSON (fallback neutre): {e}");
            return HttpResponse::Ok().json(serde_json::json!({
                "valeur": 50,
                "label": "Neutral",
                "source": "fallback"
            }));
        }
    };

    let valeur = raw["data"][0]["value"]
        .as_str()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(50);

    let label = raw["data"][0]["value_classification"]
        .as_str()
        .unwrap_or("Neutral")
        .to_string();

    let data = serde_json::json!({
        "valeur": valeur,
        "label": label,
        "categorie": categoriser(valeur),
    });

    // Mise en cache mémoire
    let mut cache = state.fear_greed_cache.write().await;
    *cache = Some((std::time::Instant::now(), data.clone()));

    // E.2 — Alimenter le contexte du Signal Engine
    state.signal_engine.mettre_a_jour_fg(valeur as i32);

    tracing::debug!("Fear&Greed mis en cache: {valeur}/100 ({label})");
    HttpResponse::Ok().json(data)
}

fn categoriser(v: u8) -> &'static str {
    match v {
        0..=24 => "extreme_fear",
        25..=49 => "fear",
        50 => "neutral",
        51..=74 => "greed",
        _ => "extreme_greed",
    }
}
