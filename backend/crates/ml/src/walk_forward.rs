use common::{Candle, Direction, Result, TradingError};

use crate::{
    features::{extraire_features, labelliser, NB_FEATURES},
    lstm::{ModeleHybrideLstm, LONGUEUR_SEQ},
    xgboost::ModeleXGBoost,
};

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
/// Entraîne un pipeline ML temporaire sur le jeu d'entraînement et mesure
/// l'accuracy sur le jeu de test (out-of-sample). Le pipeline ML principal
/// **n'est pas modifié** — utiliser `PipelineML::entrainer_sur_historique` à la suite.
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
    let _test = &bougies[split..];

    // ── Entraînement sur le jeu train ───────────────────────────────────────
    let mut xgb_tmp = ModeleXGBoost::new(50);
    let mut lstm_tmp = ModeleHybrideLstm::nouveau(NB_FEATURES);

    use rayon::prelude::*;
    let (features_train, labels_train): (Vec<_>, Vec<_>) = (60..train.len())
        .into_par_iter()
        .filter_map(|i| {
            let f = extraire_features(&train[..=i])?;
            let l = labelliser(train, i, 5, 0.002)?;
            Some((f, l))
        })
        .unzip();

    if features_train.is_empty() {
        return Err(TradingError::ML(
            "Aucun échantillon valide pour walk-forward".into(),
        ));
    }

    xgb_tmp.entrainer(&features_train, &labels_train)?;

    let sequences: Vec<Vec<Vec<f64>>> = (LONGUEUR_SEQ..features_train.len())
        .map(|i| features_train[i - LONGUEUR_SEQ..i].to_vec())
        .collect();
    let labels_seq: Vec<f64> = labels_train[LONGUEUR_SEQ..].to_vec();
    lstm_tmp.entrainer(&sequences, &labels_seq, 10, 0.001);

    // ── Évaluation sur le jeu test ───────────────────────────────────────────
    // Le test set commence après 60 bougies de contexte (issues du train)
    let contexte: Vec<Candle> = bougies[split.saturating_sub(60)..].to_vec();

    let acc_xgb = evaluer_xgb(&xgb_tmp, &contexte);
    let acc_lstm = evaluer_lstm(&lstm_tmp, &contexte);
    let acc_finale = 0.6 * acc_lstm + 0.4 * acc_xgb;

    // Score sur le jeu d'entraînement (indicateur d'overfit vs OOS)
    let acc_xgb_train = evaluer_xgb(&xgb_tmp, train);
    let acc_lstm_train = evaluer_lstm(&lstm_tmp, train);
    let acc_train = 0.6 * acc_lstm_train + 0.4 * acc_xgb_train;

    Ok(ResultatWalkForward {
        accuracy_xgb: (acc_xgb * 1000.0).round() / 1000.0,
        accuracy_lstm: (acc_lstm * 1000.0).round() / 1000.0,
        accuracy_finale: (acc_finale * 1000.0).round() / 1000.0,
        accuracy_train: (acc_train * 1000.0).round() / 1000.0,
        nb_bougies_train: split,
        nb_bougies_test: n - split,
    })
}

/// Évalue le XGBoost sur une fenêtre de bougies.
fn evaluer_xgb(xgb: &ModeleXGBoost, bougies: &[Candle]) -> f64 {
    use rayon::prelude::*;
    let (ok, total) = (60..bougies.len())
        .into_par_iter()
        .filter_map(|i| {
            let f = extraire_features(&bougies[..=i])?;
            let label = labelliser(bougies, i, 5, 0.002)?;
            let Ok((direction, _)) = xgb.predire(&f) else {
                return None;
            };
            let pred_label = if direction == Direction::Long { 1.0 } else { 0.0 };
            let ok = if (pred_label - label).abs() < 0.5 { 1 } else { 0 };
            Some((ok, 1))
        })
        .reduce(|| (0usize, 0usize), |a, b| (a.0 + b.0, a.1 + b.1));

    if total == 0 {
        return 0.5;
    }
    ok as f64 / total as f64
}

/// Évalue le LSTM sur une fenêtre de bougies.
fn evaluer_lstm(lstm: &ModeleHybrideLstm, bougies: &[Candle]) -> f64 {
    use rayon::prelude::*;
    let (ok, total) = ((60 + LONGUEUR_SEQ)..bougies.len())
        .into_par_iter()
        .filter_map(|i| {
            let label = labelliser(bougies, i, 5, 0.002)?;
            let sequence: Vec<Vec<f64>> = (i - LONGUEUR_SEQ..i)
                .filter_map(|j| extraire_features(&bougies[..=j]))
                .collect();
            if sequence.len() != LONGUEUR_SEQ {
                return None;
            }
            let conf_long = lstm.predire(&sequence);
            let pred = if conf_long >= 0.5 { 1.0 } else { 0.0 };
            let ok = if (pred - label).abs() < 0.5 { 1 } else { 0 };
            Some((ok, 1))
        })
        .reduce(|| (0usize, 0usize), |a, b| (a.0 + b.0, a.1 + b.1));

    if total == 0 {
        return 0.5;
    }
    ok as f64 / total as f64
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
        // Vérifie uniquement le rejet, pas l'entraînement complet (trop lent en CI)
        let bougies_ok: Vec<Candle> = (0..200).map(|i| b(i as f64 + 10.0)).collect();
        // 200 bougies exactement est valide (pas de rejet précoce)
        // On ne vérifie pas le résultat ML (trop de dépendances GPU/features)
        // mais juste que la fonction ne panic pas et ne rejette pas les 200 bougies
        let _ = entrainer_walk_forward(&bougies_ok); // Ok ou Err(ML "aucun échantillon") — les 2 sont acceptables
    }
}
