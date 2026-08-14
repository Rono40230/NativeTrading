//! Import CSV générique → SQLite — `POST /api/data/import-csv`.
//!
//! Le frontend lit le fichier côté client (FileReader) et envoie le contenu
//! en JSON : `{ "csv": "...", "asset": "DAX", "timeframe": "H1" }`.
//! Choix JSON plutôt que multipart : pas de dépendance actix-multipart,
//! payload simple et fiable (option explicitement validée par le plan).
//!
//! Tolérances de parsing (aucun panic, aucune ligne ne fait échouer l'import) :
//! - délimiteur `,` ou `;` auto-détecté (compte sur la 1ère ligne) ;
//! - timestamp Unix secondes (10 chiffres), millisecondes (13 chiffres) ou
//!   date lisible (`2026-01-01 00:00:00`, `2026/01/01 00:00:00`, variantes) ;
//! - en-tête textuel ignoré si la 1ère ligne n'est pas numérique ;
//! - volume optionnel (5 colonnes acceptées, défaut 0) ;
//! - décimales à la française (`4409,66`) normalisées en point.

use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use common::Candle;
use serde::Deserialize;

use crate::state::AppState;

/// Source enregistrée en DB pour les bougies issues de cet import.
const SOURCE: &str = "csv";

#[derive(Deserialize)]
pub struct RequeteImportCsv {
    /// Contenu brut du fichier CSV.
    pub csv: String,
    /// Identifiant d'asset DB (ex: "DAX", "XAUUSD").
    pub asset: String,
    /// Timeframe DB (ex: "M5", "H1").
    pub timeframe: String,
}

// ─── Parsing (fonctions pures → testables) ────────────────────────────────────

/// Détecte le délimiteur d'une ligne : le plus fréquent entre `,` et `;`
/// (défaut `,` en cas d'égalité ou d'absence).
fn detecter_delimiteur(ligne: &str) -> char {
    if ligne.matches(';').count() > ligne.matches(',').count() {
        ';'
    } else {
        ','
    }
}

/// Parse un champ timestamp en `DateTime<Utc>` :
/// - 10 chiffres → Unix secondes ;
/// - 13 chiffres → Unix millisecondes (÷ 1000) ;
/// - contient `-` ou `/` → date lisible (formats `%Y-%m-%d` / `%Y/%m/%d`,
///   heure optionnelle, séparateur `T` accepté, date seule → minuit UTC).
fn parse_timestamp(brut: &str) -> Option<DateTime<Utc>> {
    let s = brut.trim();
    if s.is_empty() {
        return None;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return match s.len() {
            13 => s.parse::<i64>().ok().and_then(|ms| Utc.timestamp_millis_opt(ms).single()),
            10 => s.parse::<i64>().ok().and_then(|sec| Utc.timestamp_opt(sec, 0).single()),
            _ => None,
        };
    }
    if s.contains('-') || s.contains('/') {
        const FORMATS_DATE_HEURE: &[&str] = &[
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d %H:%M",
            "%Y/%m/%d %H:%M:%S",
            "%Y/%m/%d %H:%M",
        ];
        for format in FORMATS_DATE_HEURE {
            if let Ok(ndt) = NaiveDateTime::parse_from_str(s, format) {
                return Some(Utc.from_utc_datetime(&ndt));
            }
        }
        const FORMATS_DATE: &[&str] = &["%Y-%m-%d", "%Y/%m/%d"];
        for format in FORMATS_DATE {
            if let Ok(nd) = NaiveDate::parse_from_str(s, format) {
                return nd
                    .and_hms_opt(0, 0, 0)
                    .map(|ndt| Utc.from_utc_datetime(&ndt));
            }
        }
    }
    None
}

/// Normalise un nombre décimal : trim + espaces de milliers retirés +
/// virgule décimale française convertie en point (`4409,66` → `4409.66`).
fn parse_nombre(brut: &str) -> Option<f64> {
    let nettoye = brut.trim().replace(' ', "");
    let normalise = if nettoye.contains(',') && !nettoye.contains('.') {
        nettoye.replace(',', ".")
    } else {
        nettoye
    };
    normalise.parse::<f64>().ok()
}

/// `true` si le premier champ ressemble à un en-tête textuel (ex:
/// "timestamp", "Date") — ni numérique, ni date parsable.
fn est_en_tete(champ: &str) -> bool {
    let s = champ.trim();
    !s.is_empty() && parse_timestamp(s).is_none() && !s.chars().all(|c| c.is_ascii_digit())
}

/// Parse une ligne CSV `timestamp,open,high,low,close[,volume]`.
/// Volume absent → 0. Ligne inexploitable → `None` (comptée comme ignorée).
fn parse_ligne(ligne: &str, delimiteur: char) -> Option<Candle> {
    let parties: Vec<&str> = ligne.split(delimiteur).collect();
    if parties.len() < 5 {
        return None;
    }
    let timestamp = parse_timestamp(parties[0])?;
    let open = parse_nombre(parties[1])?;
    let high = parse_nombre(parties[2])?;
    let low = parse_nombre(parties[3])?;
    let close = parse_nombre(parties[4])?;
    let volume = if parties.len() >= 6 {
        parse_nombre(parties[5]).unwrap_or(0.0)
    } else {
        0.0
    };
    // Cohérence minimale : prix strictement positifs, high >= low.
    if open <= 0.0 || high <= 0.0 || low <= 0.0 || close <= 0.0 || high < low {
        return None;
    }
    Some(Candle {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

/// Parse un contenu CSV complet (BOM retiré, lignes vides ignorées, en-tête
/// détecté) → bougies exploitables + nombre de lignes ignorées.
fn parser_csv(contenu: &str) -> (Vec<Candle>, u64) {
    let contenu = contenu.trim_start_matches('\u{feff}');
    let lignes: Vec<&str> = contenu.lines().filter(|l| !l.trim().is_empty()).collect();
    let Some(premiere) = lignes.first() else {
        return (Vec::new(), 0);
    };
    let delimiteur = detecter_delimiteur(premiere);
    // Saut d'en-tête : première ligne non numérique.
    let debut_donnees = match premiere.split(delimiteur).next() {
        Some(champ) if est_en_tete(champ) => 1,
        _ => 0,
    };

    let mut bougies = Vec::with_capacity(lignes.len().saturating_sub(debut_donnees));
    let mut ignorees = 0u64;
    for ligne in &lignes[debut_donnees..] {
        match parse_ligne(ligne, delimiteur) {
            Some(c) => bougies.push(c),
            None => ignorees += 1,
        }
    }
    (bougies, ignorees)
}

// ─── POST /api/data/import-csv ────────────────────────────────────────────────

pub async fn post_import_csv(
    state: web::Data<AppState>,
    body: web::Json<RequeteImportCsv>,
) -> impl Responder {
    let Some(asset) = crate::utils::parse_asset(&body.asset) else {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "erreur": format!("Asset inconnu: {}", body.asset) }));
    };
    // Validation stricte du timeframe (TryFrom) — le helper utils::parse_timeframe
    // retombe silencieusement sur M15, inacceptable pour un import ciblé.
    let Ok(timeframe) = common::Timeframe::try_from(body.timeframe.as_str()) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({ "erreur": format!("Timeframe inconnu: {}", body.timeframe) }),
        );
    };
    if body.csv.trim().is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "erreur": "Contenu CSV vide" }));
    }

    let (bougies, lignes_ignorees) = parser_csv(&body.csv);
    let total_lues = bougies.len() as u64;
    if bougies.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "erreur": "Aucune bougie exploitable (colonnes attendues: timestamp,open,high,low,close,volume)",
            "lignes_ignorees": lignes_ignorees,
        }));
    }

    match state
        .db
        .inserer_bougies_avec_source(&asset, &timeframe, &bougies, SOURCE)
        .await
    {
        Ok(total_inserees) => {
            let doublons = total_lues.saturating_sub(total_inserees);
            tracing::info!(
                "Import CSV {} {} : {} lues, {} insérées, {} doublons, {} lignes ignorées",
                body.asset,
                body.timeframe,
                total_lues,
                total_inserees,
                doublons,
                lignes_ignorees
            );
            HttpResponse::Ok().json(serde_json::json!({
                "total_lues": total_lues,
                "total_inserees": total_inserees,
                "doublons": doublons,
                "lignes_ignorees": lignes_ignorees,
                "asset": body.asset,
                "timeframe": body.timeframe,
            }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "erreur": e.to_string() })),
    }
}

// ─── Tests unitaires (pas de réseau, pas de DB) ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_delimiteur() {
        assert_eq!(detecter_delimiteur("a,b,c,d,e,f"), ',');
        assert_eq!(detecter_delimiteur("a;b;c;d;e;f"), ';');
        // Mélange (décimales à virgule + séparateur ;) → ';'.
        assert_eq!(detecter_delimiteur("ts;o;c;1,5;2"), ';');
        // Égalité ou rien → défaut ','.
        assert_eq!(detecter_delimiteur("sans delimiteur"), ',');
    }

    #[test]
    fn timestamps_unix_secondes_et_millisecondes() {
        let s = parse_timestamp("1786521600").expect("secondes");
        assert_eq!(s.timestamp(), 1786521600);
        let ms = parse_timestamp("1786521600000").expect("millisecondes");
        assert_eq!(ms.timestamp(), 1786521600);
    }

    #[test]
    fn timestamps_dates_lisibles() {
        for (brut, attendu) in [
            ("2026-08-11 14:30:00", 1786458600i64),
            ("2026/08/11 14:30:00", 1786458600),
            ("2026-08-11T14:30:00", 1786458600),
        ] {
            let ts = parse_timestamp(brut).unwrap_or_else(|| panic!("échec parse {}", brut));
            assert_eq!(ts.timestamp(), attendu, "{}", brut);
        }
        // Date seule → minuit UTC ; heure sans secondes acceptée.
        assert_eq!(parse_timestamp("2026-08-11").map(|t| t.timestamp()), Some(1786406400));
        assert!(parse_timestamp("2026-08-11 14:30").is_some());
        // Incohérent → None (jamais de panic).
        assert!(parse_timestamp("timstack").is_none());
        assert!(parse_timestamp("").is_none());
        assert!(parse_timestamp("123").is_none()); // ni 10 ni 13 chiffres
    }

    #[test]
    fn en_tete_detectee_uniquement_si_textuelle() {
        assert!(est_en_tete("timestamp"));
        assert!(est_en_tete("Date"));
        assert!(!est_en_tete("1786521600"));
        assert!(!est_en_tete("2026-08-11 14:30:00"));
    }

    #[test]
    fn parse_ligne_virgules_et_volume_optionnel() {
        let c = parse_ligne("1786521600,4409.66,4414.0,4409.45,4412.28,114.787", ',')
            .expect("bougie valide");
        assert_eq!(c.timestamp.timestamp(), 1786521600);
        assert!((c.open - 4409.66).abs() < 1e-9);
        assert!((c.volume - 114.787).abs() < 1e-9);
        // 5 colonnes → volume 0.
        let c5 = parse_ligne("1786521600,1,2,0.5,1.5", ',').expect("bougie sans volume");
        assert_eq!(c5.volume, 0.0);
        // Décimales françaises avec séparateur ';'.
        let cf = parse_ligne("1786521600;4409,66;4414,0;4409,45;4412,28;114,787", ';')
            .expect("bougie fr");
        assert!((cf.open - 4409.66).abs() < 1e-9);
        assert!((cf.volume - 114.787).abs() < 1e-9);
        // Lignes invalides → None (trop courte, prix négatif, high < low, ts invalide).
        assert!(parse_ligne("1,2,3", ',').is_none());
        assert!(parse_ligne("1786521600,-1,2,0.5,1.5", ',').is_none());
        assert!(parse_ligne("1786521600,1,0.5,2,1.5", ',').is_none());
        assert!(parse_ligne("abc,1,2,0.5,1.5", ',').is_none());
    }

    #[test]
    fn parser_csv_complet_avec_en_tete_et_lignes_vides() {
        let csv = "\u{feff}timestamp,open,high,low,close,volume\n\
                   1786521600,10,11,9,10.5,5\n\n\
                   1786521700,10.5,11.5,10,11,6\n\
                   ligne pourrie,a,b,c,d,e\n";
        let (bougies, ignorees) = parser_csv(csv);
        assert_eq!(bougies.len(), 2, "l'en-tête et la ligne pourrie sont exclues");
        assert_eq!(ignorees, 1);
        assert!(bougies[0].timestamp < bougies[1].timestamp);
    }

    #[test]
    fn parser_csv_point_virgule_avec_date_lisible() {
        let csv = "Date;Open;High;Low;Close;Volume\n\
                   2026-08-11 14:30:00;4409,66;4414,0;4409,45;4412,28;114,787";
        let (bougies, ignorees) = parser_csv(csv);
        assert_eq!(ignorees, 0);
        assert_eq!(bougies.len(), 1);
        assert_eq!(bougies[0].timestamp.timestamp(), 1786458600);
    }

    #[test]
    fn parser_csv_vide() {
        let (bougies, ignorees) = parser_csv("");
        assert!(bougies.is_empty());
        assert_eq!(ignorees, 0);
        let (bougies, _) = parser_csv("\n \n");
        assert!(bougies.is_empty());
    }
}
