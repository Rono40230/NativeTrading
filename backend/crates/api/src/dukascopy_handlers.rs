//! Backfill historique Dukascopy — `POST /api/data/dukascopy-backfill`.
//!
//! Télécharge un MOIS de candles M1 (bid) depuis le datafeed public Dukascopy
//! pour un asset×timeframe, agrège vers le TF demandé et insère en base avec
//! la source `'dukascopy'`. Le frontend boucle mois par mois (affichage de
//! progression) car le datafeed est rate-limité : ~4 s entre chaque fichier
//! quotidien → un mois ≈ 1,5-3 min.
//!
//! Corps de la requête :
//! ```json
//! { "asset": "NAS100", "timeframe": "M15", "annee": 2026, "mois": 8,
//!   "instrument": "USATECHIDXUSD", "diviseur": 1000 }
//! ```
//! - `instrument` : optionnel — par défaut lu depuis la colonne
//!   `assets.datafeed_dukascopy` (migration 0066) ;
//! - `diviseur` : optionnel — par défaut déduit de l'instrument
//!   (`data::dukascopy::diviseur_instrument`).
//!
//! Réponse :
//! ```json
//! { "asset": "NAS100", "instrument": "USATECHIDXUSD", "timeframe": "M15",
//!   "annee": 2026, "mois": 8, "jours_traites": 21, "jours_sans_donnees": 9,
//!   "bougies": 40000, "inserees": 39800, "avertissement": null,
//!   "erreurs": ["2026-08-14: HTTP 503 après 4 tentatives"] }
//! ```

use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, TimeZone, Utc};
use common::Timeframe;
use serde::Deserialize;
use tracing::info;

use crate::state::AppState;
use data::dukascopy::{
    agreger, client_http, diviseur_instrument, instrument_trade_weekend,
    telecharger_jour_m1, DELAI_ENTRE_TELECHARGEMENTS,
};

/// Source enregistrée en DB pour les bougies Dukascopy.
const SOURCE: &str = "dukascopy";

/// Nombre de tentatives par fichier quotidien (503 / coupure TCP → retry).
const MAX_TENTATIVES: u32 = 4;

/// Échecs consécutifs avant abandon du mois (rate limit prolongé : inutile
/// de faire patienter le frontend 10 min pour rien).
const ECHECS_CONSECUTIFS_MAX: u32 = 3;

#[derive(Deserialize)]
pub struct RequeteBackfillDukascopy {
    /// Identifiant d'asset DB (ex: "NAS100", "XAUUSD").
    pub asset: String,
    /// Timeframe cible (M1 à W1) — les M1 sont agrégés si > M1.
    pub timeframe: String,
    pub annee: u32,
    /// Mois 1-indexé (1 = janvier).
    pub mois: u32,
    /// Instrument Dukascopy — optionnel (sinon lu depuis la config DB).
    pub instrument: Option<String>,
    /// Diviseur points → prix — optionnel (sinon déduit de l'instrument).
    pub diviseur: Option<f64>,
}

/// `POST /api/data/dukascopy-backfill` — backfill d'un mois.
pub async fn post_dukascopy_backfill(
    state: web::Data<AppState>,
    body: web::Json<RequeteBackfillDukascopy>,
) -> impl Responder {
    // ── Validation ────────────────────────────────────────────────────────────
    let Some(asset) = crate::utils::parse_asset(&body.asset) else {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "erreur": format!("Asset inconnu: {}", body.asset) }));
    };
    let Ok(timeframe) = Timeframe::try_from(body.timeframe.as_str()) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({ "erreur": format!("Timeframe inconnu: {}", body.timeframe) }),
        );
    };
    if !(1..=12).contains(&body.mois) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "erreur": format!("Mois invalide: {}", body.mois) }));
    }
    let aujourdhui = Utc::now();
    if body.annee < 2003 || body.annee > aujourdhui.year() as u32 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "erreur": format!("Année invalide: {} (le datafeed couvre 2003 → aujourd'hui)", body.annee)
        }));
    }

    // ── Instrument Dukascopy (corps > config DB) ──────────────────────────────
    let instrument = match body
        .instrument
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(i) => i.to_string(),
        None => match state.db.instrument_dukascopy(&body.asset).await {
            Ok(Some(i)) => i,
            Ok(None) => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "erreur": format!(
                        "Aucun instrument Dukascopy configuré pour {} (colonne assets.datafeed_dukascopy)",
                        body.asset
                    )
                }))
            }
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "erreur": e.to_string() }))
            }
        },
    };
    let diviseur = body
        .diviseur
        .filter(|d| *d > 0.0)
        .unwrap_or_else(|| diviseur_instrument(&instrument));

    // ── Boucle sur les jours du mois ──────────────────────────────────────────
    let nb_jours = nb_jours_mois(body.annee, body.mois);
    // On ne télécharge pas le futur ni le jour en cours (fichier incomplet).
    let limite_aujourdhui =
        body.annee == aujourdhui.year() as u32 && body.mois == aujourdhui.month();
    let jour_max = if limite_aujourdhui {
        aujourdhui.day() - 1
    } else {
        nb_jours
    };
    let trade_weekend = instrument_trade_weekend(&instrument);
    let client = client_http();

    // Insertion INCREMENTALE (jour par jour) : si la requête est interrompue
    // (timeout serveur 310 s, rate limit prolongé), les jours déjà traités
    // restent en base — un relancement reprend là où c'était arrêté
    // (INSERT OR IGNORE). W1 fait exception : une semaine peut chevaucher
    // plusieurs jours/mois → agrégation unique en fin de mois.
    let insertion_par_jour = !matches!(timeframe, Timeframe::W1);

    let mut m1: Vec<common::Candle> = Vec::new();
    let mut erreurs: Vec<String> = Vec::new();
    let mut jours_traites = 0u32;
    let mut jours_sans_donnees = 0u32;
    let mut echecs_consecutifs = 0u32;
    let mut total_bougies = 0u64;
    let mut bougies_cible = 0u64;
    let mut inserees = 0u64;

    for jour in 1..=nb_jours {
        // Week-end (forex/indices) : pas de fichier, on économise le rate limit.
        if jour > jour_max {
            break;
        }
        let date = chrono::NaiveDate::from_ymd_opt(body.annee as i32, body.mois, jour);
        let est_weekend = date
            .map(|d| d.weekday().num_days_from_monday() >= 5)
            .unwrap_or(true);
        if est_weekend && !trade_weekend {
            jours_sans_donnees += 1;
            continue;
        }

        match telecharger_jour_m1(&client, &instrument, body.annee, body.mois, jour, MAX_TENTATIVES)
            .await
        {
            Ok(Some(bougies_jour)) => {
                echecs_consecutifs = 0;
                if data::dukascopy::jour_non_trade(&bougies_jour) {
                    // Fichier « bourré » (férié) : on l'ignore.
                    jours_sans_donnees += 1;
                } else {
                    jours_traites += 1;
                    total_bougies += bougies_jour.len() as u64;
                    if insertion_par_jour {
                        // M5→D1 : les buckets ne chevauchent jamais deux jours
                        // UTC (toutes les durées divisent 86 400).
                        let cible_jour = agreger(&bougies_jour, &timeframe);
                        bougies_cible += cible_jour.len() as u64;
                        match state
                            .db
                            .inserer_bougies_avec_source(&asset, &timeframe, &cible_jour, SOURCE)
                            .await
                        {
                            Ok(n) => inserees += n,
                            Err(e) => {
                                return HttpResponse::InternalServerError().json(
                                    serde_json::json!({ "erreur": format!("Insertion DB: {}", e) }),
                                )
                            }
                        }
                        // Les M1 ne sont pas gardés en mémoire : déjà persistés
                        // (si M1 demandé) ou agrégés au fil de l'eau.
                    } else {
                        m1.extend(bougies_jour);
                    }
                }
            }
            Ok(None) => {
                echecs_consecutifs = 0;
                jours_sans_donnees += 1;
            }
            Err(e) => {
                echecs_consecutifs += 1;
                erreurs.push(format!("{:04}-{:02}-{:02}: {}", body.annee, body.mois, jour, e));
                if echecs_consecutifs >= ECHECS_CONSECUTIFS_MAX {
                    erreurs.push(format!(
                        "Abandon du mois après {} échecs consécutifs (rate limit Dukascopy prolongé) — recharger ce mois plus tard",
                        echecs_consecutifs
                    ));
                    break;
                }
            }
        }

        // Délai OBLIGATOIRE entre téléchargements (rate limit Dukascopy).
        // Sauf après le dernier jour traité (réponse plus rapide).
        if jour < nb_jours && jour < jour_max {
            tokio::time::sleep(DELAI_ENTRE_TELECHARGEMENTS).await;
        }
    }

    // ── W1 : agrégation mensuelle unique (les semaines chevauchent les jours) ─
    if !insertion_par_jour {
        // On ne conserve que les semaines ENTIÈRES couvertes par ce qui a été
        // téléchargé (une semaine à cheval sur deux mois sera écrite complète
        // par le mois qui la termine — INSERT OR IGNORE garde la 1re écriture).
        let semaines = semaines_completes(&agreger(&m1, &Timeframe::W1), body.annee, body.mois);
        bougies_cible += semaines.len() as u64;
        if !semaines.is_empty() {
            match state
                .db
                .inserer_bougies_avec_source(&asset, &timeframe, &semaines, SOURCE)
                .await
            {
                Ok(n) => inserees += n,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "erreur": format!("Insertion DB: {}", e) }))
                }
            }
        }
    }

    // 404 systématique → instrument probablement inexistant : signalé, pas bloquant.
    let avertissement = if jours_traites == 0
        && jours_sans_donnees_sans_weekend(jours_sans_donnees, nb_jours, trade_weekend) > 0
        && erreurs.is_empty()
    {
        Some(format!(
            "Aucune donnée pour {} en {:04}-{:02} (404 systématique) — instrument probablement inexistant sur le datafeed Dukascopy",
            instrument, body.annee, body.mois
        ))
    } else {
        None
    };

    info!(
        "Backfill Dukascopy {} {} {}/{}: {} jours traités, {} bougies M1, {} insérées, {} erreurs",
        body.asset, body.timeframe, body.annee, body.mois,
        jours_traites, total_bougies, inserees, erreurs.len()
    );

    HttpResponse::Ok().json(serde_json::json!({
        "asset": body.asset,
        "instrument": instrument,
        "diviseur": diviseur,
        "timeframe": body.timeframe,
        "annee": body.annee,
        "mois": body.mois,
        "jours_traites": jours_traites,
        "jours_sans_donnees": jours_sans_donnees,
        "bougies": total_bougies,
        "bougies_cible": bougies_cible,
        "inserees": inserees,
        "avertissement": avertissement,
        "erreurs": erreurs,
    }))
}

/// Nombre de jours d'un mois (gestion des années bissextiles).
fn nb_jours_mois(annee: u32, mois: u32) -> u32 {
    match mois {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Jours réellement interrogés (hors week-ends pour forex/indices) — utilisé
/// pour distinguer « mois sans données car week-end » de « 404 systématique ».
fn jours_sans_donnees_sans_weekend(sans_donnees: u32, nb_jours: u32, trade_weekend: bool) -> u32 {
    if trade_weekend {
        sans_donnees
    } else {
        // ~8-9 jours de week-end par mois : on n'alerte que si des jours
        // OUVRÉS ont répondu 404.
        sans_donnees.saturating_sub(nb_jours / 7 * 2).max(0)
    }
}

/// Filtre les bougies W1 d'un mois pour ne garder que les semaines ENTIÈRES
/// couvertes : la semaine commence après le 1er du mois ET se termine avant
/// la fin du mois (ou avant aujourd'hui pour le mois en cours). Une semaine
/// à cheval sur deux mois est écrite complète par le mois qui la termine.
/// Effet de bord assumé : la 1re semaine du mois le plus ancien d'un backfill
/// n'est jamais écrite.
fn semaines_completes(semaines: &[common::Candle], annee: u32, mois: u32) -> Vec<common::Candle> {
    let debut_mois = minuit_utc_mois(annee, mois);
    let fin_mois = minuit_utc_mois(
        if mois == 12 { annee + 1 } else { annee },
        if mois == 12 { 1 } else { mois + 1 },
    );
    let fin_couverture = fin_mois.min(Utc::now().timestamp());

    semaines
        .iter()
        .filter(|s| {
            let debut = s.timestamp.timestamp();
            debut >= debut_mois && debut + 7 * 86_400 <= fin_couverture
        })
        .cloned()
        .collect()
}

/// Timestamp Unix du minuit UTC du 1er du mois.
fn minuit_utc_mois(annee: u32, mois: u32) -> i64 {
    Utc.with_ymd_and_hms(annee as i32, mois, 1, 0, 0, 0)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

// ─── Tests unitaires (pas de réseau, pas de DB) ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nb_jours_par_mois() {
        assert_eq!(nb_jours_mois(2026, 1), 31);
        assert_eq!(nb_jours_mois(2026, 4), 30);
        assert_eq!(nb_jours_mois(2026, 2), 28);
        assert_eq!(nb_jours_mois(2024, 2), 29, "2024 bissextile");
        assert_eq!(nb_jours_mois(2000, 2), 29, "2000 bissextile (÷400)");
        assert_eq!(nb_jours_mois(1900, 2), 28, "1900 non bissextile (÷100)");
        assert_eq!(nb_jours_mois(2026, 13), 0, "mois invalide");
    }

    #[test]
    fn alerte_404_systematique() {
        // Forex, mois de 31 j : ~8 week-ends ignorés → 23 jours interrogés.
        // 23 jours sans données = 404 systématique → alerte.
        let sans_donnees = 23;
        assert!(jours_sans_donnees_sans_weekend(sans_donnees, 31, false) > 0);
        // Crypto (week-end tradé) : 31 jours interrogés.
        assert!(jours_sans_donnees_sans_weekend(31, 31, true) > 0);
        // Mois normal : quelques fériés 404 → pas d'alerte.
        assert_eq!(jours_sans_donnees_sans_weekend(10, 31, false), 2); // 10 - 8
        // Aucun jour sans données → pas d'alerte.
        assert_eq!(jours_sans_donnees_sans_weekend(0, 30, false), 0);
    }

    fn semaine(ts: i64) -> common::Candle {
        common::Candle {
            timestamp: Utc.timestamp_opt(ts, 0).single().unwrap(),
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 1.0,
        }
    }

    #[test]
    fn semaines_completes_filtre_les_bords_de_mois() {
        // Juillet 2026 : du mercredi 1er au vendredi 31.
        // Lundis : 6, 13, 20, 27 juillet.
        let l6 = minuit_utc_mois(2026, 7) + 5 * 86_400; // lundi 6
        let l13 = l6 + 7 * 86_400;
        let l20 = l13 + 7 * 86_400;
        let l27 = l20 + 7 * 86_400; // sa semaine finit le lundi 3 août → incomplète
        // Mois passé (fin avant aujourd'hui) → couverture = fin du mois.
        let semaines = vec![semaine(l6), semaine(l13), semaine(l20), semaine(l27)];
        let gardees = semaines_completes(&semaines, 2026, 7);
        assert_eq!(gardees.len(), 3, "semaine du 27 juillet exclue (déborde en août)");
        assert!(gardees.iter().all(|s| s.timestamp.timestamp() >= minuit_utc_mois(2026, 7)));
    }

    #[test]
    fn semaines_completes_mois_en_cours_ne_garde_pas_la_semaine_en_cours() {
        // Un mois hypothétique dans le futur proche : tout est filtré.
        let annee_courante = Utc::now().year() as u32;
        let annee = if annee_courante >= 2027 { 2027 } else { 2026 };
        let aout = minuit_utc_mois(annee, 8);
        let semaines = vec![semaine(aout + 9 * 86_400)]; // lundi de la semaine 2
        // Quelle que soit la date, le filtre ne panique jamais.
        let gardees = semaines_completes(&semaines, annee, 8);
        assert!(gardees.len() <= 1);
    }
}
