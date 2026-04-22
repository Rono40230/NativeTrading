//! Fine-tuning XGBoost sur les trades Rockets clôturés (P3 ROADMAP).
//!
//! Distinct du pipeline général (entraîné sur bougies OHLCV brutes) :
//! ce modèle apprend POURQUOI un signal Rockets particulier a réussi ou échoué.
//!
//! Garde-fou : min 50 trades clôturés avec snapshot. En dessous → skip silencieux.

use crate::feature_noms::FEATURE_NOMS;
use common::{Result, TradingError};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::metrics::accuracy;
use smartcore::xgboost::{XGRegressor, XGRegressorParameters};
use std::time::Instant;

const MIN_SAMPLES: usize = 50;
const CHEMIN_XGB_ROCKETS: &str = "data/modele_xgboost_rockets.json";

/// Importance d'une feature (chute accuracy OOS lors de la permutation).
#[derive(Debug, Clone)]
pub struct ImportanceFeature {
    pub feature_idx: usize,
    pub feature_nom: &'static str,
    pub importance: f64,
}

/// Résultat du fine-tuning stratégie Rockets.
#[derive(Debug, Clone)]
pub struct ResultatFineTuning {
    /// Accuracy sur jeu OOS (20% réservé) — indicateur de qualité.
    pub accuracy_oos: f64,
    /// Nombre d'échantillons utilisés (trades clôturés avec snapshot).
    pub nb_samples: usize,
    /// `true` si le modèle a été persisté sur disque.
    pub sauvegarde: bool,
    /// Importances des features par permutation, triées par importance décroissante.
    pub importances: Vec<ImportanceFeature>,
}

/// Modèle XGBoost fine-tuné sur les trades Rockets clôturés.
/// Séparé du modèle général pour éviter la contamination des données.
pub struct XgbRockets {
    modele: Option<XGRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>,
}

impl XgbRockets {
    pub fn new() -> Self {
        Self { modele: None }
    }

    /// Charge depuis disque si disponible.
    pub fn charger_depuis_disque() -> Self {
        match std::fs::read_to_string(CHEMIN_XGB_ROCKETS) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(m) => {
                    tracing::info!("XGB Rockets chargé depuis disque");
                    Self { modele: Some(m) }
                }
                Err(e) => {
                    tracing::warn!("XGB Rockets: désérialisation échouée: {}", e);
                    Self::new()
                }
            },
            Err(_) => Self::new(),
        }
    }

    pub fn est_pret(&self) -> bool {
        self.modele.is_some()
    }

    /// Inférence — score de probabilité TP [0, 1].
    pub fn predire_score(&self, features: &[f64]) -> Option<f64> {
        let modele = self.modele.as_ref()?;
        let x = DenseMatrix::from_2d_vec(&vec![features.to_vec()]).ok()?;
        modele.predict(&x).ok().map(|p| p[0].clamp(0.0, 1.0))
    }
}

impl Default for XgbRockets {
    fn default() -> Self {
        Self::new()
    }
}

/// Entraîne le XGBoost Rockets sur les trades clôturés (features + labels TP/SL).
///
/// - `samples` : `(features_52, label)` — label 1.0=TP, 0.0=SL/invalide
/// - Guard : retourne `Ok(None)` si < 50 samples
/// - Split 80/20 train/OOS — accuracy OOS retournée
/// - Persistance automatique si OOS >= 52%
pub fn entrainer_sur_trades_clotures(
    samples: &[(Vec<f64>, f64)],
) -> Result<Option<ResultatFineTuning>> {
    if samples.len() < MIN_SAMPLES {
        tracing::info!(
            "XGB Rockets: {} samples < {} requis — fine-tuning ignoré",
            samples.len(),
            MIN_SAMPLES
        );
        return Ok(None);
    }

    let debut = Instant::now();
    let nb = samples.len();

    // Split 80% train / 20% OOS
    let split = (nb as f64 * 0.80) as usize;
    let (train, oos) = samples.split_at(split);

    let features_train: Vec<Vec<f64>> = train.iter().map(|(f, _)| f.clone()).collect();
    let labels_train: Vec<f64> = train.iter().map(|(_, l)| *l).collect();
    let features_oos: Vec<Vec<f64>> = oos.iter().map(|(f, _)| f.clone()).collect();
    let labels_oos: Vec<f64> = oos.iter().map(|(_, l)| *l).collect();

    // Bail si le split OOS est trop petit
    if features_oos.is_empty() || features_train.len() < 30 {
        return Ok(None);
    }

    let x_train = DenseMatrix::from_2d_vec(&features_train)
        .map_err(|e| TradingError::ML(format!("Matrice XGB Rockets train: {}", e)))?;
    let x_oos = DenseMatrix::from_2d_vec(&features_oos)
        .map_err(|e| TradingError::ML(format!("Matrice XGB Rockets OOS: {}", e)))?;

    let params = XGRegressorParameters {
        n_estimators: 100, // Augmenté pour construire plus d'arbres
        max_depth: 5, // Autorisons un peu d'overfit pour chercher le signal dans le bruit
        learning_rate: 0.03, // Plus lent, plus robuste
        lambda: 0.5,  // Réduction forte de la pénalité L2 pour débloquer les splits marginaux
        gamma: 0.01,  // Quasi zéro : accepte de scinder l'arbre si un gain même infime existe
        subsample: 0.8,
        ..XGRegressorParameters::default()
    };

    let modele = XGRegressor::fit(&x_train, &labels_train, params)
        .map_err(|e| TradingError::ML(format!("Entraînement XGB Rockets: {}", e)))?;

    // Accuracy OOS
    let preds_oos = modele
        .predict(&x_oos)
        .map_err(|e| TradingError::ML(format!("Prédiction XGB Rockets OOS: {}", e)))?;
    let pred_cls: Vec<u8> = preds_oos.iter().map(|&p| if p >= 0.5 { 1 } else { 0 }).collect();
    let true_cls: Vec<u8> = labels_oos.iter().map(|&l| if l >= 0.5 { 1 } else { 0 }).collect();
    let accuracy_oos = accuracy(&true_cls, &pred_cls);

    tracing::info!(
        "XGB Rockets fine-tuning: {} samples ({} train / {} OOS) | accuracy_OOS={:.1}% | {:?}",
        nb,
        split,
        oos.len(),
        accuracy_oos * 100.0,
        debut.elapsed()
    );

    // Persistance si résultat acceptable
    let sauvegarde = if accuracy_oos >= 0.52 {
        let json = serde_json::to_string(&modele)
            .map_err(|e| TradingError::ML(format!("Sérialisation XGB Rockets: {}", e)))?;
        if let Some(parent) = std::path::Path::new(CHEMIN_XGB_ROCKETS).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(CHEMIN_XGB_ROCKETS, &json)
            .map_err(|e| TradingError::ML(format!("Écriture XGB Rockets: {}", e)))?;
        tracing::info!("XGB Rockets sauvegardé (OOS={:.1}%)", accuracy_oos * 100.0);
        true
    } else {
        tracing::warn!(
            "XGB Rockets non sauvegardé: OOS={:.1}% < 52%",
            accuracy_oos * 100.0
        );
        false
    };

    Ok(Some(ResultatFineTuning {
        accuracy_oos,
        nb_samples: nb,
        sauvegarde,
        importances: calculer_importances(&modele, &features_oos, &labels_oos, accuracy_oos),
    }))
}

/// Calcule l'importance de chaque feature par permutation sur le jeu OOS.
/// Pour chaque feature i : on permute sa colonne, on mesure la chute d'accuracy.
/// Chute importante = feature très prédictive.
fn calculer_importances(
    modele: &XGRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>,
    features_oos: &[Vec<f64>],
    _labels_oos: &[f64],
    _accuracy_base: f64, // Conservé pour compatibilité de signature
) -> Vec<ImportanceFeature> {
    let n = features_oos.len();
    if n < 5 {
        return Vec::new();
    }
    let nb_features = features_oos[0].len().min(FEATURE_NOMS.len());

    // Calcul des prédictions de base (continues) sur le jeu OOS
    let features_oos_vec = features_oos.to_vec();
    let x_base = match DenseMatrix::from_2d_vec(&features_oos_vec) {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    let preds_base = match modele.predict(&x_base) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut importances: Vec<ImportanceFeature> = (0..nb_features)
        .filter_map(|i| {
            let mut permute = features_oos.to_vec();
            // Permutation aléatoire ou circulaire pour casser la corrélation de la feature
            let mut col_values: Vec<f64> = permute.iter().map(|f| f[i]).collect();
            col_values.rotate_left(n / 2); // Décalage circulaire garanti sans random externe
            
            for (row, val) in permute.iter_mut().zip(col_values) {
                row[i] = val;
            }

            let x_perm = DenseMatrix::from_2d_vec(&permute).ok()?;
            let preds_perm = modele.predict(&x_perm).ok()?;
            
            // Mesure du décalage absolu moyen des prédictions (MAE Drift)
            // L'importance est l'ampleur moyenne du changement occasionné par la permutation
            let mae_drift: f64 = preds_base.iter().zip(preds_perm.iter())
                .map(|(&b, &p)| (b - p).abs())
                .sum::<f64>() / n as f64;

            Some(ImportanceFeature {
                feature_idx: i,
                feature_nom: FEATURE_NOMS[i],
                importance: mae_drift, // Plus le drift est grand, plus la feature est importante
            })
        })
        .collect();

    // Normalisation des importances pour que la somme fasse 1.0 au max (ou 100%)
    let sum_importance: f64 = importances.iter().map(|f| f.importance).sum();
    if sum_importance > 0.0 {
        for f in &mut importances {
            f.importance /= sum_importance;
        }
    }

    importances.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
    importances
}
