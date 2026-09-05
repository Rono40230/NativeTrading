//! Journal de bord du propriétaire — endpoints (§16 roadmap).
//! Fil de notes horodatées attachées aux trades : contexte à l'ouverture,
//! ressenti en cours, verdict à la clôture. Append-only (suppression
//! d'entrée possible). Matière première future de l'analyse IA.

use crate::state::AppState;
use actix_web::{web, HttpResponse};
use serde::Deserialize;

/// GET /api/journal/comptes → { signal_id: nb_notes } (badges historique).
pub async fn get_comptes(state: web::Data<AppState>) -> impl actix_web::Responder {
    match state.db.journal_comptes().await {
        Ok(comptes) => {
            let map: serde_json::Map<String, serde_json::Value> = comptes
                .into_iter()
                .map(|(id, n)| (id, serde_json::json!(n)))
                .collect();
            HttpResponse::Ok().json(serde_json::Value::Object(map))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/journal/{signal_id} → fil du trade (chrono croissant).
pub async fn get_journal(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    match state.db.journal_du_signal(&path.into_inner()).await {
        Ok(entrees) => HttpResponse::Ok().json(serde_json::json!({ "entrees": entrees })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct CorpsNote {
    pub texte: String,
}

/// POST /api/journal/{signal_id} — ajoute une entrée (max 2 000 caractères,
/// blancs purs rejetés).
pub async fn post_note(
    state: web::Data<AppState>,
    path: web::Path<String>,
    corps: web::Json<CorpsNote>,
) -> impl actix_web::Responder {
    let signal_id = path.into_inner();
    let texte = corps.texte.trim().to_string();
    if texte.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Note vide" }));
    }
    let texte: String = texte.chars().take(2000).collect();
    match state
        .db
        .ajouter_note_journal(&signal_id, &texte, chrono::Utc::now().timestamp())
        .await
    {
        Ok(e) => HttpResponse::Ok().json(serde_json::json!({ "entree": e })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// DELETE /api/journal/entree/{id} — supprime une entrée.
pub async fn supprimer_note(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> impl actix_web::Responder {
    match state.db.supprimer_note_journal(path.into_inner()).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Ok(false) => {
            HttpResponse::NotFound().json(serde_json::json!({ "error": "Entrée inconnue" }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}
