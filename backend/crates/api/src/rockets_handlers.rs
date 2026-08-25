use actix_web::{web, HttpResponse, Responder};
use db::rockets;
use serde::Deserialize;

use crate::state::AppState;

// ── Config endpoints ─────────────────────────────────────────────────────────

/// GET /api/rockets/config
pub async fn get_config(state: web::Data<AppState>) -> impl Responder {
    let cfg = rockets::lire_config(state.db.pool()).await;
    HttpResponse::Ok().json(cfg)
}

/// PUT /api/rockets/config
pub async fn put_config(
    state: web::Data<AppState>,
    body: web::Json<rockets::RocketsConfig>,
) -> impl Responder {
    match rockets::sauvegarder_config(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── DTOs ────────────────────────────────────────────────────────────────────


#[derive(Deserialize)]
pub struct QueryHistorique {
    pub limite: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────



/// GET /api/rockets/historique?limite=50 — uniquement les trades clôturés (statut='ferme')
pub async fn get_historique(
    state: web::Data<AppState>,
    query: web::Query<QueryHistorique>,
) -> impl Responder {
    let pool = state.db.pool();
    let limite = query.limite.unwrap_or(50);
    match rockets::historique(pool, limite).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique rockets: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/rockets/actifs — trades en cours (statut='ouvert' ou 'attente')
pub async fn get_actifs(state: web::Data<AppState>) -> impl Responder {
    match rockets::lister_actifs(state.db.pool()).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Rockets actifs: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// DELETE /api/rockets/signal/{id} — annule et supprime un signal actif
pub async fn supprimer_signal(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match rockets::supprimer(state.db.pool(), id).await {
        Ok(true)  => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Ok(false) => HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Signal introuvable ou déjà clôturé" })),
        Err(e) => {
            tracing::error!("Suppression rocket {}: {}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
