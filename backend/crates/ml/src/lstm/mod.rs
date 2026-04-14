mod couche;
mod entrainement;
mod math;

#[cfg(feature = "cuda")]
pub(crate) mod gpu;
#[cfg(feature = "cuda")]
pub(crate) mod entrainement_gpu;

pub use couche::CoucheLinSortie;
#[cfg(feature = "cuda")]
pub(crate) use gpu::LstmGpu;

use common::{Result, TradingError};
use couche::CoucheLstm;
use math::softmax;
use serde::{Deserialize, Serialize};

/// Poids bruts extraits des 3 couches LSTM + sortie.
/// Permet le transfert CPU → GPU sans exposer `CoucheLstm` hors du module.
#[cfg(feature = "cuda")]
pub(crate) struct PoidsCouches {
    pub l1_poids: Vec<f64>,
    pub l1_biais: Vec<f64>,
    pub l1_in: usize,
    pub l1_h: usize,
    pub l2_poids: Vec<f64>,
    pub l2_biais: Vec<f64>,
    pub l2_in: usize,
    pub l2_h: usize,
    pub l3_poids: Vec<f64>,
    pub l3_biais: Vec<f64>,
    pub l3_in: usize,
    pub l3_h: usize,
    pub sortie_poids: Vec<Vec<f64>>,
    pub sortie_biais: Vec<f64>,
}

/// Longueur de séquence : 10 timesteps (dernières 10 fenêtres de features)
pub const LONGUEUR_SEQ: usize = 10;

const L1: usize = 128;
const L2: usize = 64;
const L3: usize = 32;

// ─── Modèle LSTM 3 couches (128→64→32) ───────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct ModeleHybrideLstm {
    l1: CoucheLstm,
    l2: CoucheLstm,
    l3: CoucheLstm,
    pub sortie: CoucheLinSortie,
    entraine: bool,
}

impl ModeleHybrideLstm {
    pub fn nouveau(nb_features: usize) -> Self {
        let mut g = 2026_u64;
        Self {
            l1: CoucheLstm::nouveau(nb_features, L1, &mut g),
            l2: CoucheLstm::nouveau(L1, L2, &mut g),
            l3: CoucheLstm::nouveau(L2, L3, &mut g),
            sortie: CoucheLinSortie::nouveau(L3, &mut g),
            entraine: false,
        }
    }

    /// Extrait le vecteur LSTM final (dernier état caché L3)
    pub fn avant_lstm(&self, seq: &[Vec<f64>]) -> Vec<f64> {
        let h1 = self.l1.forward_etats(seq);
        let h2 = self.l2.forward_etats(&h1);
        let h3 = self.l3.forward_etats(&h2);
        h3.into_iter().last().unwrap_or_else(|| vec![0.0; L3])
    }

    /// Probabilité Long ∈ [0,1]
    pub fn predire(&self, seq: &[Vec<f64>]) -> f64 {
        let h = self.avant_lstm(seq);
        let logits = self.sortie.avant(&h);
        softmax(&logits)[1] // index 1 = Long
    }

    /// BPTT complet sur les 3 couches + couche de sortie avec early stopping
    pub fn entrainer(
        &mut self,
        sequences: &[Vec<Vec<f64>>],
        labels: &[f64],
        epochs: usize,
        lr: f64,
    ) -> f64 {
        if sequences.is_empty() {
            return 0.0;
        }
        let mut acc = 0.0;
        let mut best_acc = 0.0;
        let mut epochs_sans_amelioration = 0usize;
        const PATIENCE: usize = 3;

        for epoch in 0..epochs {
            let mut correct = 0usize;
            for (seq, &label) in sequences.iter().zip(labels.iter()) {
                self.entrainer_un_exemple(seq, label, lr, &mut correct);
            }
            acc = correct as f64 / sequences.len() as f64;

            // Gradient clipping + détection NaN
            if !self.clip_et_verifier_poids() {
                tracing::error!("LSTM: divergence détectée à epoch {}", epoch + 1);
                break;
            }

            // Early stopping
            if acc > best_acc {
                best_acc = acc;
                epochs_sans_amelioration = 0;
            } else {
                epochs_sans_amelioration += 1;
            }

            if epochs_sans_amelioration >= PATIENCE {
                tracing::info!(
                    "LSTM early stopping epoch {}: best_acc={:.1}% (patience={})",
                    epoch + 1,
                    best_acc * 100.0,
                    PATIENCE
                );
                break;
            }

            if epoch > 0 && epoch % 5 == 0 {
                tracing::debug!(
                    "LSTM BPTT epoch {}/{}: acc={:.1}% (best={:.1}%)",
                    epoch + 1,
                    epochs,
                    acc * 100.0,
                    best_acc * 100.0
                );
            }
        }
        self.entraine = true;
        acc
    }

    pub fn est_pret(&self) -> bool {
        self.entraine
    }

    pub fn sauvegarder(&self, chemin: &str) -> Result<()> {
        let json = serde_json::to_string(self)
            .map_err(|e| TradingError::ML(format!("Sérialisation LSTM: {}", e)))?;
        std::fs::write(chemin, json)
            .map_err(|e| TradingError::ML(format!("Écriture LSTM: {}", e)))?;
        Ok(())
    }

    pub fn charger(chemin: &str) -> Result<Self> {
        let json = std::fs::read_to_string(chemin)
            .map_err(|e| TradingError::ML(format!("Lecture LSTM: {}", e)))?;
        serde_json::from_str(&json)
            .map_err(|e| TradingError::ML(format!("Désérialisation LSTM: {}", e)))
    }

    /// Applique les poids rapatriés depuis le GPU vers le modèle CPU sérialisable.
    #[cfg(feature = "cuda")]
    pub(crate) fn appliquer_poids_depuis_gpu(&mut self, p: PoidsCouches) {
        self.l1.set_poids_biais(p.l1_poids, p.l1_biais);
        self.l2.set_poids_biais(p.l2_poids, p.l2_biais);
        self.l3.set_poids_biais(p.l3_poids, p.l3_biais);
        self.sortie.poids = p.sortie_poids;
        self.sortie.biais = p.sortie_biais;
        self.entraine = true;
    }

    /// Extrait les poids pour transfert vers GPU (feature `cuda` uniquement).
    #[cfg(feature = "cuda")]
    pub(crate) fn extraire_poids_gpu(&self) -> PoidsCouches {
        PoidsCouches {
            l1_poids: self.l1.poids_ref().to_vec(),
            l1_biais: self.l1.biais_ref().to_vec(),
            l1_in: self.l1.entree,
            l1_h: self.l1.cachee,
            l2_poids: self.l2.poids_ref().to_vec(),
            l2_biais: self.l2.biais_ref().to_vec(),
            l2_in: self.l2.entree,
            l2_h: self.l2.cachee,
            l3_poids: self.l3.poids_ref().to_vec(),
            l3_biais: self.l3.biais_ref().to_vec(),
            l3_in: self.l3.entree,
            l3_h: self.l3.cachee,
            sortie_poids: self.sortie.poids.clone(),
            sortie_biais: self.sortie.biais.clone(),
        }
    }
}
