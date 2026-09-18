//! Périmètre straddle — « Choix des Assets & créneaux » (17/09).
//! Extraits de creneaux_ia.rs (limite 600 lignes).

use crate::state::AppState;
use actix_web::{web, HttpResponse};

// ── Périmètre straddle (17/09 — « Choix des Assets & créneaux ») ─────────────

/// GET /api/straddle/perimetre — liste effective (config ou défaut).
pub async fn get_perimetre(state: web::Data<AppState>) -> impl actix_web::Responder {
    let liste = crate::runtime_perimetre::lire_perimetre_straddle(&state.db).await;
    let par_defaut = state
        .db
        .lire_config("perimetre_straddle")
        .await
        .ok()
        .flatten()
        .is_none();
    HttpResponse::Ok().json(serde_json::json!({ "assets": liste, "defaut": par_defaut }))
}

#[derive(serde::Deserialize)]
pub struct BodyPerimetre {
    pub assets: Vec<String>,
}

/// PUT /api/straddle/perimetre — sélection propriétaire des assets surveillés
/// (moteurs M1 + annonces). Validation : assets connus et actifs, dédoublonnés,
/// liste non vide. Le vécu et les créneaux armés ne sont pas effacés : les
/// créneaux d'assets retirés sont simplement ignorés (badge dans l'agenda).
pub async fn put_perimetre(
    state: web::Data<AppState>,
    body: web::Json<BodyPerimetre>,
) -> impl actix_web::Responder {
    let demande = body.into_inner().assets;
    if demande.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Le périmètre ne peut pas être vide" }));
    }
    // Assets connus et actifs uniquement.
    let connus: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT id FROM assets WHERE actif = 1",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();
    let inconnus: Vec<String> = demande
        .iter()
        .filter(|a| !connus.iter().any(|c| c == *a))
        .cloned()
        .collect();
    if !inconnus.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Assets inconnus ou inactifs : {}", inconnus.join(", "))
        }));
    }
    // Dédoublonnage en conservant l'ordre.
    let mut vus = std::collections::HashSet::new();
    let assets: Vec<String> = demande
        .into_iter()
        .filter(|a| vus.insert(a.clone()))
        .collect();
    let valeur = serde_json::to_string(&assets).unwrap_or_default();
    if let Err(e) = state
        .db
        .ecrire_config("perimetre_straddle", &valeur)
        .await
    {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() }));
    }
    // Créneaux armés désormais hors périmètre (information, pas de purge).
    let hors: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT asset FROM creneaux_ia WHERE arme = 1",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default()
    .into_iter()
    .filter(|a| !assets.contains(a))
    .collect();
    tracing::info!(
        "Périmètre straddle mis à jour : {} asset(s) — appliqué au prochain tick (≤ 60 s)",
        assets.len()
    );
    HttpResponse::Ok().json(serde_json::json!({
        "assets": assets,
        "creneaux_armes_hors_perimetre": hors,
    }))
}
