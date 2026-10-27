//! Handlers HTTP des analyses (extrait d'analyses.rs — garde-fou 600
//! lignes). Même pattern que presse_handlers / prix_handlers.

use actix_web::{web, HttpResponse};

use super::analyser;
use crate::state::AppState;

/// GET /api/analyses — vue d'ensemble : une ligne par stratégie active.
pub async fn get_analyses(state: web::Data<AppState>) -> impl actix_web::Responder {
    let ids: Vec<&str> = crate::registre_strategies::MANIFESTES
        .iter()
        .map(|m| m.id)
        .collect();
    let mut liste = Vec::with_capacity(ids.len());
    for id in ids {
        let a = analyser(&state.db, id).await;
        liste.push(serde_json::json!({
            "strategie": a.strategie,
            "etat": a.etat,
            "source": a.source,
            "nb_trades": a.nb_trades,
            "capital_depart": a.capital_depart,
            "capital_actuel": a.capital_actuel,
            "r_total": a.r_total,
            "r_distance_total": a.r_distance_total,
            "recalcul": a.recalcul,
            "taux_reussite": a.taux_reussite,
            "hier": a.hier,
        }));
    }
    HttpResponse::Ok().json(serde_json::json!({ "strategies": liste }))
}

/// GET /api/analyses/{strategie}/historique — snapshots quotidiens persistés
/// (§14 : évolution des métriques et des avis IA jour après jour).
pub async fn get_historique_analyses(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    match state.db.lister_analyses_snapshots(&id, 60).await {
        Ok(snapshots) => HttpResponse::Ok().json(serde_json::json!({ "snapshots": snapshots })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/analyses/{strategie} — analyse complète d'une stratégie.
pub async fn get_analyse(state: web::Data<AppState>, path: web::Path<String>) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    HttpResponse::Ok().json(analyser(&state.db, &id).await)
}
