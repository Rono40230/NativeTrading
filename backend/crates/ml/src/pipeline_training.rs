use std::time::Instant;
use common::{Candle, Result, TradingError};
use rayon::prelude::*;

use crate::pipeline::{PipelineML};
use crate::lstm::LONGUEUR_SEQ;
use crate::features::labelliser;
use crate::features_corrompues;

pub fn entrainer_sur_historique(
    pipeline: &mut PipelineML,
    bougies: &[Candle],
    horizon: usize,
    seuil_pct: f64,
) -> Result<(f64, f64)> {
    tracing::info!(
        "Entraînement hybride XGBoost+LSTM sur {} bougies",
        bougies.len()
    );
    let debut = Instant::now();

    let series = crate::features_precalc::precalculer(bougies);
    let paires: Vec<(Vec<f64>, f64)> = (60..bougies.len())
        .into_par_iter()
        .filter_map(|i| {
            let f = crate::features_precalc::extraire_depuis_series(&series, bougies, i)?;
            let l = labelliser(bougies, i, horizon, seuil_pct)?;
            Some((f, l))
        })
        .collect();

    let (features_dataset, labels): (Vec<Vec<f64>>, Vec<f64>) =
        paires.into_iter().unzip();

    if features_dataset.is_empty() {
        return Err(TradingError::ML("Aucun échantillon valide".into()));
    }

    let n_corrompus = features_dataset.iter().filter(|f| features_corrompues(f)).count();
    if n_corrompus > features_dataset.len() / 2 {
        return Err(TradingError::ML(format!(
            "Données corrompues : {}/{} échantillons avec NaN/Inf — asset ignoré",
            n_corrompus, features_dataset.len()
        )));
    }

    // Zone de valeur : 15k-30k. En dessous = sous-apprentissage (3-4j de M1 seulement),
    // au-delà = régimes conflictuels. Release build gère 20k en ~2-3s par asset.
    const MAX_SAMPLES_XGB: usize = 20_000;
    let debut_xgb = features_dataset.len().saturating_sub(MAX_SAMPLES_XGB);
    let acc_xgb = pipeline.xgb.entrainer(&features_dataset[debut_xgb..], &labels[debut_xgb..])?;

    // FIX OOM : Ollama occupe ~22 GB VRAM → ~2 GB libres sur RTX 3090.
    // 4k séquences × [60, 52] × float32 ≈ 47 MB tenseur + ~100 MB gradients
    // = ~150 MB peak VRAM, safe même avec seulement 1.5 GB libres.
    // Le mutex dans entrainement_gpu sérialise les jobs concurrents.
    #[cfg(feature = "cuda")]
    let max_seq = if tch::Cuda::is_available() { 4_000 } else { 2_000 };
    #[cfg(not(feature = "cuda"))]
    let max_seq = 2_000;

    let debut_seq = features_dataset.len().saturating_sub(max_seq).max(LONGUEUR_SEQ);
    let seq_total: Vec<Vec<Vec<f64>>> = (debut_seq..features_dataset.len())
        .map(|i| features_dataset[i - LONGUEUR_SEQ..i].to_vec())
        .collect();
    let labels_seq_total: Vec<f64> = labels[debut_seq..].to_vec();

    #[cfg(feature = "cuda")]
    let acc_lstm = if tch::Cuda::is_available() {
        tracing::info!("🚀 Démarrage entraînement LSTM GPU sur {} séquences (epochs: 15)...", seq_total.len());
        let chrono_gpu = std::time::Instant::now();
        
        // catch_unwind : tch-rs peut paniquer (unwrap interne) sur CUDA OOM.
        // On convertit le panic en fallback CPU sans crasher la tâche scheduler.
        let gpu_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::lstm::entrainement_gpu::entrainer_sur_gpu(
                &mut pipeline.lstm, &seq_total, &labels_seq_total, 15, 0.001,
                crate::pipeline::CHEMIN_LSTM_PT,
            )
        }));
        match gpu_result {
            Ok(Ok(acc)) => {
                tracing::info!("✅ LSTM GPU terminé en {:.2?}. Precision OOS: {:.2}%", chrono_gpu.elapsed(), acc * 100.0);
                acc
            }
            Ok(Err(e)) => {
                tracing::warn!("LSTM GPU échoué, fallback CPU: {}", e);
                const MAX: usize = 2_000;
                let d = seq_total.len().saturating_sub(MAX);
                pipeline.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
            }
            Err(_) => {
                tracing::warn!("LSTM GPU panic (CUDA OOM probable), fallback CPU");
                const MAX: usize = 2_000;
                let d = seq_total.len().saturating_sub(MAX);
                pipeline.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
            }
        }
    } else {
        const MAX: usize = 2_000;
        let d = seq_total.len().saturating_sub(MAX);
        pipeline.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
    };
    #[cfg(not(feature = "cuda"))]
    let acc_lstm = {
        const MAX: usize = 2_000;
        let d = seq_total.len().saturating_sub(MAX);
        pipeline.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
    };

    tracing::info!(
        "Pipeline hybride XGB+LSTM entraîné en {:?}: {} éch. XGB={:.1}% LSTM={:.1}%",
        debut.elapsed(),
        features_dataset.len(),
        acc_xgb * 100.0,
        acc_lstm * 100.0
    );

    if let Err(e) = pipeline.sauvegarder_sur_disque() {
        tracing::warn!("Échec sauvegarde pipeline ML: {}", e);
    }

    #[cfg(feature = "cuda")]
    pipeline.activer_gpu_si_pret();

    Ok((acc_xgb, acc_lstm))
}
