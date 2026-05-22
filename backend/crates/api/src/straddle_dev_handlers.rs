//! Endpoints de développement/test straddle — désactivés en production.
//! Activation : variable d'env STRADDLE_DEV_ENDPOINTS=1
use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use common::{Direction, Signal};
use serde::Deserialize;

use crate::state::AppState;

fn dev_endpoints_actifs() -> bool {
    std::env::var("STRADDLE_DEV_ENDPOINTS").ok().as_deref() == Some("1")
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
    /// ATR actuel — si absent, fallback sur 0.4% du prix d'entrée.
    pub atr: Option<f64>,
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

    // Charger les multiplicateurs SL/TP depuis StraddleParams (même source que la boucle auto)
    let straddle_params = db::strategies_params::lire_straddle_params(state.db.pool()).await;
    let atr = body.atr.unwrap_or(prix_entree * 0.004).max(0.0001);
    let sl_long  = prix_entree - straddle_params.sl_mult  * atr;
    let sl_short = prix_entree + straddle_params.sl_mult  * atr;
    let tp_long  = vec![
        prix_entree + straddle_params.tp_mult_1 * atr,
        prix_entree + straddle_params.tp_mult_2 * atr,
        prix_entree + straddle_params.tp_mult_3 * atr,
    ];
    let tp_short = vec![
        prix_entree - straddle_params.tp_mult_1 * atr,
        prix_entree - straddle_params.tp_mult_2 * atr,
        prix_entree - straddle_params.tp_mult_3 * atr,
    ];

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
        "atr_utilise": atr,
        "params_utilises": {
            "sl_mult":   straddle_params.sl_mult,
            "tp_mult_1": straddle_params.tp_mult_1,
            "tp_mult_2": straddle_params.tp_mult_2,
            "tp_mult_3": straddle_params.tp_mult_3,
        },
        "stop_loss_long":  sl_long,
        "stop_loss_short": sl_short,
        "tp_long":  tp_long,
        "tp_short": tp_short,
    }))
}
