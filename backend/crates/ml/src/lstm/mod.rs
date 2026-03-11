mod couche;
mod math;

pub use couche::CoucheLinSortie;

use common::{Result, TradingError};
use couche::CoucheLstm;
use math::softmax;
use serde::{Deserialize, Serialize};

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

    /// BPTT complet sur les 3 couches + couche de sortie
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
        for epoch in 0..epochs {
            let mut correct = 0usize;
            for (seq, &label) in sequences.iter().zip(labels.iter()) {
                self.entrainer_un_exemple(seq, label, lr, &mut correct);
            }
            acc = correct as f64 / sequences.len() as f64;
            if epoch > 0 && epoch % 5 == 0 {
                tracing::debug!("LSTM BPTT epoch {}/{}: acc={:.1}%", epoch + 1, epochs, acc * 100.0);
            }
        }
        self.entraine = true;
        acc
    }

    fn entrainer_un_exemple(
        &mut self,
        seq: &[Vec<f64>],
        label: f64,
        lr: f64,
        correct: &mut usize,
    ) {
        let y = usize::from(label >= 0.5);

        let etats_l1 = self.l1.forward_complet(seq);
        let h1_seq: Vec<Vec<f64>> = etats_l1.iter().map(|e| e.h.clone()).collect();
        let etats_l2 = self.l2.forward_complet(&h1_seq);
        let h2_seq: Vec<Vec<f64>> = etats_l2.iter().map(|e| e.h.clone()).collect();
        let etats_l3 = self.l3.forward_complet(&h2_seq);
        let h3_final = etats_l3.last().map(|e| e.h.clone()).unwrap_or_else(|| vec![0.0; L3]);

        let logits = self.sortie.avant(&h3_final);
        let proba = softmax(&logits);

        if (proba[1] >= 0.5) == (y == 1) {
            *correct += 1;
        }

        // Gradient cross-entropy sur couche sortie
        let mut dh3 = vec![0.0f64; L3];
        for (out_i, &p_val) in proba.iter().enumerate() {
            let grad_logit = p_val - if out_i == y { 1.0 } else { 0.0 };
            for (j, (&hv, dh)) in h3_final.iter().zip(dh3.iter_mut()).enumerate() {
                *dh += grad_logit * self.sortie.poids[out_i][j];
                self.sortie.poids[out_i][j] -= lr * grad_logit * hv;
            }
            self.sortie.biais[out_i] -= lr * grad_logit;
        }

        // BPTT couche 3
        let mut dc3 = vec![0.0f64; L3];
        let mut dh2_seq = vec![vec![0.0f64; L2]; etats_l2.len()];
        for t in (0..etats_l3.len()).rev() {
            let dh3_t = if t == etats_l3.len() - 1 { dh3.clone() } else { vec![0.0f64; L3] };
            let (dh2_t, dc3_t) = self.l3.bptt_step(&etats_l3[t], &dh3_t, &dc3, lr);
            dc3 = dc3_t;
            if t < dh2_seq.len() {
                dh2_seq[t] = dh2_t;
            }
        }

        // BPTT couche 2
        let mut dc2 = vec![0.0f64; L2];
        let mut dh1_seq = vec![vec![0.0f64; L1]; etats_l1.len()];
        for t in (0..etats_l2.len()).rev() {
            let (dh1_t, dc2_t) = self.l2.bptt_step(&etats_l2[t], &dh2_seq[t], &dc2, lr);
            dc2 = dc2_t;
            if t < dh1_seq.len() {
                dh1_seq[t] = dh1_t;
            }
        }

        // BPTT couche 1
        let mut dc1 = vec![0.0f64; L1];
        for t in (0..etats_l1.len()).rev() {
            let (_dx, dc1_t) = self.l1.bptt_step(&etats_l1[t], &dh1_seq[t], &dc1, lr);
            dc1 = dc1_t;
        }


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
}
