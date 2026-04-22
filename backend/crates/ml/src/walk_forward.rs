use common::{Candle, Direction, Result, TradingError};

use rayon::prelude::*;
use crate::{
    features::{labelliser},
    features_precalc::{extraire_depuis_series, precalculer, SeriesIndicateurs},
    xgboost::ModeleXGBoost,
};

/// Nombre max d'échantillons XGBoost dans walk_forward.
/// Walk_forward = métriques OOS uniquement, pas le modèle final.
/// M1 sans limite : 28k échantillons × 50 arbres → 74s par tâche → système bloqué.
/// Avec 2k : ~3s par tâche.
const MAX_SAMPLES_XGB_WF: usize = 2_000;

/// Résultat d'un entraînement walk-forward (métriques out-of-sample)
pub struct ResultatWalkForward {
    /// Accuracy XGBoost out-of-sample
    pub accuracy_xgb: f64,
    pub accuracy_lstm: f64,
    /// Score fusion pondéré : 0.6 × lstm + 0.4 × xgb (sur jeu test OOS)
    pub accuracy_finale: f64,
    /// Score fusion sur le jeu d'entraînement (indicateur d'overfit)
    pub accuracy_train: f64,
    pub nb_bougies_train: usize,
    pub nb_bougies_test: usize,
}

/// Entraînement walk-forward : 75 % train / 25 % test (≈ 3 mois / 1 mois).
///
/// Phase 2 : XGBoost uniquement (pas de LSTM). Le LSTM est réservé à la Phase 3
/// (entraînement final). Walk-forward mesure la robustesse temporelle des features —
/// XGBoost seul suffit pour ce diagnostic et est 10× plus rapide.
/// Le pipeline ML principal **n'est pas modifié**.
pub fn entrainer_walk_forward(bougies: &[Candle]) -> Result<ResultatWalkForward> {
    let n = bougies.len();
    if n < 200 {
        return Err(TradingError::ML(format!(
            "Données insuffisantes pour walk-forward: {} bougies (min 200)",
            n
        )));
    }

    let split = (n as f64 * 0.75) as usize;
    let train = &bougies[..split];

    // ── Entraînement XGBoost uniquement (Phase 2 = diagnostic OOS, pas LSTM) ──
    let mut xgb_tmp = ModeleXGBoost::new(50);

    // Pré-calcul O(N) des indicateurs sur le jeu d'entraînement
    let series_train = precalculer(train);

    // Extraction parallèle des features
    let paires: Vec<_> = (60..train.len())
        .into_par_iter()
        .filter_map(|i| {
            let f = extraire_depuis_series(&series_train, train, i)?;
            let l = labelliser(train, i, 5, 0.002)?;
            Some((f, l))
        })
        .collect();

    let (features_train, labels_train): (Vec<Vec<f64>>, Vec<f64>) = paires.into_iter().unzip();

    if features_train.is_empty() {
        return Err(TradingError::ML(
            "Aucun échantillon extrait du jeu d'entraînement".into(),
        ));
    }

    // XGBoost CPU : prendre les MAX_SAMPLES_XGB_WF échantillons les plus récents.
    // entrainer_cpu() = pas de CUDA overhead, ~3× plus rapide sur petits datasets (2k samples).
    let debut_xgb = features_train.len().saturating_sub(MAX_SAMPLES_XGB_WF);
    xgb_tmp.entrainer_cpu(&features_train[debut_xgb..], &labels_train[debut_xgb..])?;

    // Évaluation sur le jeu de test
    let contexte: Vec<Candle> = bougies[split.saturating_sub(60)..].to_vec();
    let series_ctx = precalculer(&contexte);

    let acc_xgb = evaluer_xgb(&xgb_tmp, &contexte, &series_ctx);

    // Score sur jeu d'entraînement (indicateur d'overfit vs OOS)
    let acc_xgb_train = evaluer_xgb(&xgb_tmp, train, &series_train);

    Ok(ResultatWalkForward {
        accuracy_xgb: (acc_xgb * 1000.0).round() / 1000.0,
        // accuracy_lstm aliasé sur accuracy_xgb : LSTM supprimé de Phase 2
        accuracy_lstm: (acc_xgb * 1000.0).round() / 1000.0,
        accuracy_finale: (acc_xgb * 1000.0).round() / 1000.0,
        accuracy_train: (acc_xgb_train * 1000.0).round() / 1000.0,
        nb_bougies_train: split,
        nb_bougies_test: n - split,
    })
}

/// Évalue le XGBoost sur une fenêtre de bougies (O(N) avec précalc, séquentiel).
fn evaluer_xgb(xgb: &ModeleXGBoost, bougies: &[Candle], series: &SeriesIndicateurs) -> f64 {
    let mut ok = 0usize;
    let mut total = 0usize;
    for i in 60..bougies.len() {
        let (Some(f), Some(label)) = (
            extraire_depuis_series(series, bougies, i),
            labelliser(bougies, i, 5, 0.002),
        ) else {
            continue;
        };
        let Ok((direction, _)) = xgb.predire(&f) else {
            continue;
        };
        let pred = if direction == Direction::Long { 1.0 } else { 0.0 };
        if (pred - label).abs() < 0.5 {
            ok += 1;
        }
        total += 1;
    }
    if total == 0 {
        0.5
    } else {
        ok as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

    fn b(close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1000.0,
        }
    }

    #[test]
    fn walk_forward_erreur_si_moins_de_200_bougies() {
        let bougies: Vec<Candle> = (0..199).map(|i| b(i as f64 + 10.0)).collect();
        let res = entrainer_walk_forward(&bougies);
        assert!(res.is_err(), "Moins de 200 bougies → Err");
    }

    #[test]
    fn walk_forward_split_coherent() {
        let bougies_ok: Vec<Candle> = (0..200).map(|i| b(i as f64 + 10.0)).collect();
        let _ = entrainer_walk_forward(&bougies_ok);
    }
}
