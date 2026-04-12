use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use smc::{kill_zone, scorer, sweep};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

#[derive(Deserialize)]
pub struct SmcQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

/// GET /api/smc/analyse?asset=BTC&timeframe=M15&limit=200
/// Retourne le score de confluence SMC (0–100) et les détails par composant.
pub async fn analyse_smc(
    query: web::Query<SmcQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": format!("Asset inconnu: {}", query.asset) }))
        }
    };
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200) as i64;

    let bougies = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit)
        .await
        .unwrap_or_default();

    if bougies.len() < 30 {
        // Pas assez de données → retourner un score zéro (200) au lieu de 503
        return HttpResponse::Ok().json(serde_json::json!({
            "total": 0.0,
            "tendance": 0.0,
            "order_block": 0.0,
            "imbalance": 0.0,
            "ifvg": 0.0,
            "fibonacci": 0.0,
            "direction": "Both",
            "confluence": false,
            "kill_zone_active": false,
            "sweep_detecte": false,
            "message": "IB Gateway non connecté — données insuffisantes"
        }));
    }

    match scorer(&bougies) {
        Some(score) => HttpResponse::Ok().json(score),
        None => HttpResponse::Ok().json(serde_json::json!({
            "total": 0.0,
            "tendance": 0.0,
            "order_block": 0.0,
            "imbalance": 0.0,
            "ifvg": 0.0,
            "fibonacci": 0.0,
            "direction": "Both",
            "confluence": false,
            "kill_zone_active": smc::kill_zone::est_en_kill_zone(chrono::Utc::now()),
            "sweep_detecte": false,
            "message": "Marché indécis ou données insuffisantes"
        })),
    }
}

/// GET /api/smc/score-debug?asset=EURUSD&timeframe=M15
/// Retourne le score SMC détaillé + diagnostic (raison du blocage, heure UTC, kill zone, sweep).
/// Utile pour comprendre pourquoi SMC ne génère aucun signal.
pub async fn score_debug(
    query: web::Query<SmcQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": format!("Asset inconnu: {}", query.asset) }))
        }
    };
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200) as i64;

    let bougies = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit)
        .await
        .unwrap_or_default();

    let params = db::strategies_params::lire_smc_params(state.db.pool()).await;
    let maintenant = chrono::Utc::now();
    let derniere_ts = bougies.last().map(|b| b.timestamp).unwrap_or(maintenant);

    let en_kill_zone = kill_zone::est_en_kill_zone(derniere_ts);
    let nom_kz = kill_zone::nom_kill_zone(derniere_ts);
    let sweep = sweep::detecter_sweep(&bougies);
    let score = if bougies.len() >= 30 {
        scorer(&bougies)
    } else {
        None
    };

    // Diagnostiquer ce qui bloquerait le signal
    let mut bloqueurs: Vec<&str> = Vec::new();
    if bougies.len() < 30 {
        bloqueurs.push("données insuffisantes (<30 bougies)");
    }
    if params.kill_zone_filtre && !en_kill_zone {
        bloqueurs.push("hors Kill Zone ICT");
    }
    if sweep.is_none() && bougies.len() >= 30 {
        bloqueurs.push("aucun Liquidity Sweep détecté");
    }
    if let Some(ref s) = score {
        if s.total < params.score_min as f64 {
            bloqueurs.push("score < score_min");
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "asset": query.asset,
        "timeframe": query.timeframe.as_deref().unwrap_or("M15"),
        "bougies_disponibles": bougies.len(),
        "heure_utc": maintenant.format("%H:%M UTC").to_string(),
        "derniere_bougie_ts": derniere_ts.format("%Y-%m-%d %H:%M UTC").to_string(),
        "kill_zone": {
            "filtre_actif": params.kill_zone_filtre,
            "en_kill_zone": en_kill_zone,
            "session": nom_kz,
        },
        "sweep_detecte": sweep.is_some(),
        "score": score.as_ref().map(|s| serde_json::json!({
            "total": s.total,
            "tendance": s.tendance,
            "order_block": s.order_block,
            "imbalance": s.imbalance,
            "ifvg": s.ifvg,
            "fibonacci": s.fibonacci,
            "direction": s.direction,
            "confluence": s.confluence,
        })),
        "score_min_requis": params.score_min,
        "signal_emis": bloqueurs.is_empty(),
        "bloqueurs": bloqueurs,
    }))
}
