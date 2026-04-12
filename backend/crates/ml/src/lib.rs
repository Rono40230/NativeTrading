use std::time::Instant;

use common::{Candle, Direction, Result, TradingError};

pub mod features;
pub mod feedback_analyser;
pub mod lstm;
pub mod params_suggester;
pub mod walk_forward;
pub mod xgboost;

pub use features::{extraire_features, labelliser, NB_FEATURES};
pub use lstm::{ModeleHybrideLstm, LONGUEUR_SEQ};
pub use walk_forward::entrainer_walk_forward;
pub use xgboost::ModeleXGBoost;

/// Résultat d'inférence du pipeline hybride XGBoost + LSTM
#[derive(Debug, Clone)]
pub struct PredictionML {
    pub direction: Direction,
    /// Probabilité de la direction (0.5 = incertain, 0.8+ = confiant)
    pub confiance: f64,
    /// true si le modèle est suffisamment confiant (≥ 60%)
    pub est_confiant: bool,
}

const CHEMIN_XGB: &str = "data/modele_xgboost.json";
const CHEMIN_LSTM: &str = "data/modele_lstm.json";

/// Pipeline ML hybride : XGBoost (40%) + LSTM (60%)
pub struct PipelineML {
    pub xgb: ModeleXGBoost,
    pub lstm: ModeleHybrideLstm,
    /// Cache GPU — reconstruit depuis les poids CPU. `None` si CUDA absent.
    #[cfg(feature = "cuda")]
    pub lstm_gpu: Option<lstm::LstmGpu>,
}

impl PipelineML {
    pub fn new() -> Self {
        Self {
            xgb: ModeleXGBoost::new(100),
            lstm: ModeleHybrideLstm::nouveau(NB_FEATURES),
            #[cfg(feature = "cuda")]
            lstm_gpu: None,
        }
    }

    /// Transfère les poids LSTM CPU → GPU CUDA. Sans effet si CUDA absent.
    #[cfg(feature = "cuda")]
    fn activer_gpu_si_pret(&mut self) {
        if self.lstm.est_pret() {
            self.lstm_gpu = lstm::LstmGpu::depuis_modele_cpu(&self.lstm);
            if self.lstm_gpu.is_some() {
                tracing::info!("LSTM GPU: tenseurs chargés sur CUDA:0");
            }
        }
    }

    /// Tente de charger les modèles persistés depuis le disque.
    /// Retourne Ok(true) si XGBoost est chargé, Ok(false) si aucun fichier trouvé.
    pub fn charger_depuis_disque(&mut self) -> Result<bool> {
        let xgb_charge = match ModeleXGBoost::charger(CHEMIN_XGB) {
            Ok(xgb) => {
                self.xgb = xgb;
                true
            }
            Err(e) => {
                tracing::debug!(
                    "XGBoost non chargé depuis disque (normal au 1er démarrage): {}",
                    e
                );
                false
            }
        };

        match ModeleHybrideLstm::charger(CHEMIN_LSTM) {
            Ok(lstm) => {
                self.lstm = lstm;
                tracing::info!("LSTM chargé depuis disque");
            }
            Err(e) => {
                tracing::debug!("LSTM non chargé depuis disque: {}", e);
            }
        }

        if xgb_charge {
            tracing::info!("Pipeline ML XGBoost+LSTM rechargé depuis disque");
        }
        #[cfg(feature = "cuda")]
        self.activer_gpu_si_pret();
        Ok(xgb_charge)
    }

    /// Sauvegarde les modèles entraînés sur disque.
    pub fn sauvegarder_sur_disque(&self) -> Result<()> {
        // Créer le dossier data/ s'il n'existe pas encore
        if let Some(parent) = std::path::Path::new(CHEMIN_XGB).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TradingError::ML(format!("Création dossier modèles: {}", e)))?
        }
        self.xgb.sauvegarder(CHEMIN_XGB)?;
        self.lstm.sauvegarder(CHEMIN_LSTM)?;
        tracing::info!("Pipeline ML XGBoost+LSTM sauvegardé sur disque");
        Ok(())
    }

    /// Entraîne XGBoost + LSTM sur l'historique. Retourne (accuracy_xgb, accuracy_lstm).
    pub fn entrainer_sur_historique(
        &mut self,
        bougies: &[Candle],
        horizon: usize,
        seuil_pct: f64,
    ) -> Result<(f64, f64)> {
        tracing::info!(
            "Entraînement hybride XGBoost+LSTM sur {} bougies",
            bougies.len()
        );
        let debut = Instant::now();

        let mut features_dataset = Vec::new();
        let mut labels = Vec::new();

        for i in 60..bougies.len() {
            let f = match extraire_features(&bougies[..=i]) {
                Some(f) => f,
                None => continue,
            };
            let label = match labelliser(bougies, i, horizon, seuil_pct) {
                Some(l) => l,
                None => continue,
            };
            features_dataset.push(f);
            labels.push(label);
        }

        if features_dataset.is_empty() {
            return Err(TradingError::ML("Aucun échantillon valide".into()));
        }

        // ── Entraînement XGBoost ──────────────────────────────────────────────────
        let acc_xgb = self.xgb.entrainer(&features_dataset, &labels)?;

        // ── Préparation séquences LSTM (T=10 timesteps) ───────────────────────
        let sequences: Vec<Vec<Vec<f64>>> = (LONGUEUR_SEQ..features_dataset.len())
            .map(|i| features_dataset[i - LONGUEUR_SEQ..i].to_vec())
            .collect();
        let labels_seq: Vec<f64> = labels[LONGUEUR_SEQ..].to_vec();

        let acc_lstm = self.lstm.entrainer(&sequences, &labels_seq, 15, 0.001);

        tracing::info!(
            "Pipeline hybride XGB+LSTM entraîné en {:?}: {} éch. XGB={:.1}% LSTM={:.1}%",
            debut.elapsed(),
            features_dataset.len(),
            acc_xgb * 100.0,
            acc_lstm * 100.0
        );

        // Persistance automatique après entraînement
        if let Err(e) = self.sauvegarder_sur_disque() {
            tracing::warn!("Échec sauvegarde pipeline ML: {}", e);
        }

        #[cfg(feature = "cuda")]
        self.activer_gpu_si_pret();
        Ok((acc_xgb, acc_lstm))
    }

    /// Inférence hybride — LSTM 60% + XGBoost 40% si entraînés, sinon XGB seul
    pub fn predire(&self, bougies: &[Candle]) -> Result<PredictionML> {
        let debut = Instant::now();

        let f = extraire_features(bougies)
            .ok_or_else(|| TradingError::ML("Pas assez de bougies (min 60)".into()))?;

        let pred =
            if self.xgb.est_pret() && self.lstm.est_pret() && bougies.len() >= 60 + LONGUEUR_SEQ {
                // Construire la séquence des LONGUEUR_SEQ derniers vecteurs de features
                let n = bougies.len();
                let sequence: Vec<Vec<f64>> = (n - LONGUEUR_SEQ..n)
                    .filter_map(|i| extraire_features(&bougies[..=i]))
                    .collect();

                if sequence.len() == LONGUEUR_SEQ {
                    #[cfg(feature = "cuda")]
                    let conf_long_lstm = self
                        .lstm_gpu
                        .as_ref()
                        .and_then(|g| g.predire(&sequence))
                        .unwrap_or_else(|| self.lstm.predire(&sequence));
                    #[cfg(not(feature = "cuda"))]
                    let conf_long_lstm = self.lstm.predire(&sequence);
                    let score_xgb = self.xgb.predire_score(&f).unwrap_or(0.5);
                    // Fusion pondérée : LSTM 60% + XGBoost 40%
                    let conf_long = 0.6 * conf_long_lstm + 0.4 * score_xgb;
                    let direction = if conf_long >= 0.5 {
                        Direction::Long
                    } else {
                        Direction::Short
                    };
                    let confiance = if direction == Direction::Long {
                        conf_long
                    } else {
                        1.0 - conf_long
                    };
                    PredictionML {
                        direction,
                        confiance,
                        est_confiant: confiance >= 0.60,
                    }
                } else {
                    // Fallback : XGBoost seul si séquence LSTM incomplète
                    let (direction, confiance) = self.xgb.predire(&f)?;
                    PredictionML {
                        direction,
                        confiance,
                        est_confiant: confiance >= 0.60,
                    }
                }
            } else if self.xgb.est_pret() {
                // XGBoost seul si LSTM non entraîné
                let (direction, confiance) = self.xgb.predire(&f)?;
                PredictionML {
                    direction,
                    confiance,
                    est_confiant: confiance >= 0.60,
                }
            } else {
                return Err(TradingError::ML("Pipeline ML non entraîné".into()));
            };

        let duree = debut.elapsed();
        if duree > std::time::Duration::from_millis(200) {
            tracing::warn!("Inférence ML lente: {:?}", duree);
        }
        Ok(pred)
    }

    pub fn est_pret(&self) -> bool {
        self.xgb.est_pret()
    }
}
impl Default for PipelineML {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests;
