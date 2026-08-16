//! Endpoints du runtime tick (Phases 1.5 et 2.5).
//!
//! - `GET  /api/runtime/concordance?heures=24` — Gate 1 : concordance des
//!   bougies runtime vs officielles ;
//! - `POST /api/runtime/replay` — Gate 2 (méthode R) : rejoue l'historique
//!   DB par le chemin du plugin v12, archive le journal, verdict de parité ;
//! - `GET  /api/runtime/replay` — derniers runs archivés ;
//! - `GET  /api/runtime/replay/{id}` — journal complet d'un run.

use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;

use crate::state::AppState;

/// Fenêtre par défaut (Gate 1 : 24 h).
const HEURES_DEFAUT: i64 = 24;
/// Fenêtre maximale (limite de charge).
const HEURES_MAX: i64 = 24 * 7;

pub async fn get_concordance(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let heures = query
        .get("heures")
        .and_then(|h| h.parse::<i64>().ok())
        .unwrap_or(HEURES_DEFAUT)
        .clamp(1, HEURES_MAX);

    let depuis_ts = Utc::now().timestamp() - heures * 3600;

    match state.db.lire_concordance(depuis_ts).await {
        Ok(rapport) => HttpResponse::Ok().json(rapport),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("lecture concordance impossible: {}", e)),
    }
}

/// Corps de `POST /api/runtime/replay`.
#[derive(Debug, Deserialize)]
pub struct CorpsReplay {
    pub asset: String,
    pub timeframe: String,
    /// Profondeur en semaines (1..=52, défaut 4 — Gate 2 : ≥ 4).
    #[serde(default)]
    pub semaines: Option<i64>,
    /// Simuler les évaluations intrabar (mode alertes) — défaut faux (parité).
    #[serde(default)]
    pub simuler_ticks: Option<bool>,
}

/// Exécute un replay v12 sur l'historique DB et archive le journal.
pub async fn post_replay(state: web::Data<AppState>, corps: web::Json<CorpsReplay>) -> HttpResponse {
    let Ok(asset) = common::Asset::try_from(corps.asset.as_str()) else {
        return HttpResponse::BadRequest().body(format!("asset inconnu: {}", corps.asset));
    };
    let Ok(tf) = common::Timeframe::try_from(corps.timeframe.as_str()) else {
        return HttpResponse::BadRequest()
            .body(format!("timeframe inconnu: {}", corps.timeframe));
    };
    let semaines = corps.semaines.unwrap_or(4).clamp(1, 52);
    let simuler_ticks = corps.simuler_ticks.unwrap_or(false);

    // Profondeur : semaines × barres/semaine (marché 24/7 — majorant).
    let barres = semaines * 7 * 1440 / tf.minutes() as i64;
    let bougies = match state.db.obtenir_bougies(&asset, &tf, barres).await {
        Ok(b) if !b.is_empty() => b,
        Ok(_) => {
            return HttpResponse::NotFound()
                .body(format!("aucune bougie {} {}", corps.asset, corps.timeframe))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("lecture DB: {}", e))
        }
    };

    let resultat = engine_v12::replay::rejouer_bougies(asset, tf, &bougies, simuler_ticks);
    let resume = engine_v12::replay::resume(&resultat);

    let journal = serde_json::json!({
        "signaux": resultat.signaux,
        "evenements": resultat.evenements,
    });
    let archivage = state
        .db
        .inserer_run_replay(
            &resultat.asset,
            &resultat.timeframe,
            resultat.simule_ticks,
            resultat.nb_bougies,
            resultat.periode_de,
            resultat.periode_a,
            resultat.signaux.len(),
            resultat.evenements.len(),
            resultat.conforme_reference,
            resultat.nb_trades_reference,
            resultat.duree_ms as u64,
            &journal,
        )
        .await;

    match archivage {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({
            "id": id,
            "resume": resume,
            "conforme_reference": resultat.conforme_reference,
            "nb_signaux": resultat.signaux.len(),
            "nb_evenements": resultat.evenements.len(),
            "nb_trades_reference": resultat.nb_trades_reference,
        })),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("archivage impossible: {}", e)),
    }
}

/// Derniers runs archivés (résumés).
pub async fn get_replays(state: web::Data<AppState>) -> HttpResponse {
    match state.db.lister_runs_replay(20).await {
        Ok(runs) => HttpResponse::Ok().json(runs),
        Err(e) => HttpResponse::InternalServerError().body(format!("lecture runs: {}", e)),
    }
}

/// Journal complet d'un run.
pub async fn get_replay_journal(
    state: web::Data<AppState>,
    chemin: web::Path<i64>,
) -> HttpResponse {
    match state.db.journal_run_replay(chemin.into_inner()).await {
        Ok(Some(journal)) => HttpResponse::Ok().json(journal),
        Ok(None) => HttpResponse::NotFound().body("run inconnu"),
        Err(e) => HttpResponse::InternalServerError().body(format!("lecture journal: {}", e)),
    }
}

/// Émissions LIVE du runtime (shadow mode) — `?heures=24&asset=BTC&tf=M15&type=signal`.
/// La matière brute du test de vérité : chaque signal/événement à l'instant
/// exact de son émission.
pub async fn get_emissions(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let heures = query
        .get("heures")
        .and_then(|h| h.parse::<i64>().ok())
        .unwrap_or(24)
        .clamp(1, 24 * 30);
    let depuis_ms = Utc::now().timestamp_millis() - heures * 3_600_000;

    match state
        .db
        .lister_emissions(
            depuis_ms,
            query.get("asset").map(|s| s.as_str()),
            query.get("tf").map(|s| s.as_str()),
            query.get("type").map(|s| s.as_str()),
        )
        .await
    {
        Ok(emissions) => HttpResponse::Ok().json(emissions),
        Err(e) => HttpResponse::InternalServerError().body(format!("lecture émissions: {}", e)),
    }
}
