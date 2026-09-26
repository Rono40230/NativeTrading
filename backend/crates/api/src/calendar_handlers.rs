use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Datelike, Duration, Utc};
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
    let client = &*crate::http_client::HTTP_CLIENT;

    let urls = [
        "https://nfs.faireconomy.media/ff_calendar_thisweek.json",
        "https://nfs.faireconomy.media/ff_calendar_nextweek.json",
    ];

    let maintenant = Utc::now();
    let seuil_inclusion = maintenant - Duration::hours(12);
    let mut toutes: Vec<serde_json::Value> = Vec::new();

    for url in &urls {
        let resp = match client
            .get(*url)
            .header(reqwest::header::USER_AGENT, "NativeTrading/1.0")
            .send()
            .await
        {
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

// ─── Job de synchronisation : échéances hebdo, JAMAIS au redémarrage ────────
// (décision owner 26/09 : les annonces sont planifiées à l'avance — 2 appels/
// semaine suffisent, lundi + jeudi 01:00 UTC, pour couvrir le roulement des
// fichiers ForexFactory cette-semaine/semaine-prochaine. Le ban 429 du 25/09
// venait du cycle 30 min + du fetch à chaque relance d'app.)

const CLE_DERNIER_FETCH: &str = "calendrier_dernier_fetch";
const HORIZON_MIN_HEURES: i64 = 48;
const RETRY_APRES_ECHEC_HEURES: i64 = 3;
const TTL_CACHE_SEC: i64 = 30 * 86_400;

/// Prochaine échéance planifiée : lundi 01:00 UTC ou jeudi 01:00 UTC après
/// `depuis_ts` (les jours UTC s'alignent sur l'epoch — divisions entières).
fn prochaine_echeance(depuis_ts: i64) -> i64 {
    let jour = depuis_ts - depuis_ts.rem_euclid(86_400);
    for j in 0..9 {
        let candidat = jour + j * 86_400 + 3_600;
        if candidat <= depuis_ts {
            continue;
        }
        let lundi_ou_jeudi = chrono::TimeZone::timestamp_opt(&Utc, candidat, 0)
            .single()
            .map(|d| matches!(d.weekday(), chrono::Weekday::Mon | chrono::Weekday::Thu))
            .unwrap_or(false);
        if lundi_ou_jeudi {
            return candidat;
        }
    }
    depuis_ts + 7 * 86_400
}

/// Le plus lointain événement futur du cache (timestamp), si connu.
async fn horizon_cache(db: &db::Database) -> Option<i64> {
    let cache = db.lire_calendrier_cache(TTL_CACHE_SEC).await.ok()?;
    cache
        .iter()
        .filter_map(|a| a["date_heure"].as_str())
        .filter_map(|s| s.parse::<DateTime<Utc>>().ok())
        .map(|d| d.timestamp())
        .filter(|t| *t > Utc::now().timestamp())
        .max()
}

/// Date/heure ISO du dernier fetch tenté (pour l'état honnête de la source).
async fn dernier_fetch_iso(db: &db::Database) -> Option<String> {
    db.lire_config(CLE_DERNIER_FETCH)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|ts| chrono::TimeZone::timestamp_opt(&Utc, ts, 0).single())
        .map(|d| d.to_rfc3339())
}

pub fn demarrer_refresh_calendrier_job(db: Arc<db::Database>) {
    tokio::spawn(async move {
        loop {
            let maintenant = Utc::now().timestamp();
            let dernier = db
                .lire_config(CLE_DERNIER_FETCH)
                .await
                .ok()
                .flatten()
                .and_then(|s| s.parse::<i64>().ok());
            let horizon = horizon_cache(db.as_ref()).await;

            // Dû ? : jamais fetché, OU une échéance lundi/jeudi est passée
            // depuis le dernier fetch, OU l'avenir couvert fond sous 48 h.
            // Un simple redémarrage ne déclenche RIEN : l'horodatage tranche.
            let echeance_passee = dernier
                .map(|d| prochaine_echeance(d) <= maintenant)
                .unwrap_or(true);
            let horizon_court = horizon
                .map(|h| h < maintenant + HORIZON_MIN_HEURES * 3_600)
                .unwrap_or(true);

            // Échec récent (ban 429) : attendre la fin de la fenêtre de
            // reprise, même si l'horizon est court — un redémarrage en
            // rafale ne doit JAMAIS provoquer de fetch (exigence owner).
            let echec_recent = dernier
                .map(|d| maintenant < d + RETRY_APRES_ECHEC_HEURES * 3_600)
                .unwrap_or(false);

            let sommeil_jusqu_a = if echec_recent {
                dernier.map(|d| d + RETRY_APRES_ECHEC_HEURES * 3_600).unwrap_or(maintenant)
            } else if echeance_passee || horizon_court {
                let n = rafraichir_calendrier(db.as_ref()).await.len();
                // Horodater la TENTATIVE (réussie ou non) : les reprises se
                // font par le sommeil ci-dessous, jamais en martelant.
                let _ = db
                    .ecrire_config(CLE_DERNIER_FETCH, &maintenant.to_string())
                    .await;
                if n > 0 {
                    prochaine_echeance(Utc::now().timestamp())
                } else {
                    // Échec (ex. ban 429) : réessayer dans 3 h jusqu'à réussite.
                    Utc::now().timestamp() + RETRY_APRES_ECHEC_HEURES * 3_600
                }
            } else {
                // Dormir jusqu'à la prochaine échéance, OU jusqu'à ce que
                // l'horizon restant passe sous 48 h (garde-fou).
                let e = prochaine_echeance(maintenant);
                horizon
                    .map(|h| e.min(h - (HORIZON_MIN_HEURES - 1) * 3_600))
                    .unwrap_or(e)
            };

            let secondes = (sommeil_jusqu_a - Utc::now().timestamp())
                .clamp(60, 7 * 86_400) as u64;
            tokio::time::sleep(std::time::Duration::from_secs(secondes)).await;
        }
    });
}

/// GET /api/calendar/etat — l'état honnête de la source (badge + modale du
/// bandeau : distinguer « aucune annonce prévue » de « source injoignable »).
pub async fn get_calendar_etat(state: web::Data<AppState>) -> impl Responder {
    let nb_futurs = state
        .db
        .lire_calendrier_cache(TTL_CACHE_SEC)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|a| {
            a["date_heure"]
                .as_str()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                .map(|d| d.timestamp() > Utc::now().timestamp())
                .unwrap_or(false)
        })
        .count();
    HttpResponse::Ok().json(serde_json::json!({
        "dernier_fetch": dernier_fetch_iso(&state.db).await,
        "nb_futurs": nb_futurs,
        "source_prete": nb_futurs > 0,
    }))
}

/// GET /api/calendar?days=3
/// Retourne les annonces économiques High/Medium dans les prochains `days`
/// jours. Ne fetch JAMAIS en ligne (décision owner 26/09) : le cache SQLite
/// est servi tel quel — la synchronisation vit dans le job lundi/jeudi, la
/// réponse signale honnêtement quand la source est à sec.
pub async fn get_calendar(
    state: web::Data<AppState>,
    query: web::Query<CalendarQuery>,
) -> impl Responder {
    let jours = query.days.unwrap_or(7).clamp(1, 14);

    // Fenêtre : événements des 12 dernières heures jusqu'à `jours` jours.
    let debut = (Utc::now() - Duration::hours(12)).timestamp();
    let limite = (Utc::now() + Duration::days(jours)).timestamp();

    let filtrees: Vec<serde_json::Value> = state
        .db
        .lire_calendrier_cache(TTL_CACHE_SEC)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|a| {
            let ts = a["date_heure"]
                .as_str()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                .map(|d| d.timestamp())
                .unwrap_or(0);
            ts >= debut && ts <= limite
        })
        .collect();

    if filtrees.is_empty() {
        return HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": "Source calendrier indisponible",
            "derniere_synchro": dernier_fetch_iso(&state.db).await,
        }));
    }
    HttpResponse::Ok().json(filtrees)
}
