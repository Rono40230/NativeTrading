//! Frise des états de l'OTE swing pour le replay par période (owner 28/10).
//!
//! `GET /api/smc/v12/ote-frise?asset=XAUUSD&timeframe=M30&debut=<unix>&fin=<unix>`
//!
//! Rejoue le moteur sur les bougies clôturées ≤ fin et capture CHAQUE
//! changement de la zone OTE swing (sens + ancres + bande). Le frontend
//! anime la période en local, curseur bougie par bougie, SANS requête par
//! pas : état au curseur = dernier état de la frise dont ts ≤ curseur.
//! Un seul appel par période — la version réseau (1 requête/barre) a été
//! écartée d'emblée (owner 28/10 : « version 2 »).

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use smc::v12::lifecycle::ModeBeForce;
use smc::v12::{BarInput, SmcV12Engine};

use crate::smc_v12_handlers::duree_tf_s;
use crate::smc_v12_out::SwingOteOut;
use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

#[derive(Deserialize)]
pub(crate) struct FriseQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    /// Début de la période rejouée (secondes Unix).
    pub debut: i64,
    /// Fin de la période (les bougies sont clôturées ≤ fin).
    pub fin: i64,
    pub limit: Option<u32>,
}

/// Un état de la zone OTE, valable à partir de la barre `ts`.
#[derive(Serialize)]
pub(crate) struct EtatFrise {
    pub ts: i64,
    pub zone: Option<SwingOteOut>,
}

#[derive(Serialize)]
struct FriseResponse {
    asset: String,
    timeframe: String,
    debut: i64,
    fin: i64,
    nb_bougies: usize,
    /// Changements successifs — le 1er = état en sortie de warmup.
    etats: Vec<EtatFrise>,
}

/// Signature d'une zone (ce qui définit un « changement ») : sens + ancres.
type SigZone = Option<(bool, f64, f64, i64, i64)>;

fn signature(z: Option<smc::v12::SwingOteZone>) -> SigZone {
    z.map(|z| (z.bearish, z.haut.prix, z.bas.prix, z.haut.ts, z.bas.ts))
}

pub async fn ote_frise(query: web::Query<FriseQuery>, state: web::Data<AppState>) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Asset inconnu: {}", query.asset)
            }));
        }
    };
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    if query.debut <= 0 || query.fin <= query.debut {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"error": "bornes invalides : fin > debut > 0 (secondes Unix)"}),
        );
    }
    let limit = query.limit.unwrap_or(1000).clamp(200, 5000) as i64;
    let cutoff = query.fin - duree_tf_s(&timeframe);
    let bougies = state
        .db
        .obtenir_bougies_avant(&asset, &timeframe, limit, cutoff)
        .await
        .unwrap_or_default();

    // Rejeu minimal : l'OTE swing est TF-locale, pas d'amorce MTF nécessaire.
    let mut engine = SmcV12Engine::new(asset.as_str(), timeframe.as_str())
        .avec_mode_be_force(ModeBeForce::Supprime);
    let mut etats: Vec<EtatFrise> = Vec::new();
    let mut precedente: SigZone = None;
    let mut premiere_barre = true;
    for b in &bougies {
        engine.update(&BarInput {
            timestamp: b.timestamp.timestamp(),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        });
        let zone = engine.swing_ote.zone();
        let sig = signature(zone);
        if sig != precedente || premiere_barre {
            precedente = sig;
            premiere_barre = false;
            etats.push(EtatFrise {
                ts: b.timestamp.timestamp(),
                zone: zone.map(SwingOteOut::depuis),
            });
        }
    }

    HttpResponse::Ok().json(FriseResponse {
        asset: asset.as_str().to_string(),
        timeframe: timeframe.as_str().to_string(),
        debut: query.debut,
        fin: query.fin,
        nb_bougies: bougies.len(),
        etats,
    })
}
