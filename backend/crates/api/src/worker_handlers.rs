//! Endpoints de pilotage des workers d'ingestion — consommés par la vue
//! Données de l'UI (contrôleurs ▶/⏸, timeframes, historique, statut, assets).
//!
//! Toute la configuration vit dans la table `configuration` (clés
//! `worker_*`) — les workers la relisent à chaque session/cycle, aucun
//! redémarrage n'est nécessaire.

use actix_web::{web, HttpResponse, Responder};
use common::Timeframe;
use data::worker_config;
use data::worker_status;
use serde::Deserialize;

use crate::state::AppState;

/// Corps accepté par `PUT /api/worker/config` — tous les champs sont
/// optionnels : seules les clés présentes sont mises à jour.
#[derive(Deserialize)]
pub struct MiseAJourWorkerConfig {
    /// Timeframes communs aux workers (ex: ["M5","H1"]).
    pub timeframes: Option<Vec<String>>,
    /// Profondeur d'historique en mois (bornée 1..=24).
    pub historique_mois: Option<i64>,
    /// Interrupteur du worker Bybit WS.
    pub actif_bybit: Option<bool>,
}

/// Snapshot JSON de la config worker courante.
async fn config_courante(db: &std::sync::Arc<db::Database>) -> serde_json::Value {
    let timeframes = worker_config::lire_timeframes(db).await;
    serde_json::json!({
        "timeframes": timeframes.iter().map(|t| t.as_str()).collect::<Vec<&str>>(),
        "historique_mois": worker_config::lire_historique_mois(db).await,
        "actif_bybit": worker_config::lire_actif(db, worker_config::CLE_ACTIF_BYBIT).await,
    })
}

// ─── GET /api/worker/config ───────────────────────────────────────────────────

pub async fn get_worker_config(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(config_courante(&state.db).await)
}

// ─── PUT /api/worker/config ───────────────────────────────────────────────────

pub async fn put_worker_config(
    state: web::Data<AppState>,
    body: web::Json<MiseAJourWorkerConfig>,
) -> impl Responder {
    // Timeframes : validation stricte — une valeur inconnue est rejetée en
    // bloc (400) plutôt que silencieusement ignorée.
    if let Some(tfs_bruts) = &body.timeframes {
        let mut tfs = Vec::with_capacity(tfs_bruts.len());
        for tf_str in tfs_bruts {
            match Timeframe::try_from(tf_str.as_str()) {
                Ok(tf) => tfs.push(tf),
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "erreur": format!("Timeframe inconnu: {}", tf_str)
                    }));
                }
            }
        }
        if tfs.is_empty() {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "erreur": "Au moins un timeframe est requis" }));
        }
        if let Err(e) = state
            .db
            .ecrire_config(
                worker_config::CLE_TIMEFRAMES,
                &worker_config::serialise_timeframes(&tfs),
            )
            .await
        {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "erreur": e.to_string() }));
        }
        tracing::info!("Config worker mise à jour: timeframes = {:?}", tfs_bruts);
    }

    // Historique : borné 1..=24 mois (quota providers + taille des requêtes).
    if let Some(mois) = body.historique_mois {
        let borne = mois.clamp(1, 24);
        if let Err(e) = state
            .db
            .ecrire_config(worker_config::CLE_HISTORIQUE_MOIS, &borne.to_string())
            .await
        {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "erreur": e.to_string() }));
        }
        tracing::info!("Config worker mise à jour: historique_mois = {}", borne);
    }

    // Interrupteur Bybit ("1"/"0" en DB).
    for (flag, cle, label) in [(
        body.actif_bybit,
        worker_config::CLE_ACTIF_BYBIT,
        "bybit",
    )] {
        if let Some(valeur) = flag {
            if let Err(e) = state
                .db
                .ecrire_config(cle, if valeur { "1" } else { "0" })
                .await
            {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "erreur": e.to_string() }));
            }
            tracing::info!("Config worker mise à jour: actif_{} = {}", label, valeur);
        }
    }

    // Réponse = config effective après écriture (les bornes sont visibles).
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


