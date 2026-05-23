//! Fine-tuning XGBoost sur les trades Straddle clôturés.
//!
//! Distinct du pipeline général (entraîné sur bougies OHLCV brutes) :
//! ce modèle apprend POURQUOI un signal Straddle particulier a réussi ou échoué.
//!
//! Features : 56 (52 OHLCV + ratio_atr + categorie + session + score_llm).
//! Label    : amplitude suffisante — `1.0` si au moins une jambe atteint TP1+ (pnl_r ≥ 1.0).
//!            L'amplitude (mouvement max quel que soit la direction) est le critère pertinent
//!            pour Straddle. Voir aussi `features::labelliser_straddle` pour le label candle.
//! Garde-fou : min 50 trades clôturés avec snapshot. En dessous → skip silencieux.

use common::{Result, TradingError};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::metrics::accuracy;
use smartcore::xgboost::{XGRegressor, XGRegressorParameters};
use std::time::Instant;

const MIN_SAMPLES: usize = 50;
const CHEMIN_XGB_STRADDLE: &str = "data/modele_xgboost_straddle.json";

/// Noms des 4 features contextuelles Straddle (après les 52 OHLCV standard).
pub const FEATURES_CTX_STRADDLE: [&str; 4] =
    ["ratio_atr", "categorie_enc", "session_enc", "score_llm"];

/// Résultat du fine-tuning stratégie Straddle.
#[derive(Debug, Clone)]
pub struct ResultatFineTuningStraddle {
    pub accuracy_oos: f64,
    pub nb_samples: usize,
    pub sauvegarde: bool,
}

/// Modèle XGBoost fine-tuné sur les trades Straddle clôturés.
pub struct XgbStraddle {
    modele: Option<XGRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>,
}

impl XgbStraddle {
    pub fn new() -> Self {
        Self { modele: None }
    }

    /// Charge depuis disque si disponible. Échec silencieux → modèle vide.
    pub fn charger_depuis_disque() -> Self {
        match std::fs::read_to_string(CHEMIN_XGB_STRADDLE) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(m) => {
                    tracing::info!("XGB Straddle chargé depuis disque");
                    Self { modele: Some(m) }
                }
                Err(e) => {
                    tracing::warn!("XGB Straddle: désérialisation échouée: {}", e);
                    Self::new()
                }
            },
            Err(_) => Self::new(),
        }
    }

    pub fn est_pret(&self) -> bool {
        self.modele.is_some()
    }

    /// Inférence — probabilité de succès Straddle [0, 1].
    /// `features` doit contenir 56 éléments (52 OHLCV + 4 contextuelles).
    pub fn predire_score(&self, features: &[f64]) -> Option<f64> {
        let modele = self.modele.as_ref()?;
        let x = DenseMatrix::from_2d_vec(&vec![features.to_vec()]).ok()?;
        modele.predict(&x).ok().map(|p| p[0].clamp(0.0, 1.0))
    }
}

impl Default for XgbStraddle {
    fn default() -> Self {
        Self::new()
    }
}

/// Entraîne le XGBoost Straddle sur les trades clôturés (features 56 + labels).
///
/// - `samples` : `(features_56, label)` — label 1.0=gagnant, 0.0=perdant
/// - Guard : retourne `Ok(None)` si < 50 samples
/// - Split 80/20 train/OOS
/// - Persistance automatique si OOS >= 52%
pub fn entrainer_sur_trades_clotures(
    samples: &[(Vec<f64>, f64)],
) -> Result<Option<ResultatFineTuningStraddle>> {
    if samples.len() < MIN_SAMPLES {
        tracing::info!(
            "XGB Straddle: {} samples < {} requis — fine-tuning ignoré",
            samples.len(),
            MIN_SAMPLES
        );
        return Ok(None);
    }

    let debut = Instant::now();
    let nb = samples.len();

    let split = (nb as f64 * 0.80) as usize;
    let (train, oos) = samples.split_at(split);

    if oos.is_empty() || train.len() < 30 {
        return Ok(None);
    }

    let features_train: Vec<Vec<f64>> = train.iter().map(|(f, _)| f.clone()).collect();
    let labels_train: Vec<f64> = train.iter().map(|(_, l)| *l).collect();
    let features_oos: Vec<Vec<f64>> = oos.iter().map(|(f, _)| f.clone()).collect();
    let labels_oos: Vec<f64> = oos.iter().map(|(_, l)| *l).collect();

    let x_train = DenseMatrix::from_2d_vec(&features_train)
        .map_err(|e| TradingError::ML(format!("Matrice XGB Straddle train: {}", e)))?;
    let x_oos = DenseMatrix::from_2d_vec(&features_oos)
        .map_err(|e| TradingError::ML(format!("Matrice XGB Straddle OOS: {}", e)))?;

    let params = XGRegressorParameters {
        n_estimators: 80,
        max_depth: 4,
        learning_rate: 0.05,
        lambda: 2.0,
        gamma: 0.2,
        subsample: 0.7,
        ..XGRegressorParameters::default()
    };

    let modele = XGRegressor::fit(&x_train, &labels_train, params)
        .map_err(|e| TradingError::ML(format!("Entraînement XGB Straddle: {}", e)))?;

    let preds_oos = modele
        .predict(&x_oos)
        .map_err(|e| TradingError::ML(format!("Prédiction XGB Straddle OOS: {}", e)))?;

    let pred_cls: Vec<u8> = preds_oos.iter().map(|&p| if p >= 0.5 { 1 } else { 0 }).collect();
    let true_cls: Vec<u8> = labels_oos.iter().map(|&l| if l >= 0.5 { 1 } else { 0 }).collect();
    let accuracy_oos = accuracy(&true_cls, &pred_cls);

    tracing::info!(
        "XGB Straddle fine-tuning: {} samples ({} train / {} OOS) | accuracy_OOS={:.1}% | {:?}",
        nb,
        split,
        oos.len(),
        accuracy_oos * 100.0,
        debut.elapsed()
    );

    let sauvegarde = if accuracy_oos >= 0.52 {
        let json = serde_json::to_string(&modele)
            .map_err(|e| TradingError::ML(format!("Sérialisation XGB Straddle: {}", e)))?;
        if let Some(parent) = std::path::Path::new(CHEMIN_XGB_STRADDLE).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(CHEMIN_XGB_STRADDLE, &json)
            .map_err(|e| TradingError::ML(format!("Écriture XGB Straddle: {}", e)))?;
        tracing::info!("XGB Straddle sauvegardé (OOS={:.1}%)", accuracy_oos * 100.0);
        true
    } else {
        tracing::warn!(
            "XGB Straddle non sauvegardé: OOS={:.1}% < 52%",
            accuracy_oos * 100.0
        );
        false
    };

    Ok(Some(ResultatFineTuningStraddle {
        accuracy_oos,
        nb_samples: nb,
        sauvegarde,
    }))
}
