use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use common::{Direction, Signal};
use serde::Deserialize;

use crate::state::AppState;

// Re-exports des handlers volumineux extraits dans un module dédié
pub use crate::straddle_monitoring_handlers::{monitoring_ml, volatilite_live};

fn dev_endpoints_actifs() -> bool {
    std::env::var("STRADDLE_DEV_ENDPOINTS").ok().as_deref() == Some("1")
}

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

#[derive(Deserialize)]
pub struct BodySeedCreneauxDev {
    pub asset: Option<String>,
}

#[derive(Deserialize)]
pub struct BodySignalTestDev {
    pub asset: String,
    pub timeframe: Option<String>,
    pub prix_entree: Option<f64>,
    pub ratio_atr: Option<f64>,
    pub categorie: Option<String>,
}

pub async fn dev_seed_creneaux(
    state: web::Data<AppState>,
    body: web::Json<BodySeedCreneauxDev>,
) -> impl Responder {
    if !dev_endpoints_actifs() {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Endpoint dev désactivé (STRADDLE_DEV_ENDPOINTS=1 requis)",
        }));
    }

    let asset = body.asset.clone().unwrap_or_else(|| "BTC".to_string()).to_uppercase();
    if crate::utils::parse_asset(&asset).is_none() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Asset non supporté" }));
    }

    let _ = db::straddle::supprimer_creneaux_asset(state.db.pool(), &asset).await;
    let seed = vec![
        db::straddle::NouveauCreneau {
            asset: asset.clone(),
            jour_semaine: Some(1),
            heure_debut: "14:00".to_string(),
            heure_fin: "16:00".to_string(),
            atr_moyen: Some(1.2),
            frequence: Some(0.62),
            llm_raison: Some("Seed dev: ouverture US".to_string()),
            llm_conviction: Some(78),
        },
        db::straddle::NouveauCreneau {
            asset: asset.clone(),
            jour_semaine: Some(3),
            heure_debut: "15:00".to_string(),
            heure_fin: "17:00".to_string(),
            atr_moyen: Some(1.35),
            frequence: Some(0.57),
            llm_raison: Some("Seed dev: confluence calendrier".to_string()),
            llm_conviction: Some(74),
        },
    ];

    match db::straddle::inserer_creneaux(state.db.pool(), &seed).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "asset": asset,
            "inserted": seed.len(),
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string(),
        })),
    }
}

pub async fn dev_signal_test(
    state: web::Data<AppState>,
    body: web::Json<BodySignalTestDev>,
) -> impl Responder {
    if !dev_endpoints_actifs() {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Endpoint dev désactivé (STRADDLE_DEV_ENDPOINTS=1 requis)",
        }));
    }

    let asset_str = body.asset.trim().to_uppercase();
    let Some(asset) = crate::utils::parse_asset(&asset_str) else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Asset non supporté" }));
    };
    let tf = crate::utils::parse_timeframe(body.timeframe.as_deref().unwrap_or("M15"));
    let prix_entree = body.prix_entree.unwrap_or(100.0).max(0.0001);
    let risque = (prix_entree * 0.004).max(0.0001);
    let sl_long = prix_entree - risque;
    let sl_short = prix_entree + risque;
    let tp_long = vec![prix_entree + 2.0 * risque, prix_entree + 3.0 * risque, prix_entree + 5.0 * risque];
    let tp_short = vec![prix_entree - 2.0 * risque, prix_entree - 3.0 * risque, prix_entree - 5.0 * risque];

    let signal = Signal::nouveau(
        asset.clone(),
        tf,
        Direction::Both,
        78.0,
        prix_entree,
        sl_long,
        tp_long.clone(),
        "straddle",
    );

    if let Err(e) = state
        .db
        .inserer_signal_straddle_complet(&signal, sl_short, &tp_short, None)
        .await
    {
        return HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }));
    }

    let categorie = body.categorie.as_deref().unwrap_or("dev_test");
    let ratio_atr = body.ratio_atr.unwrap_or(1.3);
    let _ = db::straddle_feedback::inserer_feedback(
        state.db.pool(),
        &db::straddle_feedback::NouveauFeedback {
            signal_id: &signal.id.to_string(),
            pic_id: None,
            asset: asset_str.as_str(),
            timeframe: tf.as_str(),
            timestamp_signal: Utc::now().timestamp(),
            categorie,
            evenement_nom: Some("DEV_TEST"),
            session_active: Some("DEV"),
            ratio_atr,
            score_llm: 7.8,
        },
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "ok": true,
        "signal_id": signal.id.to_string(),
        "asset": asset_str,
        "timeframe": tf.as_str(),
        "prix_entree": prix_entree,
        "stop_loss_long": sl_long,
        "stop_loss_short": sl_short,
        "tp_long": tp_long,
        "tp_short": tp_short,
    }))
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
