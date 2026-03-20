use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

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

/// GET /api/calendar?days=3
/// Retourne les annonces économiques High/Medium dans les prochains `days` jours.
/// Cache SQLite TTL 1h pour éviter le spam vers ForexFactory.
pub async fn get_calendar(
    state: web::Data<AppState>,
    query: web::Query<CalendarQuery>,
) -> impl Responder {
    let jours = query.days.unwrap_or(3).clamp(1, 14);
    let ttl = 3600i64;

    // 1. Lecture cache SQLite
    match state.db.lire_calendrier_cache(ttl).await {
        Ok(cached) if !cached.is_empty() => {
            let maintenant = Utc::now().timestamp();
            let limite = (Utc::now() + Duration::days(jours)).timestamp();
            let filtrees: Vec<serde_json::Value> = cached
                .into_iter()
                .filter(|a| {
                    let ts = a["date_heure"]
                        .as_str()
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                        .map(|d| d.timestamp())
                        .unwrap_or(0);
                    ts >= maintenant && ts <= limite
                })
                .collect();
            return HttpResponse::Ok().json(filtrees);
        }
        Err(e) => tracing::warn!("Lecture cache calendrier: {}", e),
        _ => {}
    }

    // 2. Cache absent ou expiré → fetch ForexFactory
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("NativeTrading/1.0")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Création client reqwest: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    let urls = [
        "https://nfs.faireconomy.media/ff_calendar_thisweek.json",
        "https://nfs.faireconomy.media/ff_calendar_nextweek.json",
    ];

    let maintenant = Utc::now();
    let mut toutes: Vec<serde_json::Value> = Vec::new();

    for url in &urls {
        let events: Vec<FfEvent> = match client.get(*url).send().await {
            Ok(resp) => match resp.json().await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Parse ForexFactory {}: {}", url, e);
                    continue;
                }
            },
            Err(e) => {
                tracing::warn!("Fetch ForexFactory {}: {}", url, e);
                continue;
            }
        };

        for ev in events {
            if ev.impact != "High" && ev.impact != "Medium" {
                continue;
            }
            let dt_utc: DateTime<Utc> = match DateTime::parse_from_rfc3339(&ev.date) {
                Ok(dt) => dt.into(),
                Err(_) => continue,
            };
            if dt_utc < maintenant {
                continue;
            }
            let id = format!("{}-{}-{}", dt_utc.timestamp(), ev.country, ev.title.len());
            toutes.push(serde_json::json!({
                "id":        id,
                "date_heure": dt_utc.to_rfc3339(),
                "devise":    ev.country,
                "titre":     ev.title,
                "impact":    ev.impact,
                "precedent": ev.previous,
                "prevision": ev.forecast,
            }));
        }
    }

    // Tri chronologique
    toutes.sort_by(|a, b| {
        a["date_heure"]
            .as_str()
            .unwrap_or("")
            .cmp(b["date_heure"].as_str().unwrap_or(""))
    });

    // Mise en cache SQLite
    if !toutes.is_empty() {
        if let Err(e) = state.db.ecrire_calendrier_cache(&toutes).await {
            tracing::warn!("Écriture cache calendrier: {}", e);
        }
    }

    // Filtre fenêtre temporelle demandée
    let limite = (maintenant + Duration::days(jours)).timestamp();
    let filtrees: Vec<serde_json::Value> = toutes
        .into_iter()
        .filter(|a| {
            let ts = a["date_heure"]
                .as_str()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                .map(|d| d.timestamp())
                .unwrap_or(0);
            ts >= maintenant.timestamp() && ts <= limite
        })
        .collect();

    HttpResponse::Ok().json(filtrees)
}
