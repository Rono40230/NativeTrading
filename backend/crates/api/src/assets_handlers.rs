use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct AjoutAssetBody {
    pub id: String,
    pub nom: String,
    #[serde(rename = "type")]
    pub type_asset: String,
    pub source: String,
    /// Symbole Bybit — requis si source = binance.
    pub symbol_bybit: Option<String>,
    /// Symbole MT5 du broker (ex : dax40.fs) — requis si source = mt5.
    pub symbol_mt5: Option<String>,
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
    // Règle famille → worker (actée 26/08) : 'binance' = Bybit WS (crypto),
    // 'mt5' = broker Axi (métaux, forex, indices). Dukascopy supprimé.
    let sources_valides = ["binance", "mt5"];
    if !sources_valides.contains(&body.source.as_str()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Source invalide. Valeurs : 'binance', 'mt5'."
        }));
    }

    // Réactivation (écart 24, 13/10) : recocher une case de la page Données
    // n'envoie PAS les symboles — si l'asset existe soft-deleted, ses
    // symboles actuels font foi ; la validation ci-dessous ne s'applique
    // alors qu'à la CRÉATION (nouveau ticker).
    let existant = sqlx::query(
        "SELECT symbol_bybit, symbol_mt5 FROM assets WHERE id = ? AND actif = 0",
    )
    .bind(&id)
    .fetch_optional(state.db.pool())
    .await
    .ok()
    .flatten();
    let symbole_existant_bybit = existant
        .as_ref()
        .and_then(|r| r.get::<Option<String>, _>("symbol_bybit"))
        .filter(|s| !s.trim().is_empty());
    let symbole_existant_mt5 = existant
        .as_ref()
        .and_then(|r| r.get::<Option<String>, _>("symbol_mt5"))
        .filter(|s| !s.trim().is_empty());

    // Cohérence source ↔ symbole : un asset Bybit exige son symbole Bybit,
    // un asset MT5 exige son symbole broker (sauf réactivation : cf. ci-dessus).
    let symbol_bybit = match body.source.as_str() {
        "binance" => {
            let symbole = body
                .symbol_bybit
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_uppercase)
                .or(symbole_existant_bybit);
            if symbole.is_none() {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Un asset Bybit exige son symbole (ex : TON → TONUSDT)."
                }));
            }
            symbole
        }
        "mt5" => {
            let fourni = body
                .symbol_mt5
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if fourni.is_none() && symbole_existant_mt5.is_none() {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Un asset MT5 exige son symbole broker (ex : DAX → dax40.fs)."
                }));
            }
            None
        }
        _ => None,
    };

    match state
        .db
        .ajouter_asset(
            &id,
            &nom,
            &body.type_asset,
            &body.source,
            symbol_bybit.as_deref(),
            body
                .symbol_mt5
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or(symbole_existant_mt5.as_deref()),
        )
        .await
    {
        Ok(()) => {
            // Ajout pipeline (doctrine 15/10) : disponible partout, ARMÉ
            // NULLE PART — mais les analyses le voient immédiatement.
            crate::volatility_handlers::vider_cache_patterns().await;
            crate::evenements_matrice::vider_cache().await;
            crate::briefing_ia::vider_cache();
            HttpResponse::Ok().json(serde_json::json!({ "ok": true, "id": id }))
        }
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
    // DOCTRINE PIPELINE (owner 15/10) : le retrait CASCADE — l'asset quitte
    // le périmètre straddle (+ cases), ses couples SMC, la liste KDJ, et
    // les caches d'analyse se vident. Jamais de suppression de données.
    let recap = crate::pipeline_cascade::retirer(&state.db, &id).await;
    match state.db.supprimer_asset(&id).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true, "retrait": recap })),
        Err(e) => {
            tracing::warn!("supprimer_asset '{}': {}", id, e);
            HttpResponse::NotFound().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
