use chrono::{Timelike, Utc};
use db::entrainements::EntrainementRecord;
use db::Database;
use ml::{walk_forward::entrainer_walk_forward, PipelineML};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::utils::{parse_asset, parse_timeframe};

const MIN_BOUGIES_ENTRAINEMENT: i64 = 200;

/// Démarre le scheduler d'entraînement automatique.
/// Si `modele_deja_charge` = false (pas de modèle persisté), lance un entraînement immédiat.
/// Sinon attend 00h00 UTC. Dans les deux cas, re-entraîne toutes les 24h.
pub fn demarrer_scheduler(
    db: Arc<Database>,
    pipeline_ml: Arc<Mutex<PipelineML>>,
    modele_deja_charge: bool,
) {
    tokio::spawn(async move {
        if !modele_deja_charge {
            tracing::info!(
                "🤖 Scheduler ML: aucun modèle persisté — entraînement immédiat au démarrage"
            );
            executer_entrainements_tous(&db, &pipeline_ml).await;
        } else {
            tracing::info!(
                "✅ Scheduler ML: modèle chargé depuis disque — pas d'entraînement immédiat"
            );
        }

        let delai_init = secondes_jusqu_a_minuit_utc();
        tracing::info!(
            "⏰ Scheduler ML: prochain entraînement dans {}h{}m",
            delai_init / 3600,
            (delai_init % 3600) / 60
        );
        sleep(Duration::from_secs(delai_init)).await;

        loop {
            tracing::info!("🤖 Scheduler ML: démarrage entraînement quotidien (tous assets × TF)");
            executer_entrainements_tous(&db, &pipeline_ml).await;
            sleep(Duration::from_secs(86400)).await;
        }
    });
}

/// Itère sur toutes les combinaisons asset × TF disponibles en DB et ré-entraîne le pipeline.
/// Appelé par le scheduler quotidien ET par les retraining manuels (ml_retrain_handler).
pub async fn executer_entrainements_tous(db: &Arc<Database>, pipeline_ml: &Arc<Mutex<PipelineML>>) {
    let combinaisons = match db.combinaisons_entrainables(MIN_BOUGIES_ENTRAINEMENT).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Scheduler ML: impossible de lire les combinaisons: {}", e);
            return;
        }
    };

    if combinaisons.is_empty() {
        tracing::warn!(
            "Scheduler ML: aucune combinaison avec ≥ {} bougies",
            MIN_BOUGIES_ENTRAINEMENT
        );
        return;
    }

    tracing::info!(
        "Scheduler ML: {} combinaison(s) à entraîner",
        combinaisons.len()
    );

    for (asset_str, tf_str) in combinaisons {
        let asset = match parse_asset(&asset_str) {
            Some(a) => a,
            None => {
                tracing::warn!("Scheduler ML: asset inconnu '{}' — ignoré", asset_str);
                continue;
            }
        };
        let timeframe = parse_timeframe(&tf_str);

        let bougies = match db.obtenir_bougies_toutes(&asset, &timeframe).await {
            Ok(b) if b.len() >= MIN_BOUGIES_ENTRAINEMENT as usize => b,
            Ok(b) => {
                tracing::warn!(
                    "Scheduler ML: {}/{} — {} bougies insuffisantes",
                    asset_str,
                    tf_str,
                    b.len()
                );
                continue;
            }
            Err(e) => {
                tracing::error!("Scheduler ML: {}/{} — erreur DB: {}", asset_str, tf_str, e);
                continue;
            }
        };

        let nb_total = bougies.len();
        let debut = std::time::Instant::now();

        // Walk-forward est CPU-intensif (LSTM + XGBoost) — spawn_blocking évite de bloquer le runtime
        let bougies_clone = bougies.clone();
        let wf = match tokio::task::spawn_blocking(move || entrainer_walk_forward(&bougies_clone)).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                tracing::error!(
                    "Scheduler ML: {}/{} — walk-forward échoué: {}",
                    asset_str,
                    tf_str,
                    e
                );
                continue;
            }
            Err(e) => {
                tracing::error!("Scheduler ML: {}/{} — spawn_blocking échoué: {}", asset_str, tf_str, e);
                continue;
            }
        };

        {
            let mut pipeline = pipeline_ml.lock().await;
            if let Err(e) = pipeline.entrainer_sur_historique(&bougies, 5, 0.002) {
                tracing::error!(
                    "Scheduler ML: {}/{} — entraînement échoué: {}",
                    asset_str,
                    tf_str,
                    e
                );
                continue;
            }
        }

        let duree_ms = debut.elapsed().as_millis() as i64;
        let derive = db.detecter_derive_ml(0.60).await.unwrap_or(false);

        let rec = EntrainementRecord {
            asset: asset_str.clone(),
            timeframe: tf_str.clone(),
            nb_bougies: nb_total as i64,
            accuracy_xgb: wf.accuracy_xgb,
            accuracy_lstm: wf.accuracy_lstm,
            accuracy_finale: wf.accuracy_finale,
            accuracy_train: wf.accuracy_train,
            accuracy_val: wf.accuracy_finale,
            duree_ms,
            derive_detectee: derive,
        };

        if let Err(e) = db.inserer_historique_entrainement(&rec).await {
            tracing::error!(
                "Scheduler ML: {}/{} — échec enregistrement: {}",
                asset_str,
                tf_str,
                e
            );
        } else {
            tracing::info!(
                "✅ {}/{} — {}ms | {} bougies | XGB={:.1}% LSTM={:.1}% Finale={:.1}%{}",
                asset_str,
                tf_str,
                duree_ms,
                nb_total,
                wf.accuracy_xgb * 100.0,
                wf.accuracy_lstm * 100.0,
                wf.accuracy_finale * 100.0,
                if derive { " ⚠️ DÉRIVE" } else { "" }
            );
        }
    }
}

/// Démarre la surveillance ML toutes les 6h.
/// Déclenche un ré-entraînement si accuracy_val récente < 52%.
pub fn demarrer_surveillance_ml(db: Arc<Database>, pipeline_ml: Arc<Mutex<PipelineML>>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(6 * 3600)).await;
        loop {
            // Dérive accuracy : ré-entraîner si accuracy < 52%
            match db.accuracy_val_recente(3).await {
                Ok(Some(moy)) if moy < 0.52 => {
                    tracing::warn!(
                        "🔁 Surveillance ML: accuracy_val={:.1}% < 52% — ré-entraînement auto",
                        moy * 100.0
                    );
                    executer_entrainements_tous(&db, &pipeline_ml).await;
                }
                Ok(Some(moy)) => {
                    tracing::debug!("Surveillance ML: accuracy_val={:.1}% ✓", moy * 100.0);
                }
                Ok(None) => tracing::debug!("Surveillance ML: aucun historique disponible"),
                Err(e) => tracing::error!("Surveillance ML: erreur DB: {}", e),
            }
            // Accumulation samples réels : ré-entraîner si ≥100 nouveaux trades dans les 24h
            match db::ml_samples::compter_nouveaux_samples(db.pool(), -24).await {
                Ok(n) if n >= 100 => {
                    tracing::info!(
                        "🔁 Surveillance ML: {} nouveaux samples (24h) ≥ 100 — ré-entraînement incrémental",
                        n
                    );
                    executer_entrainements_tous(&db, &pipeline_ml).await;
                }
                Ok(n) => tracing::debug!("Surveillance ML: {} nouveaux samples (24h)", n),
                Err(e) => tracing::warn!("Surveillance ML: erreur compter samples: {}", e),
            }
            sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
}

fn secondes_jusqu_a_minuit_utc() -> u64 {
    let now = Utc::now();
    let ecoules = now.hour() as u64 * 3600 + now.minute() as u64 * 60 + now.second() as u64;
    let restant = 86400u64.saturating_sub(ecoules);
    if restant == 0 {
        86400
    } else {
        restant
    }
}
