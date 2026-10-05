//! Endpoints de pilotage des workers d'ingestion — consommés par la vue
//! Données de l'UI (contrôleurs ▶/⏸, timeframes, historique, statut, assets).
//!
//! Toute la configuration vit dans la table `configuration` (clés
//! `worker_*`) — les workers la relisent à chaque session/cycle, aucun
//! redémarrage n'est nécessaire.

use actix_web::{web, HttpResponse, Responder};
use data::worker_config;
use data::worker_status;
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
        "actif_bybit": worker_config::lire_actif(db, worker_config::CLE_ACTIF_BYBIT).await,
    })
}

pub async fn get_worker_config(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(config_courante(&state.db).await)
}


// ─── GET /api/worker/status ───────────────────────────────────────────────────

/// Statut runtime + routing des workers : interrupteurs (config), connexion,
/// nombre d'actifs couverts, dernière bougie insérée. Les timestamps Unix
/// nuls sont renvoyés en `null` (jamais connecté / aucune bougie).
pub async fn get_worker_status(state: web::Data<AppState>) -> impl Responder {
    let db = &state.db;

    // Compteurs de routing depuis la DB (indépendants de l'état des workers).
    let mut nb_bybit = 0u64;
    match db.lister_assets_worker().await {
        Ok(assets) => {
            for a in &assets {
                if !a.actif {
                    continue;
                }
                if a.source == "binance" && a.symbol_bybit.is_some() {
                    nb_bybit += 1;
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "erreur": e.to_string() }));
        }
    }

    let ts_option = |ts: i64| if ts > 0 { serde_json::json!(ts) } else { serde_json::json!(null) };
    let bybit = worker_status::STATUT_BYBIT.instantane();

    HttpResponse::Ok().json(serde_json::json!({
        "bybit": {
            "actif": worker_config::lire_actif(db, worker_config::CLE_ACTIF_BYBIT).await,
            "connecte": bybit.connecte,
            "nb_assets": nb_bybit,
            "nb_assets_session": bybit.nb_assets,
            "derniere_connexion": ts_option(bybit.derniere_connexion),
            "derniere_bougie": ts_option(bybit.derniere_bougie),
            "bougies_inserees": bybit.bougies_inserees,
        },
    }))
}


