//! Import MT5 → CSV → SQLite
//! Scanne MQL5/Files/ pour les fichiers export_*.csv générés par ExportHistorique.mq5
//! Le script MQL5 écrit déjà les IDs DB dans les noms de fichiers (ex: export_DAX_H1.csv)
use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, TimeZone, Utc};
use common::{Asset, Candle, Timeframe};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::state::AppState;

/// Chemin par défaut du dossier MQL5/Files sous Wine
const MT5_PATH_DEFAUT: &str =
    "/home/rono/.mt5/drive_c/Program Files/MetaTrader 5/MQL5/Files";

fn parse_timeframe(s: &str) -> Option<Timeframe> {
    match s {
        "M1"  => Some(Timeframe::M1),
        "M5"  => Some(Timeframe::M5),
        "M15" => Some(Timeframe::M15),
        "M30" => Some(Timeframe::M30),
        "H1"  => Some(Timeframe::H1),
        "H4"  => Some(Timeframe::H4),
        "D1"  => Some(Timeframe::D1),
        "W1"  => Some(Timeframe::W1),
        _     => None,
    }
}

fn parse_asset(s: &str) -> Option<Asset> {
    crate::utils::parse_asset(s)
}

/// Parse une ligne CSV MT5 : timestamp_unix,open,high,low,close,volume
fn parse_ligne(ligne: &str) -> Option<Candle> {
    let parts: Vec<&str> = ligne.split(',').collect();
    if parts.len() < 6 {
        return None;
    }
    let ts_unix: i64 = parts[0].trim().parse().ok()?;
    let open: f64    = parts[1].trim().parse().ok()?;
    let high: f64    = parts[2].trim().parse().ok()?;
    let low: f64     = parts[3].trim().parse().ok()?;
    let close: f64   = parts[4].trim().parse().ok()?;
    let volume: f64  = parts[5].trim().parse().ok()?;
    let timestamp: DateTime<Utc> = Utc.timestamp_opt(ts_unix, 0).single()?;
    Some(Candle { timestamp, open, high, low, close, volume })
}

#[derive(Deserialize)]
pub struct RequeteImportMt5 {
    /// Chemin du dossier MQL5/Files (optionnel — utilise le chemin Wine par défaut)
    pub chemin: Option<String>,
}

// ─── POST /api/data/import-mt5 ────────────────────────────────────────────────
pub async fn post_import_mt5(
    state: web::Data<AppState>,
    body: web::Json<RequeteImportMt5>,
) -> impl Responder {
    let dossier = body
        .chemin
        .clone()
        .unwrap_or_else(|| MT5_PATH_DEFAUT.to_string());

    let mut resultats = Vec::new();
    let mut total_bougies: u64 = 0;
    let mut total_inseres: u64 = 0;

    // Lister les fichiers export_*.csv
    let entrees = match fs::read_dir(&dossier) {
        Ok(e) => e,
        Err(err) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "erreur": format!("Dossier inaccessible '{}' : {}", dossier, err)
            }));
        }
    };

    for entree in entrees.flatten() {
        let path: PathBuf = entree.path();
        let nom = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None    => continue,
        };

        // Ne traiter que export_*.csv
        if !nom.starts_with("export_") || !nom.ends_with(".csv") {
            continue;
        }

        // Extraire l'id DB et le TF depuis "export_DAX_H1.csv"
        // Le script MQL5 écrit directement l'id DB dans le nom (ex: DAX, SP500, EURUSD)
        let sans_ext = nom.trim_end_matches(".csv");
        let parties: Vec<&str> = sans_ext.splitn(3, '_').collect();
        if parties.len() != 3 {
            resultats.push(serde_json::json!({ "fichier": nom, "erreur": "Nom invalide (attendu: export_ASSET_TF.csv)" }));
            continue;
        }
        let db_id  = parties[1]; // Déjà l'id DB — pas de mapping nécessaire
        let tf_str = parties[2];

        let asset = match parse_asset(db_id) {
            Some(a) => a,
            None => {
                resultats.push(serde_json::json!({
                    "fichier": nom, "erreur": format!("Asset '{}' inconnu", db_id)
                }));
                continue;
            }
        };

        let tf = match parse_timeframe(tf_str) {
            Some(t) => t,
            None => {
                resultats.push(serde_json::json!({
                    "fichier": nom, "erreur": format!("Timeframe '{}' inconnu", tf_str)
                }));
                continue;
            }
        };

        // Lire le CSV et construire les bougies
        let contenu = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                resultats.push(serde_json::json!({ "fichier": nom, "erreur": e.to_string() }));
                continue;
            }
        };

        let bougies: Vec<Candle> = contenu
            .lines()
            .filter_map(parse_ligne)
            .collect();

        let nb_lues = bougies.len() as u64;
        if nb_lues == 0 {
            resultats.push(serde_json::json!({
                "fichier": nom, "asset": db_id, "timeframe": tf_str,
                "lues": 0, "inseres": 0
            }));
            continue;
        }

        match state.db.inserer_bougies(&asset, &tf, &bougies).await {
            Ok(inseres) => {
                let doublons = nb_lues.saturating_sub(inseres);
                tracing::info!(
                    "Import MT5 {} {} : {} lues, {} insérées, {} doublons",
                    db_id, tf_str, nb_lues, inseres, doublons
                );
                total_bougies += nb_lues;
                total_inseres += inseres;
                resultats.push(serde_json::json!({
                    "fichier": nom, "asset": db_id, "timeframe": tf_str,
                    "lues": nb_lues, "inseres": inseres, "doublons": doublons
                }));
            }
            Err(e) => {
                resultats.push(serde_json::json!({
                    "fichier": nom, "asset": db_id, "timeframe": tf_str, "erreur": e.to_string()
                }));
            }
        }
    }

    if resultats.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "message": "Aucun fichier export_*.csv trouvé dans le dossier",
            "dossier": dossier,
            "total_bougies": 0, "total_inseres": 0, "resultats": []
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "dossier": dossier,
        "total_bougies": total_bougies,
        "total_inseres": total_inseres,
        "resultats": resultats
    }))
}
