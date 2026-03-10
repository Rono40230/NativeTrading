use std::time::Instant;

use common::{Candle, Result, TradingError};

pub mod features;
pub mod modele;

pub use features::{extraire_features, labelliser, NB_FEATURES};
pub use modele::{ModeleRandomForest, PredictionML};

/// Pipeline ML complet : entraînement + inférence
pub struct PipelineML {
    pub modele: ModeleRandomForest,
}

impl PipelineML {
    pub fn new() -> Self {
        Self {
            modele: ModeleRandomForest::new(100),
        }
    }

    /// Entraîne sur l'historique complet (labels automatiques à horizon=5 bougies)
    pub fn entrainer_sur_historique(
        &mut self,
        bougies: &[Candle],
        horizon: usize,
        seuil_pct: f64,
    ) -> Result<f64> {
        tracing::info!("Entraînement ML sur {} bougies...", bougies.len());
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
            return Err(TradingError::ML("Aucun échantillon valide pour l'entraînement".into()));
        }

        let accuracy = self.modele.entrainer(&features_dataset, &labels)?;
        tracing::info!(
            "Pipeline ML entraîné en {:?}: {} échantillons, accuracy={:.1}%",
            debut.elapsed(),
            features_dataset.len(),
            accuracy * 100.0
        );
        Ok(accuracy)
    }

    /// Inférence sur les dernières bougies — renvoie prédiction + durée
    pub fn predire(&self, bougies: &[Candle]) -> Result<PredictionML> {
        let debut = Instant::now();
        let f = extraire_features(bougies)
            .ok_or_else(|| TradingError::ML("Pas assez de bougies (min 60)".into()))?;
        let pred = self.modele.predire(&f)?;
        let duree = debut.elapsed();

        if duree > std::time::Duration::from_millis(200) {
            tracing::warn!("Inférence ML lente: {:?}", duree);
        } else {
            tracing::debug!("Inférence ML: {:?}", duree);
        }
        Ok(pred)
    }

    pub fn est_pret(&self) -> bool {
        self.modele.est_pret()
    }
}

impl Default for PipelineML {
    fn default() -> Self {
        Self::new()
    }
}
