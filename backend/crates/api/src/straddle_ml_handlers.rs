//! Endpoints API pour le monitoring ML de la stratégie Straddle.
//!
//! Routes :
//!   GET  /api/straddle/volatilite-live          → pics 2h + annonces imminentes
//!   GET  /api/straddle/monitoring-ml             → stats globales + par catégorie
//!   GET  /api/straddle/calibration               → seuils calibrés par asset+catégorie
//!   GET  /api/straddle/pics                      → liste paginée des pics
//!   GET  /api/straddle/feedback                  → historique des feedbacks filtrés
//!   POST /api/straddle/feedback/{signal_id}/cloturer → clôture manuelle
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state::AppState;

// Re-exports des handlers volumineux extraits dans un module dédié
pub use crate::straddle_monitoring_handlers::{monitoring_ml, volatilite_live};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct QueryPics {
    pub asset: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct QueryFeedback {
    pub asset: Option<String>,
    pub categorie: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct BodyCloturer {
    pub verdict: String, // "tp1" | "tp2" | "tp3" | "sl" | "expire"
    pub prix_verdict: f64,
}

// ── GET /api/straddle/calibration ─────────────────────────────────────────────

pub async fn get_calibration(state: web::Data<AppState>) -> impl Responder {
    match db::straddle_calibration::lister_toutes(state.db.pool()).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/straddle/pics ────────────────────────────────────────────────────

pub async fn get_pics(state: web::Data<AppState>, query: web::Query<QueryPics>) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(200);
    let pool = state.db.pool();

    let result = match &query.asset {
        Some(asset) => db::straddle_pics::lister_recents_asset(pool, asset, 48, limit).await,
        None => db::straddle_pics::lister_recents(pool, 48, limit).await,
    };

    match result {
        Ok(pics) => HttpResponse::Ok().json(pics),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/straddle/feedback ────────────────────────────────────────────────

pub async fn get_feedback(
    state: web::Data<AppState>,
    query: web::Query<QueryFeedback>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(500);
    let pool = state.db.pool();

    let asset = query.asset.as_deref().unwrap_or("%");
    let categorie = query.categorie.as_deref().unwrap_or("%");

    match db::straddle_feedback::lister_recents_asset_categorie(pool, asset, categorie, limit).await
    {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── POST /api/straddle/feedback/{signal_id}/cloturer ─────────────────────────

pub async fn cloturer_feedback(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<BodyCloturer>,
) -> impl Responder {
    let signal_id = path.into_inner();
    let pool = state.db.pool();

    // Maj dans signaux
    if let Err(e) =
        db::signaux::maj_verdict(pool, &signal_id, &body.verdict, body.prix_verdict).await
    {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() }));
    }

    // Charger les infos nécessaires pour le feedback (prix_entree, cree_le)
    use sqlx::Row;
    let sig = sqlx::query("SELECT prix_entree, stop_loss, cree_le FROM signaux WHERE id = ?")
        .bind(&signal_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    if let Some(s) = sig {
        let prix_entree: f64 = s.get("prix_entree");
        let stop_loss: f64 = s.get("stop_loss");
        let cree_le: i64 = s.get("cree_le");
        let risque = (prix_entree - stop_loss).abs().max(f64::EPSILON);
        let _ = db::straddle_feedback::maj_feedback_verdict(
            pool,
            &signal_id,
            &body.verdict,
            prix_entree,
            body.prix_verdict,
            risque,
            cree_le,
        )
        .await;
    }

    HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
}
