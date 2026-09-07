//! Étape 4 — agenda de la verticale Straddle : événements qui arment la
//! stratégie (annonces US fortes + ouverture européenne DAX) et passes en
//! cours, pour la section dédiée du bloc central du dashboard.

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct AnnonceAgenda {
    ts: i64,
    titre: String,
    devise: String,
    /// Assets dont le moteur straddle s'arme sur cet événement.
    actifs: Vec<String>,
}

#[derive(Serialize)]
struct PasseEnCours {
    asset: String,
    timeframe: String,
    direction: String,
    prix_entree: f64,
    cree_le: i64,
}

#[derive(Serialize)]
struct AgendaStraddle {
    annonces: Vec<AnnonceAgenda>,
    passes: Vec<PasseEnCours>,
    /// Assets du périmètre acté mais sans flux temps réel (attendent MT5).
    en_attente_mt5: Vec<&'static str>,
}

/// GET /api/straddle/agenda — prochains événements + passes en cours.
pub async fn get_agenda(state: web::Data<AppState>) -> impl Responder {
    // Annonces US High des 7 prochains jours (même source que le moteur).
    let mut annonces = match state.db.lire_calendrier_cache(6 * 3600).await {
        Ok(rows) => rows
            .iter()
            .filter_map(|r| {
                let impact = r.get("impact").and_then(|v| v.as_str()).unwrap_or("");
                if impact != "High" {
                    return None;
                }
                if r.get("devise").and_then(|v| v.as_str()).unwrap_or("") != "USD" {
                    return None;
                }
                let dh = r.get("date_heure").and_then(|v| v.as_str())?;
                let ts = chrono::DateTime::parse_from_rfc3339(dh)
                    .or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(&format!(
                            "{}:{}",
                            &dh[..dh.len() - 2],
                            &dh[dh.len() - 2..]
                        ))
                    })
                    .ok()?
                    .timestamp();
                let maintenant = chrono::Utc::now().timestamp();
                if ts <= maintenant || ts > maintenant + 7 * 24 * 3600 {
                    return None;
                }
                Some(AnnonceAgenda {
                    ts,
                    titre: r.get("titre").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    devise: "USD".into(),
                    actifs: vec!["XAUUSD".into(), "BTC".into()],
                })
            })
            .take(6)
            .collect(),
        Err(_) => Vec::new(),
    };

    // §16 (07/09) : créneaux IA armés — même liste, badge 🤖 (la prochaine
    // occurrence hebdomadaire, celle que le moteur recevra).
    {
        const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
        let rows = sqlx::query("SELECT asset, jour, heure FROM creneaux_ia WHERE arme = 1")
            .fetch_all(state.db.pool())
            .await
            .unwrap_or_default();
        use sqlx::Row as _;
        let maintenant = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris);
        for r in &rows {
            let asset: String = r.get("asset");
            let jour = r.get::<i64, _>("jour").clamp(1, 7) as u32;
            let heure = r.get::<i64, _>("heure").clamp(0, 23) as u32;
            use chrono::{Datelike, Timelike};
            for delta in 0..=14i64 {
                // Candidat à l'heure PILE du créneau (maintenant + N jours
                // garde sinon les minutes courantes — jamais 00).
                let jour_glisse = maintenant + chrono::Duration::days(delta);
                let Some(candidat) = jour_glisse
                    .with_hour(heure)
                    .and_then(|d| d.with_minute(0))
                    .and_then(|d| d.with_second(0))
                else { continue };
                if candidat.weekday().number_from_monday() != jour
                    || candidat <= maintenant
                {
                    continue;
                }
                annonces.push(AnnonceAgenda {
                    ts: candidat.timestamp(),
                    titre: format!("🤖 Créneau IA {} {}", asset, JOURS[(jour - 1) as usize]),
                    devise: if asset == "DAX" { "EUR".into() } else { "USD".into() },
                    actifs: vec![asset.clone()],
                });
                break;
            }
        }
        annonces.sort_by_key(|a| a.ts);
        annonces.truncate(8);
    }

    // Passes en cours : signaux straddle actifs (jambe survivante).
    let passes = match state.db.obtenir_signaux(50).await {
        Ok(liste) => liste
            .iter()
            .filter(|s| {
                s.get("strategie").and_then(|v| v.as_str()) == Some("straddle")
                    && s.get("statut").and_then(|v| v.as_str()) == Some("Actif")
            })
            .filter_map(|s| {
                Some(PasseEnCours {
                    asset: s.get("asset")?.as_str()?.to_string(),
                    timeframe: s.get("timeframe")?.as_str()?.to_string(),
                    direction: s.get("direction")?.as_str()?.to_string(),
                    prix_entree: s.get("prix_entree")?.as_f64()?,
                    cree_le: s.get("cree_le")?.as_i64()?,
                })
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    HttpResponse::Ok().json(AgendaStraddle {
        annonces,
        passes,
        en_attente_mt5: vec!["NAS100", "SP500", "DAX"],
    })
}
