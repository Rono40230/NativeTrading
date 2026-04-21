use std::sync::Arc;
use tokio::sync::Mutex;
use ml::PipelineML;
use db::Database;

/// Fine-tuning XGBoost Straddle sur les trades clôturés (P13).
/// Silencieux si < 50 samples disponibles.
pub(crate) async fn executer_fine_tuning_straddle(
    db: &Arc<Database>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
) {
    let samples = match db::straddle_features::lire_snapshots_avec_labels(db.pool()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Fine-tuning Straddle: lecture snapshots échouée: {}", e);
            return;
        }
    };

    let nb = samples.len();
    let resultat = match tokio::task::spawn_blocking(move || {
        ml::straddle_trainer::entrainer_sur_trades_clotures(&samples)
    })
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => { tracing::error!("Fine-tuning Straddle: erreur entraînement: {}", e); return; }
        Err(e) => { tracing::error!("Fine-tuning Straddle: spawn_blocking échoué: {}", e); return; }
    };

    match resultat {
        None => {
            tracing::info!("Fine-tuning Straddle: {} samples < 50 — ignoré", nb);
            inserer_importances_defaut(db.pool(), "straddle", &["ratio_atr", "straddle_categorie", "straddle_session", "score_llm", "rendement_1", "volume_rel", "range_rel", "corps_rel", "rsi14", "atr14_rel"]).await;
        },
        Some(r) => {
            tracing::info!("Fine-tuning Straddle: {} samples | OOS={:.1}% | sauvegardé={}", r.nb_samples, r.accuracy_oos * 100.0, r.sauvegarde);
            inserer_importances_defaut(db.pool(), "straddle", &["ratio_atr", "straddle_categorie", "straddle_session", "score_llm", "rendement_1", "volume_rel", "range_rel", "corps_rel", "rsi14", "atr14_rel"]).await;
            if r.sauvegarde {
                let mut pipeline = pipeline_ml.lock().await;
                pipeline.xgb_straddle = ml::XgbStraddle::charger_depuis_disque();
            }
        }
    }
}

/// Fine-tuning XGBoost SMC sur les trades clôturés (P13).
/// Silencieux si < 50 samples disponibles.
pub(crate) async fn executer_fine_tuning_smc(
    db: &Arc<Database>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
) {
    let samples = match db::smc_features::lire_snapshots_avec_labels(db.pool()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Fine-tuning SMC: lecture snapshots échouée: {}", e);
            return;
        }
    };

    let nb = samples.len();
    let resultat = match tokio::task::spawn_blocking(move || {
        ml::smc_trainer::entrainer_sur_trades_clotures(&samples)
    })
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => { tracing::error!("Fine-tuning SMC: erreur entraînement: {}", e); return; }
        Err(e) => { tracing::error!("Fine-tuning SMC: spawn_blocking échoué: {}", e); return; }
    };

    match resultat {
        None => {
            tracing::info!("Fine-tuning SMC: {} samples < 50 — ignoré", nb);
            inserer_importances_defaut(db.pool(), "smc", &["smc_tendance", "smc_order_block", "smc_ifvg", "smc_fibonacci", "smc_imbalance", "smc_kill_zone", "smc_sweep", "rendement_1", "volume_rel", "rsi14"]).await;
        },
        Some(r) => {
            tracing::info!(
                "Fine-tuning SMC: {} samples | OOS={:.1}% | sauvegardé={}",
                r.nb_samples, r.accuracy_oos * 100.0, r.sauvegarde
            );
            inserer_importances_defaut(db.pool(), "smc", &["smc_tendance", "smc_order_block", "smc_ifvg", "smc_fibonacci", "smc_imbalance", "smc_kill_zone", "smc_sweep", "rendement_1", "volume_rel", "rsi14"]).await;
            if r.sauvegarde {
                let mut pipeline = pipeline_ml.lock().await;
                pipeline.xgb_smc = ml::XgbSmc::charger_depuis_disque();
            }
        }
    }
}

/// Fine-tuning XGBoost Rockets sur les trades clôturés (P3).
/// Silencieux si < 50 samples disponibles.
pub(crate) async fn executer_fine_tuning_rockets(
    db: &Arc<Database>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
) {
    let samples = match db::rockets_features::lire_snapshots_avec_labels(db.pool()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Fine-tuning Rockets: lecture snapshots échouée: {}", e);
            return;
        }
    };

    let nb = samples.len();
    let resultat = match tokio::task::spawn_blocking(move || {
        ml::entrainer_sur_trades_clotures(&samples)
    })
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => { tracing::error!("Fine-tuning Rockets: erreur entraînement: {}", e); return; }
        Err(e) => { tracing::error!("Fine-tuning Rockets: spawn_blocking échoué: {}", e); return; }
    };

    match resultat {
        None => tracing::info!("Fine-tuning Rockets: {} samples < 50 — ignoré", nb),
        Some(r) => {
            tracing::info!(
                "Fine-tuning Rockets: {} samples | OOS={:.1}% | sauvegardé={}",
                r.nb_samples,
                r.accuracy_oos * 100.0,
                r.sauvegarde
            );

            // P4 : persister les importances de features en DB
            if !r.importances.is_empty() {
                let fis: Vec<db::ml_feature_importance::FeatureImportance> = r.importances.iter()
                    .map(|fi| db::ml_feature_importance::FeatureImportance {
                        feature_idx: fi.feature_idx as i64,
                        feature_nom: fi.feature_nom.to_string(),
                        importance: fi.importance,
                    })
                    .collect();
                match db::ml_feature_importance::inserer_importances(db.pool(), "rockets", &fis).await {
                    Err(e) => tracing::warn!("Fine-tuning Rockets: importances: {}", e),
                    Ok(_) => tracing::info!("top feature = {} ({:.4})", r.importances[0].feature_nom, r.importances[0].importance),
                }
            }

            // Recharger le modèle Rockets dans le pipeline si sauvegardé
            if r.sauvegarde {
                let mut pipeline = pipeline_ml.lock().await;
                pipeline.xgb_rockets = ml::XgbRockets::charger_depuis_disque();
            }
        }
    }
}

async fn inserer_importances_defaut(
    pool: &sqlx::SqlitePool,
    strategie: &str,
    noms: &[&str],
) {
    let fis: Vec<db::ml_feature_importance::FeatureImportance> = noms
        .iter()
        .enumerate()
        .map(|(i, &nom)| db::ml_feature_importance::FeatureImportance {
            feature_idx: i as i64,
            feature_nom: nom.to_string(),
            importance: 0.0,
        })
        .collect();
    let _ = db::ml_feature_importance::inserer_importances(pool, strategie, &fis).await;
}
