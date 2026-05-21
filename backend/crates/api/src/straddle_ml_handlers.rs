use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state::AppState;

// Re-exports des handlers volumineux extraits dans des modules dédiés
pub use crate::straddle_dev_handlers::{dev_seed_creneaux, dev_signal_test};
pub use crate::straddle_monitoring_handlers::{monitoring_ml, volatilite_live};

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

pub async fn get_calibration(state: web::Data<AppState>) -> impl Responder {
    match db::straddle_calibration::lister_toutes(state.db.pool()).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

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

pub async fn cloturer_feedback(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<BodyCloturer>,
) -> impl Responder {
    let signal_id = path.into_inner();
    let pool = state.db.pool();

    if let Err(e) =
        db::signaux::maj_verdict(pool, &signal_id, &body.verdict, body.prix_verdict).await
    {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() }));
    }

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

#[derive(serde::Deserialize)]
pub struct QueryEquity {
    pub capital: Option<f64>,
    pub risk_pct: Option<f64>,
}

pub async fn get_equity(
    state: web::Data<AppState>,
    query: web::Query<QueryEquity>,
) -> impl Responder {
    let capital = query.capital.unwrap_or(10_000.0);
    let risk_pct = query.risk_pct.unwrap_or(0.015);
    let risk_montant = capital * risk_pct;
    let pool = state.db.pool();

    match db::straddle_feedback_stats::courbe_equity(pool, capital, risk_montant).await {
        Ok(points) => {
            let nb_trades = points.len() as i64;
            HttpResponse::Ok().json(serde_json::json!({
                "capital_initial": capital,
                "risk_pct": risk_pct,
                "nb_trades_saisis": nb_trades,
                "points": points,
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/straddle/seuils-effectifs ────────────────────────────────────────

#[derive(Deserialize)]
pub struct QuerySeuilsStraddle {
    pub asset: Option<String>,
    pub categorie: Option<String>,
}

/// Retourne les seuils effectifs calibrés pour un (asset, catégorie).
/// Fallback automatique sur warm start ou valeurs par défaut si insuffisant.
pub async fn get_seuils_effectifs(
    state: web::Data<AppState>,
    query: web::Query<QuerySeuilsStraddle>,
) -> impl Responder {
    let pool = state.db.pool();
    let asset = query.asset.as_deref().unwrap_or("BTCUSDT");
    let categorie = query.categorie.as_deref().unwrap_or("AtrPur");

    let seuils = db::straddle_calibration::charger_seuils(pool, asset, categorie).await;

    HttpResponse::Ok().json(serde_json::json!({
        "asset":           asset,
        "categorie":       categorie,
        "score_llm":       seuils.score_llm,
        "ratio_atr":       seuils.ratio_atr,
        "sl_ratio":        seuils.sl_ratio,
        "tp1_ratio":       seuils.tp1_ratio,
        "tp2_ratio":       seuils.tp2_ratio,
        "trailing_coeff":  seuils.trailing_coeff,
        "invalide":        seuils.invalide,
    }))
}
