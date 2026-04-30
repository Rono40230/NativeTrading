use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CalendarQuery {
    pub days: Option<i64>,
}

/// Forme brute renvoyée par l'API ForexFactory
#[derive(Deserialize)]
struct FfEvent {
    title: String,
    country: String,
    date: String,
    impact: String,
    forecast: Option<String>,
    previous: Option<String>,
}

pub async fn rafraichir_calendrier(db: &db::Database) -> Vec<serde_json::Value> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("NativeTrading/1.0")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Création client reqwest: {}", e);
            return Vec::new();
        }
    };

    let urls = [
        "https://nfs.faireconomy.media/ff_calendar_thisweek.json",
        "https://nfs.faireconomy.media/ff_calendar_nextweek.json",
    ];

    let maintenant = Utc::now();
    let seuil_inclusion = maintenant - Duration::hours(12);
    let mut toutes: Vec<serde_json::Value> = Vec::new();

    for url in &urls {
        let resp = match client.get(*url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::debug!("ForexFactory {} → HTTP {}", url, r.status());
                continue;
            }
            Err(e) => {
                tracing::warn!("Fetch ForexFactory {}: {}", url, e);
                continue;
            }
        };
        let events: Vec<FfEvent> = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Parse ForexFactory {}: {}", url, e);
                continue;
            }
        };

        for ev in events {
            if ev.impact != "High" && ev.impact != "Medium" {
                continue;
            }
            if ev.country != "USD" && ev.country != "EUR" {
                continue;
            }
            let dt_utc: DateTime<Utc> = match DateTime::parse_from_rfc3339(&ev.date) {
                Ok(dt) => dt.into(),
                Err(_) => continue,
            };
            if dt_utc < seuil_inclusion {
                continue;
            }
            let est_passe = dt_utc < maintenant;
            let id = format!("{}-{}-{}", dt_utc.timestamp(), ev.country, ev.title.len());
            toutes.push(serde_json::json!({
                "id":        id,
                "date_heure": dt_utc.to_rfc3339(),
                "devise":    ev.country,
                "titre":     ev.title,
                "impact":    ev.impact,
                "precedent": ev.previous,
                "prevision": ev.forecast,
                "est_passe": est_passe,
            }));
        }
    }

    toutes.sort_by(|a, b| {
        a["date_heure"]
            .as_str()
            .unwrap_or("")
            .cmp(b["date_heure"].as_str().unwrap_or(""))
    });

    if !toutes.is_empty() {
        if let Err(e) = db.ecrire_calendrier_cache(&toutes).await {
            tracing::warn!("Écriture cache calendrier: {}", e);
        }
    }

    toutes
}

pub fn demarrer_refresh_calendrier_job(db: Arc<db::Database>) {
    tokio::spawn(async move {
        loop {
            let n = rafraichir_calendrier(db.as_ref()).await.len();
            tracing::debug!("Calendrier refresh job: {} événements en cache", n);
            tokio::time::sleep(std::time::Duration::from_secs(30 * 60)).await;
        }
    });
}

/// GET /api/calendar?days=3
/// Retourne les annonces économiques High/Medium dans les prochains `days` jours.
/// Cache SQLite TTL 1h pour éviter le spam vers ForexFactory.
pub async fn get_calendar(
    state: web::Data<AppState>,
    query: web::Query<CalendarQuery>,
) -> impl Responder {
    let jours = query.days.unwrap_or(7).clamp(1, 14);
    let ttl = 3600i64;

    // Fenêtre : événements des 12 dernières heures jusqu'à `jours` jours dans le futur
    let debut = Utc::now() - Duration::hours(12);

    // 1. Lecture cache SQLite
    match state.db.lire_calendrier_cache(ttl).await {
        Ok(cached) if !cached.is_empty() => {
            let limite = (Utc::now() + Duration::days(jours)).timestamp();
            let filtrees: Vec<serde_json::Value> = cached
                .into_iter()
                .filter(|a| {
                    let ts = a["date_heure"]
                        .as_str()
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                        .map(|d| d.timestamp())
                        .unwrap_or(0);
                    ts >= debut.timestamp() && ts <= limite
                })
                .collect();
            // Si des événements sont encore dans la fenêtre, servir depuis le cache
            if !filtrees.is_empty() {
                return HttpResponse::Ok().json(filtrees);
            }
            // Tous passés → re-fetch
            tracing::info!("Cache calendrier: tous événements passés, re-fetch");
        }
        Err(e) => tracing::warn!("Lecture cache calendrier: {}", e),
        _ => {}
    }

    // 2. Cache absent ou expiré → refresh ForexFactory
    let toutes = rafraichir_calendrier(state.db.as_ref()).await;
    if toutes.is_empty() {
        return HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": "Calendrier indisponible" }));
    }

    // Filtre fenêtre temporelle demandée
    let maintenant = Utc::now();
    let seuil_inclusion = maintenant - Duration::hours(12);
    let limite = (maintenant + Duration::days(jours)).timestamp();
    let filtrees: Vec<serde_json::Value> = toutes
        .into_iter()
        .filter(|a| {
            let ts = a["date_heure"]
                .as_str()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                .map(|d| d.timestamp())
                .unwrap_or(0);
            ts >= seuil_inclusion.timestamp() && ts <= limite
        })
        .collect();

    HttpResponse::Ok().json(filtrees)
}
