use chrono::{Timelike, Utc};
use db::Database;
use ml::PipelineML;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

use crate::scheduler_execution::executer_entrainements_tous;

static SURVEILLANCE_ML_DEMARREE: AtomicBool = AtomicBool::new(false);
fn marquer_surveillance_demarree() -> bool {
    !SURVEILLANCE_ML_DEMARREE.swap(true, Ordering::SeqCst)
}

/// Démarre le scheduler d'entraînement automatique.
/// Si `modele_deja_charge` = false (pas de modèle persisté), lance un entraînement immédiat.
/// Sinon attend 18h00 heure de Paris. Dans les deux cas, re-entraîne toutes les 24h.
pub fn demarrer_scheduler(
    db: Arc<Database>,
    pipeline_ml: Arc<RwLock<PipelineML>>,
    modele_deja_charge: bool,
) {
    tokio::spawn(async move {
        if !modele_deja_charge {
            tracing::info!(
                "🤖 Scheduler ML: aucun modèle persisté — entraînement immédiat au démarrage"
            );
            executer_entrainements_tous(&db, &pipeline_ml, None).await;
        } else {
            tracing::info!(
                "✅ Scheduler ML: modèle chargé depuis disque — pas d'entraînement immédiat"
            );
        }

        let delai_init = secondes_jusqu_a_18h_paris();
        tracing::info!(
            "⏰ Scheduler ML: prochain entraînement dans {}h{}m",
            delai_init / 3600,
            (delai_init % 3600) / 60
        );
        sleep(Duration::from_secs(delai_init)).await;

        loop {
            tracing::info!("🤖 Scheduler ML: démarrage entraînement quotidien (tous assets × TF)");
            executer_entrainements_tous(&db, &pipeline_ml, None).await;
            // Fine-tuning P3 : Rockets sur trades clôturés (silencieux si < 50 samples)
            crate::ml_retrain_fine_tuning::executer_fine_tuning_rockets(&db, &pipeline_ml).await;
            sleep(Duration::from_secs(86400)).await;
        }
    });
}

/// Démarre la surveillance ML toutes les 6h.
/// Déclenche un ré-entraînement si accuracy_val récente < 52%.
pub fn demarrer_surveillance_ml(db: Arc<Database>, pipeline_ml: Arc<RwLock<PipelineML>>) {
    if !marquer_surveillance_demarree() {
        tracing::warn!("⚠️  Surveillance ML déjà démarrée — second appel ignoré");
        return;
    }
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
                    executer_entrainements_tous(&db, &pipeline_ml, None).await;
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
                    executer_entrainements_tous(&db, &pipeline_ml, None).await;
                }
                Ok(n) => tracing::debug!("Surveillance ML: {} nouveaux samples (24h)", n),
                Err(e) => tracing::warn!("Surveillance ML: erreur compter samples: {}", e),
            }
            sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
}

/// Calcule le délai en secondes jusqu'à 18h00 heure de Paris (Europe/Paris =
/// UTC+1 hiver / UTC+2 été). L'offset DST est résolu via `chrono-tz` (base IANA)
/// à travers le helper unifié `common::time` — fini le DST au mois calendaire.
fn secondes_jusqu_a_18h_paris() -> u64 {
    let now_ts = Utc::now().timestamp();
    let now_paris = common::time::paris_from_unix(now_ts);
    let heure_cible = 18u64 * 3600; // 18h00
    let ecoules =
        now_paris.hour() as u64 * 3600 + now_paris.minute() as u64 * 60 + now_paris.second() as u64;

    if ecoules < heure_cible {
        heure_cible - ecoules
    } else {
        // Déjà passé 18h aujourd'hui → attendre jusqu'à 18h demain
        86400 - ecoules + heure_cible
    }
}
