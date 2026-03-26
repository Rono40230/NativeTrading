use chrono::{Timelike, Utc};
use common::Asset;
use db::{entrainements::EntrainementRecord, Database};
use ml::{walk_forward::entrainer_walk_forward, PipelineML};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::utils::parse_timeframe;

/// Démarre le scheduler d'entraînement automatique.
/// Attend 00h00 UTC puis ré-entraîne toutes les 24h.
pub fn demarrer_scheduler(db: Arc<Database>, pipeline_ml: Arc<Mutex<PipelineML>>) {
    tokio::spawn(async move {
        // Attente jusqu'au prochain 00h00 UTC
        let delai_init = secondes_jusqu_a_minuit_utc();
        tracing::info!(
            "⏰ Scheduler ML: prochain entraînement dans {}h{}m",
            delai_init / 3600,
            (delai_init % 3600) / 60
        );
        sleep(Duration::from_secs(delai_init)).await;

        loop {
            tracing::info!("🤖 Scheduler ML: démarrage entraînement quotidien");
            executer_entrainement_auto(&db, &pipeline_ml).await;
            sleep(Duration::from_secs(86400)).await;
        }
    });
}

async fn executer_entrainement_auto(db: &Arc<Database>, pipeline_ml: &Arc<Mutex<PipelineML>>) {
    let asset = Asset::BTC;
    let timeframe = parse_timeframe("M15");
    // 3 mois de bougies M15 ≈ 8 640 bougies
    let limit = 8640i64;

    let bougies = match db.obtenir_bougies(&asset, &timeframe, limit).await {
        Ok(b) if b.len() >= 200 => b,
        Ok(b) => {
            tracing::warn!(
                "Scheduler ML: données insuffisantes ({} bougies, min 200)",
                b.len()
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                "Scheduler ML: erreur DB lors de la lecture des bougies: {}",
                e
            );
            return;
        }
    };

    let nb_total = bougies.len();
    let debut = std::time::Instant::now();

    // ── Walk-forward (métriques out-of-sample) ────────────────────────────────
    let wf = match entrainer_walk_forward(&bougies) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Scheduler ML: walk-forward échoué: {}", e);
            return;
        }
    };

    // ── Entraînement pipeline principal sur 100 % des données ─────────────────
    let mut pipeline = pipeline_ml.lock().await;
    if let Err(e) = pipeline.entrainer_sur_historique(&bougies, 5, 0.002) {
        tracing::error!(
            "Scheduler ML: entraînement pipeline principal échoué: {}",
            e
        );
        return;
    }
    drop(pipeline);

    let duree_ms = debut.elapsed().as_millis() as i64;

    // ── Dérive : accuracy < 60 % sur les 7 derniers jours ────────────────────
    let derive = db.detecter_derive_ml(0.60).await.unwrap_or(false);
    if derive {
        tracing::warn!(
            "⚠️ DÉRIVE ML DÉTECTÉE — accuracy moyenne < 60% sur 7 jours (XGB={:.1}% LSTM={:.1}% Finale={:.1}%)",
            wf.accuracy_xgb * 100.0,
            wf.accuracy_lstm * 100.0,
            wf.accuracy_finale * 100.0,
        );
    }

    let rec = EntrainementRecord {
        asset: format!("{:?}", asset),
        timeframe: "M15".to_string(),
        nb_bougies: nb_total as i64,
        accuracy_rf: wf.accuracy_xgb,
        accuracy_lstm: wf.accuracy_lstm,
        accuracy_finale: wf.accuracy_finale,
        duree_ms,
        derive_detectee: derive,
    };

    if let Err(e) = db.inserer_historique_entrainement(&rec).await {
        tracing::error!("Scheduler ML: échec enregistrement historique: {}", e);
    } else {
        tracing::info!(
            "✅ Entraînement ML quotidien terminé en {}ms | {} bougies | XGB={:.1}% LSTM={:.1}% Finale={:.1}%",
            duree_ms,
            nb_total,
            wf.accuracy_xgb * 100.0,
            wf.accuracy_lstm * 100.0,
            wf.accuracy_finale * 100.0,
        );
    }
}

/// Calcule le nombre de secondes jusqu'au prochain 00h00 UTC.
fn secondes_jusqu_a_minuit_utc() -> u64 {
    let now = Utc::now();
    let secondes_ecoules =
        now.hour() as u64 * 3600 + now.minute() as u64 * 60 + now.second() as u64;
    let secondes_dans_journee = 86400u64;
    let restant = secondes_dans_journee.saturating_sub(secondes_ecoules);
    // Si on est exactement à minuit, on attend 24h
    if restant == 0 {
        86400
    } else {
        restant
    }
}
