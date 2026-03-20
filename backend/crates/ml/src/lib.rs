use std::time::Instant;

use common::{Candle, Direction, Result, TradingError};

pub mod features;
pub mod lstm;
pub mod modele;

pub use features::{extraire_features, labelliser, NB_FEATURES};
pub use lstm::{ModeleHybrideLstm, LONGUEUR_SEQ};
pub use modele::{ModeleRandomForest, PredictionML};

const CHEMIN_RF: &str = "data/modele_rf.json";
const CHEMIN_LSTM: &str = "data/modele_lstm.json";

/// Pipeline ML hybride : RandomForest (40%) + LSTM (60%)
pub struct PipelineML {
    pub modele: ModeleRandomForest,
    pub lstm: ModeleHybrideLstm,
}

impl PipelineML {
    pub fn new() -> Self {
        Self {
            modele: ModeleRandomForest::new(100),
            lstm: ModeleHybrideLstm::nouveau(NB_FEATURES),
        }
    }

    /// Tente de charger les modèles persistés depuis le disque.
    /// Retourne Ok(true) si au moins le RF est chargé, Ok(false) si aucun fichier trouvé.
    pub fn charger_depuis_disque(&mut self) -> Result<bool> {
        let rf_charge = match ModeleRandomForest::charger(CHEMIN_RF) {
            Ok(rf) => {
                self.modele = rf;
                true
            }
            Err(e) => {
                tracing::debug!(
                    "RF non chargé depuis disque (normal au 1er démarrage): {}",
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

        if rf_charge {
            tracing::info!("Pipeline ML rechargé depuis disque");
        }
        Ok(rf_charge)
    }

    /// Sauvegarde les modèles entraînés sur disque.
    pub fn sauvegarder_sur_disque(&self) -> Result<()> {
        // Créer le dossier data/ s'il n'existe pas encore
        if let Some(parent) = std::path::Path::new(CHEMIN_RF).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TradingError::ML(format!("Création dossier modèles: {}", e)))?;
        }
        self.modele.sauvegarder(CHEMIN_RF)?;
        self.lstm.sauvegarder(CHEMIN_LSTM)?;
        tracing::info!("Pipeline ML sauvegardé sur disque");
        Ok(())
    }

    /// Entraîne RF + LSTM sur l'historique. Retourne (accuracy_rf, accuracy_lstm).
    pub fn entrainer_sur_historique(
        &mut self,
        bougies: &[Candle],
        horizon: usize,
        seuil_pct: f64,
    ) -> Result<(f64, f64)> {
        tracing::info!("Entraînement hybride RF+LSTM sur {} bougies", bougies.len());
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

        // ── Entraînement RandomForest ──────────────────────────────────────────
        let acc_rf = self.modele.entrainer(&features_dataset, &labels)?;

        // ── Préparation séquences LSTM (T=10 timesteps) ───────────────────────
        let sequences: Vec<Vec<Vec<f64>>> = (LONGUEUR_SEQ..features_dataset.len())
            .map(|i| features_dataset[i - LONGUEUR_SEQ..i].to_vec())
            .collect();
        let labels_seq: Vec<f64> = labels[LONGUEUR_SEQ..].to_vec();

        let acc_lstm = self.lstm.entrainer(&sequences, &labels_seq, 15, 0.001);

        tracing::info!(
            "Pipeline hybride entraîné en {:?}: {} éch. RF={:.1}% LSTM={:.1}%",
            debut.elapsed(),
            features_dataset.len(),
            acc_rf * 100.0,
            acc_lstm * 100.0
        );

        // Persistance automatique après entraînement
        if let Err(e) = self.sauvegarder_sur_disque() {
            tracing::warn!("Échec sauvegarde pipeline ML: {}", e);
        }

        Ok((acc_rf, acc_lstm))
    }

    /// Inférence hybride — LSTM 60% + RF 40% si LSTM entraîné, sinon RF seul
    pub fn predire(&self, bougies: &[Candle]) -> Result<PredictionML> {
        let debut = Instant::now();

        let f = extraire_features(bougies)
            .ok_or_else(|| TradingError::ML("Pas assez de bougies (min 60)".into()))?;
        let pred_rf = self.modele.predire(&f)?;

        let pred = if self.lstm.est_pret() && bougies.len() >= 60 + LONGUEUR_SEQ {
            // Construire la séquence des LONGUEUR_SEQ derniers vecteurs de features
            let n = bougies.len();
            let sequence: Vec<Vec<f64>> = (n - LONGUEUR_SEQ..n)
                .filter_map(|i| extraire_features(&bougies[..=i]))
                .collect();

            if sequence.len() == LONGUEUR_SEQ {
                let conf_long_lstm = self.lstm.predire(&sequence);
                let conf_long_rf = if pred_rf.direction == Direction::Long {
                    pred_rf.confiance
                } else {
                    1.0 - pred_rf.confiance
                };
                // Fusion pondérée
                let conf_long = 0.6 * conf_long_lstm + 0.4 * conf_long_rf;
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
                pred_rf
            }
        } else {
            pred_rf
        };

        let duree = debut.elapsed();
        if duree > std::time::Duration::from_millis(200) {
            tracing::warn!("Inférence ML lente: {:?}", duree);
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

    fn bougie(c: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: c,
            high: c + 1.0,
            low: c - 1.0,
            close: c,
            volume: 100.0,
        }
    }

    #[test]
    fn pipeline_nouveau_non_pret() {
        let pipeline = PipelineML::new();
        assert!(
            !pipeline.est_pret(),
            "Un pipeline non entraîné ne doit pas être prêt"
        );
    }

    #[test]
    fn predire_erreur_si_pas_assez_de_bougies() {
        let pipeline = PipelineML::new();
        let bougies: Vec<Candle> = (1..=30).map(|i| bougie(i as f64)).collect();
        // Moins de 60 bougies → extraire_features retourne None → Err
        assert!(pipeline.predire(&bougies).is_err());
    }

    #[test]
    fn predire_erreur_si_modele_non_entraine() {
        let pipeline = PipelineML::new();
        // 60 bougies valides mais RF non entraîné
        let bougies: Vec<Candle> = (1..=70).map(|i| bougie(i as f64 * 100.0)).collect();
        assert!(pipeline.predire(&bougies).is_err());
    }
}
