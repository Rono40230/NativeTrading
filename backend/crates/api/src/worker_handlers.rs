//! Endpoints de pilotage des workers d'ingestion — consommés par la vue
//! Données de l'UI (contrôleurs ▶/⏸, timeframes, historique, statut, assets).
//!
//! Toute la configuration vit dans la table `configuration` (clés
//! `worker_*`) — les workers la relisent à chaque session/cycle, aucun
//! redémarrage n'est nécessaire.

use actix_web::{web, HttpResponse, Responder};
use data::worker_config;
use crate::state::AppState;

// ─── GET /api/worker/config ───────────────────────────────────────────────────

/// Snapshot JSON de la config worker courante (lecture seule — le PUT a été
/// retiré le 05/10, étape 7 : la config de collecte est stable et se gère
/// en base).
async fn config_courante(db: &std::sync::Arc<db::Database>) -> serde_json::Value {
    let timeframes = worker_config::lire_timeframes(db).await;
    serde_json::json!({
        "timeframes": timeframes.iter().map(|t| t.as_str()).collect::<Vec<&str>>(),
        "historique_mois": worker_config::lire_historique_mois(db).await,
    })
}

pub async fn get_worker_config(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(config_courante(&state.db).await)
}
