//! Phase 8.4 — Réentraînement incrémental à la demande.
//!
//! Fournit deux endpoints :
//! - POST /api/ml/retrain     → Lance un job de réentraînement en arrière-plan
//! - GET  /api/ml/retrain/status/{job_id} → Statut du dernier job
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

const CHEMIN_XGB: &str = "data/modele_xgboost.json";
const CHEMIN_LSTM: &str = "data/modele_lstm.json";
const CHEMIN_XGB_BACKUP: &str = "data/modele_xgboost_backup.json";
const CHEMIN_LSTM_BACKUP: &str = "data/modele_lstm_backup.json";

// ── État du job de réentraînement ─────────────────────────────────────────────

/// Cycle de vie d'un job de réentraînement (stocké en mémoire dans AppState).
#[derive(Debug, Clone, Serialize, Default)]
pub struct RetainState {
    pub job_id: Option<String>,
    pub en_cours: bool,
    pub accuracy_avant: f64,
    pub accuracy_apres: Option<f64>,
    pub rolled_back: bool,
    pub message: String,
    pub demarre_le: Option<i64>,
    pub termine_le: Option<i64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/ml/retrain
/// Lance un réentraînement en background. Retourne 409 si un job est déjà en cours.
pub async fn declencher_retrain(state: web::Data<AppState>) -> HttpResponse {
    // Vérification : un seul job à la fois
    {
        let s = state.retrain_state.read().await;
        if s.en_cours {
            return HttpResponse::Conflict().json(serde_json::json!({
                "error": "Un réentraînement est déjà en cours",
                "job_id": s.job_id
            }));
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
            rolled_back: false,
            message: "Réentraînement en cours…".to_string(),
            demarre_le: Some(now_ts),
            termine_le: None,
        };
    }

    // Sauvegarder les modèles actuels (rollback possible)
    if let Err(e) = sauvegarder_backup() {
        let mut s = state.retrain_state.write().await;
        s.en_cours = false;
        s.message = format!("Impossible de sauvegarder les modèles avant entraînement: {e}");
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": s.message.clone()
        }));
    }

    // Lancer le job en background
    let db = state.db.clone();
    let pipeline_ml = state.pipeline_ml.clone();
    let retrain_state = state.retrain_state.clone();
    let jid = job_id.clone();

    tokio::spawn(async move {
        executer_retrain_job(db, pipeline_ml, retrain_state, accuracy_avant, jid).await;
    });

    HttpResponse::Accepted().json(serde_json::json!({
        "job_id": job_id,
        "status": "started"
    }))
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

/// Corps du job de réentraînement : entraîne, compare, rollback si besoin.
async fn executer_retrain_job(
    db: Arc<db::Database>,
    pipeline_ml: Arc<tokio::sync::Mutex<ml::PipelineML>>,
    retrain_state: Arc<RwLock<RetainState>>,
    accuracy_avant: f64,
    job_id: String,
) {
    tracing::info!("🔁 Réentraînement manuel déclenché (job_id={})", job_id);

    // Lancer le réentraînement complet (même logique que le scheduler)
    crate::scheduler::executer_entrainements_tous(&db, &pipeline_ml).await;

    // Mesurer la nouvelle accuracy (moyenne sur le dernier entraînement)
    let accuracy_apres: f64 = db
        .accuracy_val_recente(1)
        .await
        .ok()
        .flatten()
        .unwrap_or(0.0);
    let seuil_rollback = accuracy_avant - 0.02;

    let now_ts = chrono::Utc::now().timestamp();

    if accuracy_avant > 0.0 && accuracy_apres < seuil_rollback {
        // --- ROLLBACK ---
        tracing::warn!(
            "Réentraînement dégradé ({:.3} → {:.3}) — rollback",
            accuracy_avant,
            accuracy_apres
        );

        let rolled_back = match restaurer_backup() {
            Ok(_) => {
                let mut pipeline = pipeline_ml.lock().await;
                match pipeline.charger_depuis_disque() {
                    Ok(_) => {
                        tracing::info!("Rollback modèles ML effectué avec succès");
                        true
                    }
                    Err(e) => {
                        tracing::error!("Rollback charger_depuis_disque échoué: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                tracing::error!("Rollback copie fichiers échoué: {}", e);
                false
            }
        };

        let mut s = retrain_state.write().await;
        s.en_cours = false;
        s.accuracy_apres = Some(accuracy_apres);
        s.rolled_back = rolled_back;
        s.termine_le = Some(now_ts);
        s.message = if rolled_back {
            format!(
                "Dégradation détectée ({:.1}% → {:.1}%) — anciens modèles restaurés",
                accuracy_avant * 100.0,
                accuracy_apres * 100.0
            )
        } else {
            format!(
                "Dégradation détectée ({:.1}% → {:.1}%) — rollback échoué, modèles dégradés conservés",
                accuracy_avant * 100.0,
                accuracy_apres * 100.0
            )
        };
    } else {
        // --- SUCCÈS ---
        let gain = accuracy_apres - accuracy_avant;
        tracing::info!(
            "Réentraînement terminé: {:.3} → {:.3} (Δ{:+.3})",
            accuracy_avant,
            accuracy_apres,
            gain
        );

        let mut s = retrain_state.write().await;
        s.en_cours = false;
        s.accuracy_apres = Some(accuracy_apres);
        s.rolled_back = false;
        s.termine_le = Some(now_ts);
        s.message = format!(
            "Réentraînement terminé : {:.1}% → {:.1}% (Δ{:+.1}%)",
            accuracy_avant * 100.0,
            accuracy_apres * 100.0,
            gain * 100.0
        );
    }
}

/// Copie les modèles actuels vers les chemins de backup.
fn sauvegarder_backup() -> anyhow::Result<()> {
    if std::path::Path::new(CHEMIN_XGB).exists() {
        std::fs::copy(CHEMIN_XGB, CHEMIN_XGB_BACKUP)?;
    }
    if std::path::Path::new(CHEMIN_LSTM).exists() {
        std::fs::copy(CHEMIN_LSTM, CHEMIN_LSTM_BACKUP)?;
    }
    Ok(())
}

/// Restaure les modèles depuis les backups.
fn restaurer_backup() -> anyhow::Result<()> {
    if std::path::Path::new(CHEMIN_XGB_BACKUP).exists() {
        std::fs::copy(CHEMIN_XGB_BACKUP, CHEMIN_XGB)?;
    }
    if std::path::Path::new(CHEMIN_LSTM_BACKUP).exists() {
        std::fs::copy(CHEMIN_LSTM_BACKUP, CHEMIN_LSTM)?;
    }
    Ok(())
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
