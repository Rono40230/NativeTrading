//! Fine-tuning XGBoost sur les trades SMC Directionnel clôturés.
//!
//! Distinct du pipeline général : ce modèle apprend POURQUOI un signal SMC
//! particulier a réussi (TP1/TP2) ou échoué (SL).
//!
//! Features : 59 (52 OHLCV + 7 contextuelles SMC).
//! Label    : 1.0=TP1/TP2, 0.0=SL.
//! Garde-fou : min 50 trades clôturés avec snapshot. En dessous → skip silencieux.

use common::{Result, TradingError};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::metrics::accuracy;
use smartcore::xgboost::{XGRegressor, XGRegressorParameters};
use std::time::Instant;

const MIN_SAMPLES: usize = 50;
const CHEMIN_XGB_SMC: &str = "data/modele_xgboost_smc.json";

/// Noms des 7 features contextuelles SMC (après les 52 OHLCV standard).
pub const FEATURES_CTX_SMC: [&str; 7] = [
    "tendance_pts", "order_block_pts", "ifvg_pts",
    "fibonacci_pts", "imbalance_pts", "kill_zone", "sweep",
];

/// Résultat du fine-tuning stratégie SMC.
#[derive(Debug, Clone)]
pub struct ResultatFineTuningSmc {
    pub accuracy_oos: f64,
    pub nb_samples: usize,
    pub sauvegarde: bool,
}

/// Modèle XGBoost fine-tuné sur les trades SMC clôturés.
pub struct XgbSmc {
    modele: Option<XGRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>,
}

impl XgbSmc {
    pub fn new() -> Self {
        Self { modele: None }
    }

    /// Charge depuis disque si disponible. Échec silencieux → modèle vide.
    pub fn charger_depuis_disque() -> Self {
        match std::fs::read_to_string(CHEMIN_XGB_SMC) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(m) => {
                    tracing::info!("XGB SMC chargé depuis disque");
                    Self { modele: Some(m) }
                }
                Err(e) => {
                    tracing::warn!("XGB SMC: désérialisation échouée: {}", e);
                    Self::new()
                }
            },
            Err(_) => Self::new(),
        }
    }

    pub fn est_pret(&self) -> bool {
        self.modele.is_some()
    }

    /// Inférence — probabilité de succès SMC [0, 1].
    /// `features` doit contenir 59 éléments (52 OHLCV + 7 contextuelles).
    pub fn predire_score(&self, features: &[f64]) -> Option<f64> {
        let modele = self.modele.as_ref()?;
        let x = DenseMatrix::from_2d_vec(&vec![features.to_vec()]).ok()?;
        modele.predict(&x).ok().map(|p| p[0].clamp(0.0, 1.0))
    }
}

impl Default for XgbSmc {
    fn default() -> Self {
        Self::new()
    }
}

/// Entraîne le XGBoost SMC sur les trades clôturés.
///
/// - Guard : retourne `Ok(None)` si < 50 samples
/// - Split 80/20 train/OOS
/// - Sauvegarde automatique si OOS >= 52%
pub fn entrainer_sur_trades_clotures(
    samples: &[(Vec<f64>, f64)],
) -> Result<Option<ResultatFineTuningSmc>> {
    if samples.len() < MIN_SAMPLES {
        tracing::info!(
            "XGB SMC: {} samples < {} requis — fine-tuning ignoré",
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
        .map_err(|e| TradingError::ML(format!("Matrice XGB SMC train: {}", e)))?;
    let x_oos = DenseMatrix::from_2d_vec(&features_oos)
        .map_err(|e| TradingError::ML(format!("Matrice XGB SMC OOS: {}", e)))?;

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
        .map_err(|e| TradingError::ML(format!("Entraînement XGB SMC: {}", e)))?;

    let preds_oos = modele
        .predict(&x_oos)
        .map_err(|e| TradingError::ML(format!("Prédiction XGB SMC OOS: {}", e)))?;

    let pred_cls: Vec<u8> = preds_oos.iter().map(|&p| if p >= 0.5 { 1 } else { 0 }).collect();
    let true_cls: Vec<u8> = labels_oos.iter().map(|&l| if l >= 0.5 { 1 } else { 0 }).collect();
    let accuracy_oos = accuracy(&true_cls, &pred_cls);

    tracing::info!(
        "XGB SMC fine-tuning: {} samples ({} train / {} OOS) | accuracy_OOS={:.1}% | {:?}",
        nb, split, oos.len(), accuracy_oos * 100.0, debut.elapsed()
    );

    let sauvegarde = if accuracy_oos >= 0.52 {
        let json = serde_json::to_string(&modele)
            .map_err(|e| TradingError::ML(format!("Sérialisation XGB SMC: {}", e)))?;
        if let Some(parent) = std::path::Path::new(CHEMIN_XGB_SMC).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(CHEMIN_XGB_SMC, &json)
            .map_err(|e| TradingError::ML(format!("Écriture XGB SMC: {}", e)))?;
        tracing::info!("XGB SMC sauvegardé (OOS={:.1}%)", accuracy_oos * 100.0);
        true
    } else {
        tracing::warn!("XGB SMC non sauvegardé: OOS={:.1}% < 52%", accuracy_oos * 100.0);
        false
    };

    Ok(Some(ResultatFineTuningSmc { accuracy_oos, nb_samples: nb, sauvegarde }))
}
