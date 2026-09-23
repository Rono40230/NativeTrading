//! Phase 8.4 — Réentraînement incrémental à la demande.
//!
//! Le déclenchement manuel (POST /api/ml/retrain + bouton) a été retiré le
//! 23/09 : `boucle_automatique` relance un entraînement chaque fois que le
//! dernier date de plus de 7 jours. Reste en endpoint :
//! - GET /api/ml/retrain/status/{job_id} → statut du job en cours
//!
//! Logique rollback : les fichiers modèle sont sauvegardés avant le
//! réentraînement ; si l'accuracy finale est inférieure de plus de 2 pts
//! par rapport à la baseline (moyenne des 3 derniers entraînements), les
//! anciens modèles sont restaurés.

use actix_web::{web, HttpResponse};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::state::AppState;

// ── État du job de réentraînement ─────────────────────────────────────────────

/// Cycle de vie d'un job de réentraînement (stocké en mémoire dans AppState).
#[derive(Debug, Clone, Serialize, Default)]
pub struct RetainState {
    pub job_id: Option<String>,
    pub en_cours: bool,
    pub accuracy_avant: f64,
    pub accuracy_apres: Option<f64>,
    /// Score walk-forward OOS du dernier entraînement
    pub wf_score_apres: Option<f64>,
    /// Écart accuracy_train − accuracy_val_oos (>15% = overfitting)
    pub gap_train_wf: Option<f64>,
    pub overfitting: bool,
    pub rolled_back: bool,
    pub message: String,
    pub demarre_le: Option<i64>,
    pub termine_le: Option<i64>,
    /// Nombre total de combinaisons asset×TF à entraîner
    pub nb_combinaisons_total: usize,
    /// Nombre de combinaisons terminées (succès ou ignorées)
    pub nb_combinaisons_done: usize,
    /// Combinaison en cours (ex: "BTCUSD/H1") — vide si aucun job actif
    pub combinaison_en_cours: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Entraînement AUTOMATIQUE (23/09, remplace le bouton manuel — décision
/// propriétaire) : vérification horaire ; si le dernier entraînement date
/// de plus de 7 jours (et qu'aucun job ne tourne), un job est lancé. Les
/// samples croissent au fil des clôtures (~quelques trades/jour) — un
/// rythme hebdomadaire laisse à la donnée le temps de changer.
const VERIF_SEC: u64 = 3600;
const STALE_SEC: i64 = 7 * 86400;

pub async fn boucle_automatique(state: web::Data<AppState>) {
    tokio::time::sleep(std::time::Duration::from_secs(180)).await;
    loop {
        if !state.retrain_state.read().await.en_cours {
            let dernier: Option<i64> = sqlx::query_scalar(
                "SELECT MAX(cree_le) FROM historique_entrainements",
            )
            .fetch_optional(state.db.pool())
            .await
            .ok()
            .flatten();
            let maintenant = chrono::Utc::now().timestamp();
            let obsolete = dernier.map_or(true, |d| maintenant - d > STALE_SEC);
            if obsolete {
                tracing::info!("🔁 ML : entraînement automatique (dernier > 7 j)");
                lancer_si_libre(&state).await;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(VERIF_SEC)).await;
    }
}

/// Lance un réentraînement en arrière-plan si aucun job n'est en cours.
/// Retourne false (et ne fait rien) si un job tourne déjà.
async fn lancer_si_libre(state: &web::Data<AppState>) -> bool {
    // Vérification : un seul job à la fois
    {
        let s = state.retrain_state.read().await;
        if s.en_cours {
            return false;
        }
    }

    // Baseline accuracy (moyenne des 3 derniers entraînements)
    let accuracy_avant: f64 = state
        .db
        .accuracy_val_recente(3)
        .await
        .ok()
        .flatten()
        .unwrap_or(0.0);

    // Générer un job_id simple (timestamp ms)
    let job_id = chrono::Utc::now().timestamp_millis().to_string();
    let now_ts = chrono::Utc::now().timestamp();

    {
        let mut s = state.retrain_state.write().await;
        *s = RetainState {
            job_id: Some(job_id.clone()),
            en_cours: true,
            accuracy_avant,
            accuracy_apres: None,
            wf_score_apres: None,
            gap_train_wf: None,
            overfitting: false,
            rolled_back: false,
            message: "Réentraînement en cours…".to_string(),
            demarre_le: Some(now_ts),
            termine_le: None,
            nb_combinaisons_total: 0,
            nb_combinaisons_done: 0,
            combinaison_en_cours: String::new(),
        };
    }

    // Sauvegarder les modèles actuels (rollback possible)
    if let Err(e) = crate::ml_retrain_job::sauvegarder_backup() {
        let mut s = state.retrain_state.write().await;
        s.en_cours = false;
        s.message = format!("Impossible de sauvegarder les modèles avant entraînement: {e}");
        return false;
    }

    // Lancer le job en background
    let db = state.db.clone();
    let pipeline_ml = state.pipeline_ml.clone();
    let retrain_state = state.retrain_state.clone();
    let jid = job_id.clone();

    tokio::spawn(async move {
        executer_retrain_job(db, pipeline_ml, retrain_state, accuracy_avant, jid).await;
    });

    true
}

/// GET /api/ml/retrain/status/{job_id}
/// Retourne l'état actuel du dernier job (le job_id est vérifié pour cohérence).
pub async fn statut_retrain(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let job_id = path.into_inner();
    let s = state.retrain_state.read().await;

    match &s.job_id {
        Some(id) if id == &job_id => HttpResponse::Ok().json(&*s),
        Some(_) | None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Job inconnu ou expiré"
        })),
    }
}

// ── Logique interne ───────────────────────────────────────────────────────────

/// Corps du job de réentraînement : délégué à ml_retrain_job.
async fn executer_retrain_job(
    db: Arc<db::Database>,
    pipeline_ml: Arc<tokio::sync::RwLock<ml::PipelineML>>,
    retrain_state: Arc<RwLock<RetainState>>,
    accuracy_avant: f64,
    job_id: String,
) {
    crate::ml_retrain_job::executer_retrain_job(
        db,
        pipeline_ml,
        retrain_state,
        accuracy_avant,
        job_id,
    )
    .await;
}

// ── GET /api/ml/feature-importance/{strategie} ───────────────────────────────

/// GET /api/ml/feature-importance/{strategie}
/// Retourne le top 10 des features les plus prédictives pour une stratégie.
pub async fn feature_importance(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let strategie = path.into_inner();
    match db::ml_feature_importance::lire_top_importances(state.db.pool(), &strategie, 10).await {
        Ok(items) => {
            let json: Vec<_> = items
                .iter()
                .map(|fi| {
                    serde_json::json!({
                        "feature_idx": fi.feature_idx,
                        "feature_nom": fi.feature_nom,
                        "importance":  fi.importance,
                    })
                })
                .collect();
            HttpResponse::Ok().json(json)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Lecture feature importance: {}", e)
        })),
    }
}

// ── GET /api/ml/retrain/last ──────────────────────────────────────────────────

/// GET /api/ml/retrain/last — Retourne l'état du dernier job sans connaître l'ID
pub async fn dernier_statut_retrain(state: web::Data<AppState>) -> HttpResponse {
    let s = state.retrain_state.read().await;
    if s.job_id.is_none() {
        return HttpResponse::Ok().json(serde_json::json!({ "job_id": null }));
    }
    HttpResponse::Ok().json(&*s)
}
