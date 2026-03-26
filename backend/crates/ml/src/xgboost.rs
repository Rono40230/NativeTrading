use crate::features::NB_FEATURES;
use common::{Direction, Result, TradingError};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::metrics::accuracy;
use smartcore::xgboost::{XGRegressor, XGRegressorParameters};
use std::time::Instant;

/// Modèle XGBoost (Extreme Gradient Boosting) pour classification directionnelle.
///
/// Basé sur `smartcore::xgboost::XGRegressor` — pur Rust, gradient + hessian,
/// régularisation L1/L2 pour éviter l'overfitting.
/// Entraîné en régression (labels 0.0/1.0), inférence via seuil 0.5.
pub struct ModeleXGBoost {
    modele: Option<XGRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>,
    n_estimateurs: usize,
}

impl ModeleXGBoost {
    pub fn new(n_estimateurs: usize) -> Self {
        Self {
            modele: None,
            n_estimateurs,
        }
    }

    /// Entraîne le modèle sur features/labels (0.0=Short, 1.0=Long).
    /// Retourne l'accuracy sur le jeu d'entraînement.
    pub fn entrainer(&mut self, features: &[Vec<f64>], labels: &[f64]) -> Result<f64> {
        if features.len() < 50 {
            return Err(TradingError::ML(
                "Minimum 50 échantillons requis pour XGBoost".into(),
            ));
        }
        if features.len() != labels.len() {
            return Err(TradingError::ML(
                "features et labels de tailles différentes".into(),
            ));
        }

        let debut = Instant::now();

        let x = DenseMatrix::from_2d_vec(&features.to_vec())
            .map_err(|e| TradingError::ML(format!("Matrice XGBoost: {}", e)))?;
        let y: Vec<f64> = labels.to_vec();

        let params = XGRegressorParameters {
            n_estimators: self.n_estimateurs,
            max_depth: 6,
            learning_rate: 0.1,
            lambda: 1.0,
            gamma: 0.1,
            subsample: 0.8,
            ..XGRegressorParameters::default()
        };

        let modele = XGRegressor::fit(&x, &y, params)
            .map_err(|e| TradingError::ML(format!("Entraînement XGBoost: {}", e)))?;

        // Accuracy : seuil 0.5 sur les prédictions de régression
        let preds = modele
            .predict(&x)
            .map_err(|e| TradingError::ML(format!("Prédiction XGBoost train: {}", e)))?;
        let pred_classes: Vec<u8> = preds.iter().map(|&p| if p >= 0.5 { 1 } else { 0 }).collect();
        let true_classes: Vec<u8> = y.iter().map(|&l| if l >= 0.5 { 1 } else { 0 }).collect();
        let acc = accuracy(&true_classes, &pred_classes);

        self.modele = Some(modele);
        tracing::info!(
            "XGBoost entraîné: {} estimateurs, accuracy={:.1}% en {:?}",
            self.n_estimateurs,
            acc * 100.0,
            debut.elapsed()
        );
        Ok(acc)
    }

    /// Inférence sur un vecteur de features.
    /// Retourne le score brut de probabilité Long [0, 1].
    pub fn predire_score(&self, features: &[f64]) -> Result<f64> {
        let modele = self
            .modele
            .as_ref()
            .ok_or_else(|| TradingError::ML("XGBoost non entraîné".into()))?;

        if features.len() != NB_FEATURES {
            return Err(TradingError::ML(format!(
                "Attendu {} features, reçu {}",
                NB_FEATURES,
                features.len()
            )));
        }

        let x = DenseMatrix::from_2d_vec(&vec![features.to_vec()])
            .map_err(|e| TradingError::ML(format!("Matrice inférence XGB: {}", e)))?;
        let preds = modele
            .predict(&x)
            .map_err(|e| TradingError::ML(format!("Inférence XGBoost: {}", e)))?;

        Ok(preds[0].clamp(0.0, 1.0))
    }

    /// Inférence complète : retourne (direction, confiance).
    pub fn predire(&self, features: &[f64]) -> Result<(Direction, f64)> {
        let score_long = self.predire_score(features)?;
        let direction = if score_long >= 0.5 {
            Direction::Long
        } else {
            Direction::Short
        };
        let confiance = if direction == Direction::Long {
            score_long
        } else {
            1.0 - score_long
        };
        Ok((direction, confiance))
    }

    pub fn est_pret(&self) -> bool {
        self.modele.is_some()
    }

    /// Sauvegarde le modèle sur disque via serde_json.
    pub fn sauvegarder(&self, chemin: &str) -> Result<()> {
        let modele = self
            .modele
            .as_ref()
            .ok_or_else(|| TradingError::ML("Aucun modèle XGBoost à sauvegarder".into()))?;
        let json = serde_json::to_string(modele)
            .map_err(|e| TradingError::ML(format!("Sérialisation XGBoost: {}", e)))?;
        std::fs::write(chemin, json)
            .map_err(|e| TradingError::ML(format!("Écriture XGBoost: {}", e)))?;
        tracing::info!("XGBoost sauvegardé: {}", chemin);
        Ok(())
    }

    /// Charge un modèle XGBoost depuis le disque.
    pub fn charger(chemin: &str) -> Result<Self> {
        let json = std::fs::read_to_string(chemin)
            .map_err(|e| TradingError::ML(format!("Lecture XGBoost: {}", e)))?;
        let modele: XGRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> =
            serde_json::from_str(&json)
                .map_err(|e| TradingError::ML(format!("Désérialisation XGBoost: {}", e)))?;
        tracing::info!("XGBoost chargé: {}", chemin);
        Ok(Self {
            modele: Some(modele),
            n_estimateurs: 100,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dataset_synthetique(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let features: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..NB_FEATURES)
                    .map(|j| (i as f64 * 0.1 + j as f64 * 0.05).sin())
                    .collect()
            })
            .collect();
        let labels: Vec<f64> = (0..n).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
        (features, labels)
    }

    #[test]
    fn test_xgboost_entrainement_inference() {
        let mut modele = ModeleXGBoost::new(10);
        let (features, labels) = dataset_synthetique(100);
        let acc = modele.entrainer(&features, &labels).unwrap();
        assert!((0.0..=1.0).contains(&acc));
        assert!(modele.est_pret());

        let score = modele.predire_score(&features[0]).unwrap();
        assert!((0.0..=1.0).contains(&score));

        let (direction, confiance) = modele.predire(&features[0]).unwrap();
        assert!(confiance >= 0.5 && confiance <= 1.0);
        assert!(matches!(direction, Direction::Long | Direction::Short));
    }

    #[test]
    fn test_xgboost_min_echantillons() {
        let mut modele = ModeleXGBoost::new(10);
        let features = vec![vec![0.0; NB_FEATURES]; 10];
        let labels = vec![1.0; 10];
        assert!(modele.entrainer(&features, &labels).is_err());
    }

    #[test]
    fn test_xgboost_non_entraine() {
        let modele = ModeleXGBoost::new(10);
        assert!(!modele.est_pret());
        let features = vec![0.0; NB_FEATURES];
        assert!(modele.predire_score(&features).is_err());
    }
}
