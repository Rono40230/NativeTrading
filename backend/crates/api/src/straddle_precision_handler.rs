use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike};
use crate::state::AppState;

// ── POST /api/straddle/precision-horaire ─────────────────────────────────────
/// Calcule la précision à la minute pour un créneau horaire donné (sans créneau DB requis).
/// Utilisé par la heatmap historique pour le drill-down interactif.

#[derive(serde::Deserialize)]
pub struct RequetePrecisionHoraire {
    pub asset: String,
    /// Heure UTC (0–23)
    pub heure: u8,
    /// 0=Lundi … 6=Dimanche, None=tous les jours
    pub jour_semaine: Option<i64>,
}

/// Session de marché active à une heure UTC donnée.
fn session_active(heure_utc: u8) -> &'static str {
    match heure_utc {
        0..=2   => "Session Asie (Tokyo)",
        3..=7   => "Pré-ouverture Europe",
        8..=11  => "Session Londres",
        12..=16 => "Overlap Londres – New York",
        17..=21 => "Session New York",
        _       => "Session hors marché",
    }
}

/// Raison macro probable basée sur les horaires UTC récurrents.
/// 0=Lun, 1=Mar, 2=Mer, 3=Jeu, 4=Ven (chrono::num_days_from_monday)
fn raison_probable(heure_utc: u8, jour_semaine: Option<i64>) -> Option<&'static str> {
    match (heure_utc, jour_semaine) {
        (13, Some(4)) => Some("NFP — Rapport emploi US (vendredi 13h30 UTC)"),
        (13, _) | (14, _) => Some("Publications macro USD — Ouverture New York (13h30 UTC)"),
        (8, _) | (9, _)   => Some("Ouverture Londres (08h–09h UTC)"),
        (7, _)            => Some("Données économiques européennes (BCE / PMI)"),
        (18, Some(2)) | (19, Some(2)) => Some("FOMC — Réunion Fed (mercredi soir)"),
        (15, _) | (16, _) => Some("Mi-session New York — volumes élevés"),
        (21, _)           => Some("Clôture New York"),
        _                 => None,
    }
}

pub async fn handler_precision_horaire(
    state: web::Data<AppState>,
    body: web::Json<RequetePrecisionHoraire>,
) -> impl Responder {
    let asset = match crate::utils::parse_asset(&body.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset inconnu" }))
        }
    };

    let heure_debut = format!("{:02}:00", body.heure);
    let heure_fin = format!("{:02}:00", (body.heure + 1) % 24);

    let bougies = match state
        .db
        .obtenir_bougies_plage_horaire_m1(&asset, &heure_debut, &heure_fin)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let precision = strategies::straddle_precision::analyser_precision(
        &bougies,
        body.jour_semaine,
        &heure_debut,
        &heure_fin,
    );

    let session = session_active(body.heure);

    match precision {
        None => HttpResponse::Ok()
            .json(serde_json::json!({ "ok": false, "message": "Données insuffisantes" })),
        Some(p) => {
            // Heure UTC du pic (parse "HH:MM")
            let heure_pic: u8 = p.timing_optimal
                .split(':').next()
                .and_then(|h| h.parse().ok())
                .unwrap_or(body.heure);

            // Corrélation avec le calendrier économique (best effort)
            let raison_calendrier = match state.db.lire_calendrier_cache(30 * 24 * 3600).await {
                Ok(events) => events.into_iter().find_map(|ev| {
                    let date_str = ev["date_heure"].as_str()?.to_string();
                    let dt: chrono::DateTime<chrono::Utc> = date_str.parse().ok()?;
                    let ev_heure = dt.hour() as u8;
                    let ev_jour = dt.weekday().num_days_from_monday() as i64;
                    let heure_proche = (ev_heure as i16 - heure_pic as i16).unsigned_abs() <= 1;
                    let jour_ok = body.jour_semaine.map(|j| j == ev_jour).unwrap_or(true);
                    if heure_proche && jour_ok {
                        let titre = ev["titre"].as_str()?.to_string();
                        let devise = ev["devise"].as_str().unwrap_or("").to_string();
                        Some(format!("{} ({})", titre, devise))
                    } else { None }
                }),
                Err(_) => None,
            };

            let raison = raison_calendrier
                .or_else(|| raison_probable(heure_pic, body.jour_semaine).map(String::from));

            HttpResponse::Ok().json(serde_json::json!({
                "ok": true,
                "timing_optimal": p.timing_optimal,
                "fenetre_entree": p.fenetre_entree,
                "whipsaw_minutes": p.whipsaw_minutes,
                "nb_occurrences": p.nb_occurrences,
                "atr_pic": p.atr_pic,
                "session": session,
                "raison": raison,
            }))
        }
    }
}
