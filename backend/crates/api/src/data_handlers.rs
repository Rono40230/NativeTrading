//! Gestion des données historiques — collecte bulk + couverture
use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;


// ─── GET /api/data/coverage ───────────────────────────────────────────────────

/// Retourne la couverture de données stockées par asset × timeframe,
/// avec la taille actuelle de la base (PRAGMA page_count × page_size).
pub async fn get_coverage(state: web::Data<AppState>) -> impl Responder {
    let taille_db: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
    )
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    // Bougies reçues depuis minuit Paris — compteur de flux journalier
    // (la couverture % et la taille DB sont des jauges de fonds, muettes
    // à l'échelle du jour).
    let maintenant = chrono::Utc::now().timestamp();
    let minuit_paris =
        maintenant - ((maintenant + common::time::offset_paris_seconds(maintenant)) % 86_400);
    let bougies_auj: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM bougies WHERE timestamp >= ?1",
    )
    .bind(minuit_paris)
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    match state.db.obtenir_couverture_donnees().await {
        Ok(data) => HttpResponse::Ok().json(serde_json::json!({
            "couverture": data,
            "taille_db_octets": taille_db,
            "bougies_aujourd_hui": bougies_auj,
        })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "erreur": e.to_string() }))
        }
    }
}

// ─── POST /api/data/collect ───────────────────────────────────────────────────


