use common::{Direction, Result, TradingError};
use std::fs;
use xgb::{
    parameters::{self, learning, tree, BoosterType},
    Booster, DMatrix,
};
use std::path::Path;

/// Nombre de features standard OHLCV par bougie (utilise la config du projet).
/// Ici, on doit s'assurer que c'est en accord avec le reste.
pub const NB_FEATURES: usize = 52; 

/// Modèle hybride pour XGBoost (Migration C++ binding)
pub struct ModeleXGBoost {
    modele: Option<Booster>,
    pub n_estimateurs: usize,
}

// SAFETY: XGBoost C++ est thread-safe pour des instances Booster indépendantes.
// Le borrow checker Rust garantit qu'un seul thread accède à la même instance à la fois.
unsafe impl Send for ModeleXGBoost {}
unsafe impl Sync for ModeleXGBoost {}

impl ModeleXGBoost {
    pub fn new(n_estimateurs: usize) -> Self {
        Self {
            modele: None,
            n_estimateurs,
        }
    }

    /// Convertit les données en format natif XGBoost (DMatrix).
    /// xgb 3.x attend des f32 pour from_dense (contrairement à xgboost 0.1.x).
    fn creer_dmatrix(features: &[Vec<f64>], labels: Option<&[f64]>) -> Result<DMatrix> {
        let n_lignes = features.len();
        if n_lignes == 0 {
            return Err(TradingError::ML("Dataset vide pour DMatrix".into()));
        }
        let n_cols = features[0].len();

        // xgb 3.x attend &[f32] pour from_dense
        let mut flat_features: Vec<f32> = Vec::with_capacity(n_lignes * n_cols);
        for ligne in features {
            for &v in ligne.iter() {
                flat_features.push(v as f32);
            }
        }

        let mut dmat = DMatrix::from_dense(&flat_features, n_lignes)
            .map_err(|e| TradingError::ML(format!("Erreur DMatrix features: {}", e)))?;

        if let Some(lbls) = labels {
            let flat_labels: Vec<f32> = lbls.iter().map(|&l| l as f32).collect();
            dmat.set_labels(&flat_labels)
                .map_err(|e| TradingError::ML(format!("Erreur DMatrix labels: {}", e)))?;
        }

        Ok(dmat)
    }

    /// Entraîne le modèle sur GPU (Phase 3 — entraînement final, régularisation activée).
    pub fn entrainer(&mut self, features: &[Vec<f64>], labels: &[f64]) -> Result<f64> {
        self.entrainer_impl(features, labels, true)
    }

    /// Entraîne le modèle sur CPU uniquement (Phase 2 — Walk-Forward, modèle temporaire).
    /// Pas de CUDA overhead, pas de régularisation — ~3× plus rapide pour petits datasets.
    pub fn entrainer_cpu(&mut self, features: &[Vec<f64>], labels: &[f64]) -> Result<f64> {
        self.entrainer_impl(features, labels, false)
    }

    fn entrainer_impl(&mut self, features: &[Vec<f64>], labels: &[f64], use_gpu: bool) -> Result<f64> {
        if features.is_empty() || labels.is_empty() {
            return Err(TradingError::ML("Données d'entraînement vides".into()));
        }
        if features.len() != labels.len() {
            return Err(TradingError::ML("Taille features != labels".into()));
        }

        let dmat = Self::creer_dmatrix(features, Some(labels))?;

        let tree_params = tree::TreeBoosterParametersBuilder::default()
            .max_depth(6)
            .eta(0.05)
            .build()
            .map_err(|e| TradingError::ML(format!("Params arbre XGBoost: {}", e)))?;

        let learning_params = learning::LearningTaskParametersBuilder::default()
            .objective(learning::Objective::BinaryLogistic)
            .build()
            .map_err(|e| TradingError::ML(format!("Params apprentissage XGBoost: {}", e)))?;

        let booster_params = parameters::BoosterParametersBuilder::default()
            .booster_type(BoosterType::Tree(tree_params))
            .learning_params(learning_params)
            .verbose(false)
            .build()
            .map_err(|e| TradingError::ML(format!("Params booster XGBoost: {}", e)))?;

        let mut model = Booster::new_with_cached_dmats(&booster_params, &[&dmat])
            .map_err(|e| TradingError::ML(format!("Création Booster XGBoost: {}", e)))?;

        model.set_param("tree_method", "hist")
            .map_err(|e| TradingError::ML(format!("XGBoost set_param tree_method: {}", e)))?;
        model.set_param("nthread", "1")
            .map_err(|e| TradingError::ML(format!("XGBoost set_param nthread: {}", e)))?;
        model.set_param("eval_metric", "logloss")
            .map_err(|e| TradingError::ML(format!("XGBoost set_param eval_metric: {}", e)))?;

        if use_gpu {
            // Phase 3 : GPU + régularisation pour éviter XGB=100% overfitting
            model.set_param("device", "cuda:0")
                .map_err(|e| TradingError::ML(format!("XGBoost set_param device: {}", e)))?;
            model.set_param("subsample", "0.8")
                .map_err(|e| TradingError::ML(format!("XGBoost set_param subsample: {}", e)))?;
            model.set_param("colsample_bytree", "0.8")
                .map_err(|e| TradingError::ML(format!("XGBoost set_param colsample_bytree: {}", e)))?;
            model.set_param("min_child_weight", "3")
                .map_err(|e| TradingError::ML(format!("XGBoost set_param min_child_weight: {}", e)))?;
            model.set_param("lambda", "2.0")
                .map_err(|e| TradingError::ML(format!("XGBoost set_param lambda: {}", e)))?;
            model.set_param("gamma", "0.05")
                .map_err(|e| TradingError::ML(format!("XGBoost set_param gamma: {}", e)))?;
        }
        // Phase 2 CPU : pas de CUDA, pas de régularisation — modèle temporaire OOS uniquement

        for i in 0..self.n_estimateurs as i32 {
            model.update(&dmat, i)
                .map_err(|e| TradingError::ML(format!("Erreur update round {} XGBoost: {}", i, e)))?;
        }

        self.modele = Some(model);
        Ok(1.0)
    }

    /// Prédit la probabilité brute (score pour classe Long/1).
    pub fn predire_score(&self, features: &[f64]) -> Result<f64> {
        let booster = self
            .modele
            .as_ref()
            .ok_or_else(|| TradingError::ML("Modèle non entraîné".into()))?;

        // XGBoost attend un tableau 2D, ici on prapare 1 ligne:
        let wrap = vec![features.to_vec()];
        let dmat = Self::creer_dmatrix(&wrap, None)?;

        let preds = booster.predict(&dmat)
            .map_err(|e| TradingError::ML(format!("Inférence XGBoost: {}", e)))?;

        if preds.is_empty() {
             return Err(TradingError::ML("Inférence XGBoost vide".into()));
        }

        // Preds = liste de probabilités pour binary_logistic
        Ok(preds[0].clamp(0.0, 1.0) as f64)
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

    /// Sauvegarde le modèle sur disque en format natif json xgb.
    pub fn sauvegarder(&self, chemin: &str) -> Result<()> {
        let booster = self
            .modele
            .as_ref()
            .ok_or_else(|| TradingError::ML("Aucun modèle XGBoost à sauvegarder".into()))?;
        
        let path = Path::new(chemin);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        booster.save(path)
            .map_err(|e| TradingError::ML(format!("Écriture XGBoost: {}", e)))?;

        tracing::info!("XGBoost sauvegardé: {}", chemin);
        Ok(())
    }

    /// Charge un modèle XGBoost depuis le disque.
    pub fn charger(chemin: &str) -> Result<Self> {
        // En XGBoost natif, on load directement le path
        let booster = Booster::load(chemin)
            .map_err(|e| TradingError::ML(format!("Lecture/Désérialisation XGBoost: {}", e)))?;
            
        tracing::info!("XGBoost chargé: {}", chemin);
        Ok(Self {
            modele: Some(booster),
            n_estimateurs: 100, // Val par defaut, ou le charger si on peut via properties
        })
    }
}
