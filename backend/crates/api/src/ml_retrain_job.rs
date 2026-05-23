//! Corps du job de réentraînement ML : logique extraite de ml_retrain_handler.rs.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ml_retrain_handler::RetainState;
use crate::ml_retrain_fine_tuning::{executer_fine_tuning_rockets, executer_fine_tuning_straddle, executer_fine_tuning_smc};

const CHEMIN_XGB: &str = "data/modele_xgboost.json";
const CHEMIN_LSTM: &str = "data/modele_lstm.json";
const CHEMIN_XGB_BACKUP: &str = "data/modele_xgboost_backup.json";
const CHEMIN_LSTM_BACKUP: &str = "data/modele_lstm_backup.json";

/// Corps du job de réentraînement : entraîne, compare, rollback si besoin.
pub(crate) async fn executer_retrain_job(
    db: Arc<db::Database>,
    pipeline_ml: Arc<tokio::sync::RwLock<ml::PipelineML>>,
    retrain_state: Arc<RwLock<RetainState>>,
    accuracy_avant: f64,
    job_id: String,
) {
    tracing::info!("🔁 Réentraînement manuel déclenché (job_id={})", job_id);

    // Lancer le réentraînement complet avec suivi de progression
    crate::scheduler_execution::executer_entrainements_tous(&db, &pipeline_ml, Some(retrain_state.clone())).await;

    // Fine-tuning P3 : XGBoost Rockets sur trades clôturés (garde-fou 50 samples intégré)
    executer_fine_tuning_rockets(&db, &pipeline_ml).await;

    // Fine-tuning P13 : XGBoost Straddle sur trades clôturés
    executer_fine_tuning_straddle(&db, &pipeline_ml).await;

    // Fine-tuning P13 : XGBoost SMC sur trades clôturés
    executer_fine_tuning_smc(&db, &pipeline_ml).await;

    // Mesurer la nouvelle accuracy OOS (moyenne sur le dernier entraînement)
    let accuracy_apres: f64 = db
        .accuracy_val_recente(1)
        .await
        .ok()
        .flatten()
        .unwrap_or(0.0);
    let seuil_rollback = accuracy_avant - 0.02;

    // Calculer le gap train/OOS pour détecter l'overfitting
    let (gap_train_wf, overfitting) = match db.dernier_gap_train_val().await {
        Ok(Some((train, val))) => {
            let gap = train - val;
            tracing::info!(
                "Gap train/OOS: {:.1}% (train={:.1}% OOS={:.1}%)",
                gap * 100.0,
                train * 100.0,
                val * 100.0
            );
            if gap > 0.15 {
                tracing::warn!("Overfitting détecté (gap={:.1}%) → rollback", gap * 100.0);
            }
            (Some(gap), gap > 0.15)
        }
        _ => (None, false),
    };

    let now_ts = chrono::Utc::now().timestamp();

    if (accuracy_avant > 0.0 && accuracy_apres < seuil_rollback) || overfitting {
        // --- ROLLBACK ---
        tracing::warn!("Réentraînement dégradé ({:.3} → {:.3}) — rollback", accuracy_avant, accuracy_apres);

        let rolled_back = match restaurer_backup() {
            Ok(_) => {
                let mut pipeline = pipeline_ml.write().await;
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
        s.wf_score_apres = Some(accuracy_apres);
        s.gap_train_wf = gap_train_wf;
        s.overfitting = overfitting;
        s.rolled_back = rolled_back;
        s.termine_le = Some(now_ts);
        s.message = if rolled_back {
            if overfitting {
                format!(
                    "Overfitting détecté (gap={:.1}%) — anciens modèles restaurés",
                    gap_train_wf.unwrap_or(0.0) * 100.0
                )
            } else {
                format!(
                    "Dégradation détectée ({:.1}% → {:.1}%) — anciens modèles restaurés",
                    accuracy_avant * 100.0,
                    accuracy_apres * 100.0
                )
            }
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
        s.wf_score_apres = Some(accuracy_apres);
        s.gap_train_wf = gap_train_wf;
        s.overfitting = false;
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
pub(crate) fn sauvegarder_backup() -> anyhow::Result<()> {
    if std::path::Path::new(CHEMIN_XGB).exists() {
        std::fs::copy(CHEMIN_XGB, CHEMIN_XGB_BACKUP)?;
    }
    if std::path::Path::new(CHEMIN_LSTM).exists() {
        std::fs::copy(CHEMIN_LSTM, CHEMIN_LSTM_BACKUP)?;
    }
    Ok(())
}

/// Restaure les modèles depuis les backups.
pub(crate) fn restaurer_backup() -> anyhow::Result<()> {
    if std::path::Path::new(CHEMIN_XGB_BACKUP).exists() {
        std::fs::copy(CHEMIN_XGB_BACKUP, CHEMIN_XGB)?;
    }
    if std::path::Path::new(CHEMIN_LSTM_BACKUP).exists() {
        std::fs::copy(CHEMIN_LSTM_BACKUP, CHEMIN_LSTM)?;
    }
    Ok(())
}
