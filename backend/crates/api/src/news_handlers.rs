use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use news::news_traduction::{traduire_avec_cache, traduire_contenu};
use crate::state::AppState;

// ── Traduction à la demande ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TraductionParams {
    pub texte: String,
    #[serde(default)]
    pub long: bool,
}

#[derive(Serialize)]
pub struct TraductionReponse {
    pub texte_fr: String,
}

/// GET /api/news/traduire?texte=...&long=true
/// Dégradation silencieuse : retourne le texte original si Ollama est absent.
pub async fn get_traduire(
    state: web::Data<AppState>,
    params: web::Query<TraductionParams>,
) -> impl Responder {
    let texte_fr = if params.long {
        traduire_contenu(&params.texte).await
    } else {
        traduire_avec_cache(state.db.pool(), &params.texte).await
    };
    HttpResponse::Ok().json(TraductionReponse { texte_fr })
}
