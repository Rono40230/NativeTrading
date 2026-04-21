use common::Candle;
use db::entrainements::EntrainementRecord;
use db::Database;
use ml::{walk_forward::entrainer_walk_forward, PipelineML};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::ml_retrain_handler::RetainState;
use crate::utils::{parse_asset, parse_timeframe};

pub const MIN_BOUGIES_ENTRAINEMENT: i64 = 200;

/// Itère sur toutes les combinaisons asset × TF disponibles en DB et ré-entraîne le pipeline.
/// Appelé par le scheduler quotidien (progress_state=None) et les retraining manuels (Some).
pub async fn executer_entrainements_tous(
    db: &Arc<Database>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
    progress_state: Option<Arc<RwLock<RetainState>>>,
) {
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

    let total = combinaisons.len();
    tracing::info!("Scheduler ML: {} combinaison(s) à entraîner", total);

    if let Some(ref s) = progress_state {
        let mut g = s.write().await;
        g.nb_combinaisons_total = total;
        g.nb_combinaisons_done = 0;
        g.combinaison_en_cours = "Chargement des données…".to_string();
        g.message = format!("Préparation ({} combinaisons)…", total);
    }

    // ── Phase 1 : Fetch séquentiel (async DB) ────────────────────────────────
    struct CombPrep {
        asset_str:   String,
        tf_str:      String,
        bougies:     Vec<Candle>,
        nb_total_db: usize,
    }

    let mut prepared: Vec<CombPrep> = Vec::with_capacity(total);
    let mut _n_sautes = 0usize;

    for (asset_str, tf_str) in &combinaisons {
        let asset = match parse_asset(asset_str) {
            Some(a) => a,
            None => {
                tracing::warn!("Scheduler ML: asset inconnu '{}' — ignoré", asset_str);
                _n_sautes += 1;
                continue;
            }
        };
        let timeframe = parse_timeframe(tf_str);
        let bougies_raw = match db.obtenir_bougies_toutes(&asset, &timeframe).await {
            Ok(b) if b.len() >= MIN_BOUGIES_ENTRAINEMENT as usize => b,
            Ok(b) => {
                tracing::warn!(
                    "Scheduler ML: {}/{} — {} bougies insuffisantes",
                    asset_str, tf_str, b.len()
                );
                _n_sautes += 1;
                continue;
            }
            Err(e) => {
                tracing::error!("Scheduler ML: {}/{} — erreur DB: {}", asset_str, tf_str, e);
                _n_sautes += 1;
                continue;
            }
        };
        let nb_total_db = bougies_raw.len();
        let bougies = limiter_bougies_par_tf(bougies_raw, tf_str);
        prepared.push(CombPrep {
            asset_str: asset_str.clone(),
            tf_str: tf_str.clone(),
            bougies,
            nb_total_db,
        });
    }

    if let Some(ref s) = progress_state {
        let mut g = s.write().await;
        g.nb_combinaisons_total = prepared.len();
        g.nb_combinaisons_done = 0;
        g.combinaison_en_cours = format!("{} walk-forwards en parallèle…", prepared.len());
        g.message = format!("Walk-forward en cours ({} combinaisons)…", prepared.len());
    }

    // ── Phase 2 : Walk-forwards en parallèle via UN seul rayon::par_iter ────────
    // try_write() est non-bloquant : mis à jour en temps réel depuis les threads rayon.
    // Si le lock est tenu par le polling, on saute la mise à jour (pas grave pour l'UI).
    type WfResult = (String, String, Vec<Candle>, usize, ml::walk_forward::ResultatWalkForward);
    let nb_total_wf = prepared.len();
    let progress_for_rayon = progress_state.clone();
    let compteur_atomique = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let compteur_rayon = compteur_atomique.clone();

    let resultats: Vec<WfResult> = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        prepared
            .into_par_iter()
            .filter_map(|p| {
                let asset = p.asset_str.clone();
                let tf = p.tf_str.clone();
                let wf = match entrainer_walk_forward(&p.bougies) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("Walk-forward {}/{} échoué: {}", &asset, &tf, e);
                        return None;
                    }
                };
                let n = compteur_rayon.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                // Mise à jour temps réel : try_write() est non-bloquant, pas de deadlock possible
                if let Some(ref s) = progress_for_rayon {
                    if let Ok(mut g) = s.try_write() {
                        g.nb_combinaisons_done = n;
                        g.combinaison_en_cours =
                            format!("WF {}/{}: {}/{} ✓", n, nb_total_wf, asset, tf);
                    }
                }
                Some((p.asset_str, p.tf_str, p.bougies, p.nb_total_db, wf))
            })
            .collect()
    })
    .await
    .unwrap_or_default();

    let nb_wf_done = compteur_atomique.load(std::sync::atomic::Ordering::Relaxed);
    let nb_final = resultats.len();
    if let Some(ref s) = progress_state {
        let mut g = s.write().await;
        // Phase 3 repart de 0 / nb_final (walk-forward = terminé)
        g.nb_combinaisons_total = nb_final;
        g.nb_combinaisons_done = 0;
        g.combinaison_en_cours = "Entraînement final (GPU)…".to_string();
        g.message = format!("{} walk-forwards OK — entraînement final ({} combos)", nb_wf_done, nb_final);
    }

    tracing::info!(
        "Scheduler ML: {} walk-forwards terminés — phase entraînement final",
        resultats.len()
    );

    // ── Phase 3 : Entraînement final séquentiel (GPU LSTM, pipeline partagé) ─
    for (asset_str, tf_str, bougies, nb_total_db, wf) in resultats {
        let nb_total = bougies.len();

        if let Some(ref s) = progress_state {
            let mut g = s.write().await;
            g.combinaison_en_cours = format!("{}/{}", asset_str, tf_str);
            g.message = format!("Entraînement final {}/{} ({}/{})", asset_str, tf_str, g.nb_combinaisons_done + 1, nb_final);
        }

        let debut = std::time::Instant::now();

        let pipeline_shared = pipeline_ml.clone();
        let b_owned = bougies;
        let entrainement_res = tokio::task::spawn_blocking(move || {
            let mut pipeline = pipeline_shared.blocking_lock();
            pipeline.entrainer_sur_historique(&b_owned, 5, 0.002)
        }).await.unwrap_or_else(|e| {
            tracing::error!("Pannic de spawn_blocking: {}", e);
            Err(common::TradingError::ML("Thread bloquant échoué".into()))
        });

        if let Err(e) = entrainement_res {
            tracing::error!(
                "Scheduler ML: {}/{} — entraînement final échoué: {}",
                asset_str, tf_str, e
            );
            if let Some(ref s) = progress_state {
                let mut g = s.write().await;
                g.nb_combinaisons_done += 1;
            }
            continue;
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
                asset_str, tf_str, e
            );
        } else {
            tracing::info!(
                "✅ {}/{} — {}ms | {}/{} bougies | XGB={:.1}% LSTM={:.1}% Finale={:.1}%{}",
                asset_str, tf_str, duree_ms, nb_total, nb_total_db,
                wf.accuracy_xgb * 100.0,
                wf.accuracy_lstm * 100.0,
                wf.accuracy_finale * 100.0,
                if derive { " ⚠️ DÉRIVE" } else { "" }
            );
        }
        if let Some(ref s) = progress_state {
            let mut g = s.write().await;
            g.nb_combinaisons_done += 1;
        }
    }
}

/// Tronque aux N bougies les plus récentes. Évite les durées excessives (M1 ≈ 1M → 50k).
pub fn limiter_bougies_par_tf(mut bougies: Vec<common::Candle>, tf: &str) -> Vec<common::Candle> {
    let max: usize = match tf {
        "M1" => 50_000,
        "M5" => 50_000,
        "M15" => 30_000,
        "M30" => 20_000,
        "H1" => 10_000,
        "H4" => 5_000,
        "D1" => 1_000,
        _ => 500, // W1 et autres
    };
    if bougies.len() > max {
        let debut = bougies.len() - max;
        bougies.drain(..debut);
    }
    bougies
}
