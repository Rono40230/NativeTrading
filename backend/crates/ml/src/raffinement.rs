//! Raffinement du pipeline ML depuis les résultats de backtest.
use common::{Candle, Result};

use crate::{features::extraire_features, PipelineML};

impl PipelineML {
    /// Raffine le XGBoost depuis les trades réels d'un backtest.
    ///
    /// Pour chaque feedback `(indice_bougie, gagne)`, extrait les features de la bougie
    /// correspondante et entraîne une passe de calibration sur ces données validées.
    pub fn raffiner_depuis_backtest(
        &mut self,
        bougies: &[Candle],
        feedback: &[(usize, bool)],
    ) -> Result<usize> {
        if feedback.is_empty() {
            return Ok(0);
        }

        let mut features = Vec::new();
        let mut labels = Vec::new();

        for &(idx, gagne) in feedback {
            if idx < 60 || idx >= bougies.len() {
                continue;
            }
            if let Some(f) = extraire_features(&bougies[..=idx]) {
                features.push(f);
                labels.push(if gagne { 1.0 } else { 0.0 });
            }
        }

        let n = features.len();
        if n < 50 {
            tracing::info!(
                "Raffinement backtest: {} échantillons valides (min 50 pour XGB) — ignoré",
                n
            );
            return Ok(n);
        }

        match self.xgb.entrainer(&features, &labels) {
            Ok(acc) => tracing::info!(
                "Pipeline ML raffiné depuis backtest: {} trades → XGBoost accuracy={:.1}%",
                n,
                acc * 100.0
            ),
            Err(e) => {
                tracing::warn!("Raffinement backtest XGB échoué: {} — modèle inchangé", e);
                return Ok(0);
            }
        }

        if let Err(e) = self.sauvegarder_sur_disque() {
            tracing::warn!("Échec sauvegarde après raffinement backtest: {}", e);
        }

        Ok(n)
    }
}
