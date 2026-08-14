use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct AjoutAssetBody {
    pub id: String,
    pub nom: String,
    #[serde(rename = "type")]
    pub type_asset: String,
    pub source: String,
}

#[derive(serde::Deserialize)]
pub struct QueryLister {
    // Par défaut : retourner TOUS les assets (actifs + inactifs)
    // Passer ?actifs=true pour ne recevoir que les actifs
    pub actifs: Option<String>,
}

/// GET /api/assets — retourne TOUS les assets par défaut (actifs + inactifs)
/// GET /api/assets?actifs=true — retourne uniquement les assets actifs
pub async fn lister_assets(
    state: web::Data<AppState>,
    query: web::Query<QueryLister>,
) -> impl Responder {
    let actifs_seulement = matches!(query.actifs.as_deref(), Some("true") | Some("1"));
    let res = if actifs_seulement {
        state.db.lister_assets().await // actifs uniquement
    } else {
        state.db.lister_tous_assets().await // tous (actifs + inactifs)
    };
    match res {
        Ok(assets) => HttpResponse::Ok().json(assets),
        Err(e) => {
            tracing::error!("lister_assets: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// POST /api/assets — ajoute un nouvel asset
pub async fn ajouter_asset(
    state: web::Data<AppState>,
    body: web::Json<AjoutAssetBody>,
) -> impl Responder {
    // Validation format ticker
    let id = body.id.trim().to_uppercase();
    if id.is_empty() || id.len() < 2 || id.len() > 20 {
        return HttpResponse::BadRequest().json(
            serde_json::json!({ "error": "Le ticker doit faire entre 2 et 20 caractères." }),
        );
    }
    if !id.chars().all(|c| c.is_alphanumeric()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Le ticker ne peut contenir que des lettres et chiffres." }));
    }
    let nom = body.nom.trim().to_string();
    if nom.is_empty() || nom.len() > 60 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Le nom doit faire entre 1 et 60 caractères." }));
    }
    let types_valides = ["crypto", "metal", "forex", "indice"];
    if !types_valides.contains(&body.type_asset.as_str()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Type invalide. Valeurs autorisées : {:?}", types_valides)
        }));
    }
    let sources_valides = ["binance"];
    if !sources_valides.contains(&body.source.as_str()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Source invalide. Valeurs : 'binance'."
        }));
    }

    match state
        .db
        .ajouter_asset(&id, &nom, &body.type_asset, &body.source)
        .await
    {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true, "id": id })),
        Err(e) => {
            let msg = match &e {
                common::TradingError::Data(m) => m.clone(),
                _ => e.to_string(),
            };
            tracing::warn!("ajouter_asset '{}': {}", id, msg);
            HttpResponse::Conflict().json(serde_json::json!({ "error": msg }))
        }
    }
}

/// DELETE /api/assets/{id} — soft-delete un asset
pub async fn supprimer_asset(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner().to_uppercase();
    match state.db.supprimer_asset(&id).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            tracing::warn!("supprimer_asset '{}': {}", id, e);
            HttpResponse::NotFound().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct MlActifBody {
    pub ml_actif: bool,
}

/// PATCH /api/assets/{id}/ml — active ou désactive l'asset pour le réentraînement ML
pub async fn set_ml_actif(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<MlActifBody>,
) -> impl Responder {
    let id = path.into_inner().to_uppercase();
    match state.db.set_ml_actif(&id, body.ml_actif).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true, "ml_actif": body.ml_actif })),
        Err(e) => {
            tracing::warn!("set_ml_actif '{}': {}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
