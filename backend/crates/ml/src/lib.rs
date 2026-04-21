pub mod feature_noms;
pub mod features;
pub mod features_precalc;
pub mod feedback_analyser;
pub mod lstm;
pub mod params_suggester;
pub mod rockets_trainer;
pub mod smc_trainer;
pub mod straddle_trainer;
pub mod walk_forward;
pub mod xgboost;
pub mod pipeline;
pub mod pipeline_training;

pub use features::{extraire_features, labelliser, NB_FEATURES};
pub use feature_noms::FEATURE_NOMS;
pub use lstm::{ModeleHybrideLstm, LONGUEUR_SEQ};
pub use walk_forward::entrainer_walk_forward;
pub use xgboost::ModeleXGBoost;
pub use rockets_trainer::{entrainer_sur_trades_clotures, XgbRockets};
pub use smc_trainer::XgbSmc;
pub use straddle_trainer::XgbStraddle;
pub use pipeline::{PipelineML, PredictionML};

/// Vérifie qu'un vecteur de features ne contient pas de valeurs invalides (NaN, Inf).
/// Un asset avec un prix figé produit des features avec ATR=0, rendements=NaN, etc.
/// Appeler AVANT d'acquérir le lock pipeline_ml pour éviter les freezes.
#[inline]
pub fn features_corrompues(features: &[f64]) -> bool {
    features.iter().any(|v| v.is_nan() || v.is_infinite())
}

#[cfg(test)]
mod tests;
