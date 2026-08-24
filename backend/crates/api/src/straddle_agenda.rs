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
    actifs: Vec<&'static str>,
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
    let annonces = match state.db.lire_calendrier_cache(6 * 3600).await {
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
                    actifs: vec!["XAUUSD", "BTC"],
                })
            })
            .take(6)
            .collect(),
        Err(_) => Vec::new(),
    };

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
