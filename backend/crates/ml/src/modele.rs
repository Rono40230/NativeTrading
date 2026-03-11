use crate::features::NB_FEATURES;
use common::{Direction, Result, TradingError};
use smartcore::ensemble::random_forest_classifier::{
    RandomForestClassifier, RandomForestClassifierParameters,
};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::metrics::accuracy;

/// Résultat d'inférence ML
#[derive(Debug, Clone)]
pub struct PredictionML {
    pub direction: Direction,
    /// Probabilité de la direction (0.5 = incertain, 0.8+ = confiant)
    pub confiance: f64,
    /// true si le modèle est suffisamment confiant (≥ 60%)
    pub est_confiant: bool,
}

const SEUIL_CONFIANCE: f64 = 0.60;
const N_VARIANTES_PROBA: usize = 11;

pub struct ModeleRandomForest {
    modele: Option<RandomForestClassifier<f64, u8, DenseMatrix<f64>, Vec<u8>>>,
    n_arbres: u16,
}

impl ModeleRandomForest {
    pub fn new(n_arbres: u16) -> Self {
        Self {
            modele: None,
            n_arbres,
        }
    }

    /// Entraîne le modèle sur un jeu de données features/labels
    pub fn entrainer(&mut self, features: &[Vec<f64>], labels: &[f64]) -> Result<f64> {
        if features.len() < 50 {
            return Err(TradingError::ML(
                "Minimum 50 échantillons requis pour l'entraînement".into(),
            ));
        }
        if features.len() != labels.len() {
            return Err(TradingError::ML(
                "features et labels de tailles différentes".into(),
            ));
        }

        let x = DenseMatrix::from_2d_vec(&features.to_vec())
            .map_err(|e| TradingError::ML(format!("Construction matrice: {}", e)))?;
        let y: Vec<u8> = labels
            .iter()
            .map(|&l| if l >= 0.5 { 1 } else { 0 })
            .collect();

        let params = RandomForestClassifierParameters::default()
            .with_n_trees(self.n_arbres)
            .with_max_depth(8)
            .with_min_samples_leaf(5);

        let modele = RandomForestClassifier::fit(&x, &y, params)
            .map_err(|e| TradingError::ML(format!("Erreur entraînement RF: {}", e)))?;

        // Accuracy sur le jeu d'entraînement
        let pred = modele
            .predict(&x)
            .map_err(|e| TradingError::ML(format!("Erreur prédiction RF: {}", e)))?;
        let acc = accuracy(&y, &pred);

        self.modele = Some(modele);
        tracing::info!(
            "RandomForest entraîné: {} arbres, accuracy={:.1}%",
            self.n_arbres,
            acc * 100.0
        );
        Ok(acc)
    }

    /// Inférence sur un vecteur de features
    pub fn predire(&self, features: &[f64]) -> Result<PredictionML> {
        let modele = self
            .modele
            .as_ref()
            .ok_or_else(|| TradingError::ML("Modèle non entraîné".into()))?;

        if features.len() != NB_FEATURES {
            return Err(TradingError::ML(format!(
                "Attendu {} features, reçu {}",
                NB_FEATURES,
                features.len()
            )));
        }

        let x = DenseMatrix::from_2d_vec(&vec![features.to_vec()])
            .map_err(|e| TradingError::ML(format!("Matrice inférence: {}", e)))?;
        let pred = modele
            .predict(&x)
            .map_err(|e| TradingError::ML(format!("Erreur inférence: {}", e)))?;

        let label = pred[0];
        let proba = self.estimer_proba(modele, features)?;

        let direction = if label == 1 {
            Direction::Long
        } else {
            Direction::Short
        };
        let confiance = if direction == Direction::Long {
            proba
        } else {
            1.0 - proba
        };

        Ok(PredictionML {
            direction,
            confiance,
            est_confiant: confiance >= SEUIL_CONFIANCE,
        })
    }

    /// Estime une probabilité par vote sur variantes légèrement perturbées
    fn estimer_proba(
        &self,
        modele: &RandomForestClassifier<f64, u8, DenseMatrix<f64>, Vec<u8>>,
        features: &[f64],
    ) -> Result<f64> {
        let mut votes_long = 0usize;
        for k in 0..N_VARIANTES_PROBA {
            let facteur = 1.0 + (k as f64 - (N_VARIANTES_PROBA / 2) as f64) * 0.002;
            let variante: Vec<f64> = features.iter().map(|&v| v * facteur).collect();
            if let Ok(x) = DenseMatrix::from_2d_vec(&vec![variante]) {
                if let Ok(pred) = modele.predict(&x) {
                    if pred[0] == 1 {
                        votes_long += 1;
                    }
                }
            }
        }
        Ok(votes_long as f64 / N_VARIANTES_PROBA as f64)
    }

    pub fn est_pret(&self) -> bool {
        self.modele.is_some()
    }

    /// Sauvegarde le modèle RandomForest sur disque (JSON via serde)
    pub fn sauvegarder(&self, chemin: &str) -> Result<()> {
        let modele = self
            .modele
            .as_ref()
            .ok_or_else(|| TradingError::ML("Aucun modèle à sauvegarder".into()))?;
        let json = serde_json::to_string(modele)
            .map_err(|e| TradingError::ML(format!("Sérialisation RF: {}", e)))?;
        std::fs::write(chemin, json)
            .map_err(|e| TradingError::ML(format!("Écriture RF: {}", e)))?;
        tracing::info!("RandomForest sauvegardé: {}", chemin);
        Ok(())
    }

    /// Charge un modèle RandomForest depuis le disque
    pub fn charger(chemin: &str) -> Result<Self> {
        let json = std::fs::read_to_string(chemin)
            .map_err(|e| TradingError::ML(format!("Lecture RF: {}", e)))?;
        let modele: RandomForestClassifier<f64, u8, DenseMatrix<f64>, Vec<u8>> =
            serde_json::from_str(&json)
                .map_err(|e| TradingError::ML(format!("Désérialisation RF: {}", e)))?;
        tracing::info!("RandomForest chargé: {}", chemin);
        Ok(Self {
            modele: Some(modele),
            n_arbres: 100,
        })
    }
}
