use std::time::Instant;
use common::{Candle, Direction, Result, TradingError};

use crate::xgboost::ModeleXGBoost;
use crate::lstm::{ModeleHybrideLstm, LONGUEUR_SEQ};
use crate::rockets_trainer::XgbRockets;
use crate::smc_trainer::XgbSmc;
use crate::straddle_trainer::XgbStraddle;
use crate::features::{extraire_features, NB_FEATURES};
use crate::features_corrompues;

pub const CHEMIN_XGB: &str = "data/modele_xgboost.json";
pub const CHEMIN_LSTM: &str = "data/modele_lstm.json";
pub const CHEMIN_LSTM_PT: &str = "data/modele_lstm.pt";

#[derive(Debug, Clone)]
pub struct PredictionML {
    pub direction: Direction,
    pub confiance: f64,
    pub est_confiant: bool,
}

pub struct PipelineML {
    pub xgb: ModeleXGBoost,
    pub lstm: ModeleHybrideLstm,
    pub xgb_rockets: XgbRockets,
    pub xgb_straddle: XgbStraddle,
    pub xgb_smc: XgbSmc,
    #[cfg(feature = "cuda")]
    pub lstm_gpu: Option<crate::lstm::LstmGpu>,
}

impl PipelineML {
    pub fn new() -> Self {
        Self {
            xgb: ModeleXGBoost::new(50),
            lstm: ModeleHybrideLstm::nouveau(NB_FEATURES),
            xgb_rockets: XgbRockets::charger_depuis_disque(),
            xgb_straddle: XgbStraddle::charger_depuis_disque(),
            xgb_smc: XgbSmc::charger_depuis_disque(),
            #[cfg(feature = "cuda")]
            lstm_gpu: None,
        }
    }

    #[cfg(feature = "cuda")]
    pub fn activer_gpu_si_pret(&mut self) {
        self.lstm_gpu = crate::lstm::LstmGpu::depuis_pt(CHEMIN_LSTM_PT);
        if self.lstm_gpu.is_some() {
            tracing::info!("LSTM GPU: tenseurs chargés sur CUDA:0 depuis .pt");
        }
    }

    pub fn charger_depuis_disque(&mut self) -> Result<bool> {
        let xgb_charge = match ModeleXGBoost::charger(CHEMIN_XGB) {
            Ok(xgb) => {
                self.xgb = xgb;
                true
            }
            Err(e) => {
                tracing::debug!("XGBoost non chargé depuis disque: {}", e);
                false
            }
        };

        if let Ok(lstm) = ModeleHybrideLstm::charger(CHEMIN_LSTM) {
            self.lstm = lstm;
            tracing::info!("LSTM chargé depuis disque");
        }

        if xgb_charge {
            tracing::info!("Pipeline ML XGBoost+LSTM rechargé depuis disque");
        }
        self.xgb_rockets = XgbRockets::charger_depuis_disque();
        #[cfg(feature = "cuda")]
        self.activer_gpu_si_pret();
        Ok(xgb_charge)
    }

    pub fn sauvegarder_sur_disque(&self) -> Result<()> {
        if let Some(parent) = std::path::Path::new(CHEMIN_XGB).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TradingError::ML(format!("Création dossier modèles: {}", e)))?
        }
        self.xgb.sauvegarder(CHEMIN_XGB)?;
        self.lstm.sauvegarder(CHEMIN_LSTM)?;
        Ok(())
    }

    pub fn entrainer_sur_historique(
        &mut self,
        bougies: &[Candle],
        horizon: usize,
        seuil_pct: f64,
    ) -> Result<(f64, f64)> {
        crate::pipeline_training::entrainer_sur_historique(self, bougies, horizon, seuil_pct)
    }

    pub fn predire(&self, bougies: &[Candle]) -> Result<PredictionML> {
        let debut = Instant::now();

        let f = extraire_features(bougies)
            .ok_or_else(|| TradingError::ML("Pas assez de bougies (min 60)".into()))?;

        if features_corrompues(&f) {
            return Err(TradingError::ML("Features invalides (NaN/Inf) — asset corrompu".into()));
        }

        let pred =
            if self.xgb.est_pret() && self.lstm.est_pret() && bougies.len() >= 60 + LONGUEUR_SEQ {
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

                    let conf_long = if let Some(score_rockets) = self.xgb_rockets.predire_score(&f) {
                        0.4 * conf_long_lstm + 0.3 * score_xgb + 0.3 * score_rockets
                    } else {
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
                    let (direction, confiance) = self.xgb.predire(&f)?;
                    PredictionML {
                        direction,
                        confiance,
                        est_confiant: confiance >= 0.60,
                    }
                }
            } else if self.xgb.est_pret() {
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
