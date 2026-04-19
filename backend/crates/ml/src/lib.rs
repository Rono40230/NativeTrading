use std::time::Instant;

use common::{Candle, Direction, Result, TradingError};

pub mod feature_noms;
pub mod features;
pub mod feedback_analyser;
pub mod lstm;
pub mod params_suggester;
pub mod rockets_trainer;
pub mod smc_trainer;
pub mod straddle_trainer;
pub mod walk_forward;
pub mod xgboost;

pub use features::{extraire_features, labelliser, NB_FEATURES};
pub use feature_noms::FEATURE_NOMS;
pub use lstm::{ModeleHybrideLstm, LONGUEUR_SEQ};
pub use walk_forward::entrainer_walk_forward;
pub use xgboost::ModeleXGBoost;
pub use rockets_trainer::{entrainer_sur_trades_clotures, XgbRockets};
pub use smc_trainer::XgbSmc;
pub use straddle_trainer::XgbStraddle;

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

/// Pipeline ML hybride : XGBoost (40%) + LSTM (60%) + XGB Rockets fine-tuné
pub struct PipelineML {
    pub xgb: ModeleXGBoost,
    pub lstm: ModeleHybrideLstm,
    /// XGBoost fine-tuné sur les trades Rockets clôturés (P3). Optionnel.
    pub xgb_rockets: XgbRockets,
    /// XGBoost fine-tuné sur les trades Straddle clôturés (P13). Optionnel.
    pub xgb_straddle: XgbStraddle,
    /// XGBoost fine-tuné sur les trades SMC clôturés (P13). Optionnel.
    pub xgb_smc: XgbSmc,
    /// Cache GPU — reconstruit depuis les poids CPU. `None` si CUDA absent.
    #[cfg(feature = "cuda")]
    pub lstm_gpu: Option<lstm::LstmGpu>,
}

impl PipelineML {
    pub fn new() -> Self {
        Self {
            xgb: ModeleXGBoost::new(100),
            lstm: ModeleHybrideLstm::nouveau(NB_FEATURES),
            xgb_rockets: XgbRockets::charger_depuis_disque(),
            xgb_straddle: XgbStraddle::charger_depuis_disque(),
            xgb_smc: XgbSmc::charger_depuis_disque(),
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
        // Recharger XGB Rockets si disponible (silencieux si absent)
        self.xgb_rockets = XgbRockets::charger_depuis_disque();
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

        use rayon::prelude::*;
        let (features_dataset, labels): (Vec<_>, Vec<_>) = (60..bougies.len())
            .into_par_iter()
            .filter_map(|i| {
                let f = extraire_features(&bougies[..=i])?;
                let label = labelliser(bougies, i, horizon, seuil_pct)?;
                Some((f, label))
            })
            .unzip();

        if features_dataset.is_empty() {
            return Err(TradingError::ML("Aucun échantillon valide".into()));
        }

        // ── Entraînement XGBoost ──────────────────────────────────────────────────
        let acc_xgb = self.xgb.entrainer(&features_dataset, &labels)?;

        // ── Préparation séquences LSTM (T=10 timesteps) ───────────────────────
        let seq_total: Vec<Vec<Vec<f64>>> = (LONGUEUR_SEQ..features_dataset.len())
            .map(|i| features_dataset[i - LONGUEUR_SEQ..i].to_vec())
            .collect();
        let labels_seq_total: Vec<f64> = labels[LONGUEUR_SEQ..].to_vec();

        // GPU : cuDNN accéléré (RTX 3090). CPU fallback : plafond 5000 séquences.
        #[cfg(feature = "cuda")]
        let acc_lstm = if tch::Cuda::is_available() {
            match lstm::entrainement_gpu::entrainer_sur_gpu(
                &mut self.lstm, &seq_total, &labels_seq_total, 15, 0.001,
            ) {
                Ok(acc) => acc,
                Err(e) => {
                    tracing::warn!("LSTM GPU échoué, fallback CPU: {}", e);
                    const MAX: usize = 5_000;
                    let d = seq_total.len().saturating_sub(MAX);
                    self.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
                }
            }
        } else {
            const MAX: usize = 5_000;
            let d = seq_total.len().saturating_sub(MAX);
            self.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
        };
        #[cfg(not(feature = "cuda"))]
        let acc_lstm = {
            const MAX: usize = 5_000;
            let d = seq_total.len().saturating_sub(MAX);
            self.lstm.entrainer(&seq_total[d..], &labels_seq_total[d..], 15, 0.001)
        };

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

                    // Fusion pondérée selon disponibilité du modèle Rockets fine-tuné
                    let conf_long = if let Some(score_rockets) = self.xgb_rockets.predire_score(&f) {
                        // P3 disponible : LSTM 40% + XGB général 30% + XGB Rockets 30%
                        0.4 * conf_long_lstm + 0.3 * score_xgb + 0.3 * score_rockets
                    } else {
                        // P3 absent : LSTM 60% + XGB général 40%
                        0.6 * conf_long_lstm + 0.4 * score_xgb
                    };
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
