use super::{math::softmax, ModeleHybrideLstm, L1, L2, L3};

impl ModeleHybrideLstm {
    pub(super) fn entrainer_un_exemple(
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
        let h3_final = etats_l3
            .last()
            .map(|e| e.h.clone())
            .unwrap_or_else(|| vec![0.0; L3]);

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
            let dh3_t = if t == etats_l3.len() - 1 {
                dh3.clone()
            } else {
                vec![0.0f64; L3]
            };
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

    /// Normalise les gradients (clipping: norme max 5.0) + détecte NaN/inf
    pub(super) fn clip_et_verifier_poids(&mut self) -> bool {
        const GRAD_CLIP_NORM: f64 = 5.0;

        // L1: clipper + vérifier NaN
        let mut l1_norm_sq = 0.0f64;
        for w in self.l1.poids_mut() {
            if !w.is_finite() {
                tracing::error!("LSTM L1: poids NaN/inf détecté");
                return false;
            }
            l1_norm_sq += *w * *w;
        }
        let l1_norm = l1_norm_sq.sqrt();
        if l1_norm > GRAD_CLIP_NORM {
            let scale = GRAD_CLIP_NORM / (l1_norm + 1e-8);
            for w in self.l1.poids_mut() {
                *w *= scale;
            }
        }

        // L2: clipper + vérifier
        let mut l2_norm_sq = 0.0f64;
        for w in self.l2.poids_mut() {
            if !w.is_finite() {
                tracing::error!("LSTM L2: poids NaN/inf détecté");
                return false;
            }
            l2_norm_sq += *w * *w;
        }
        let l2_norm = l2_norm_sq.sqrt();
        if l2_norm > GRAD_CLIP_NORM {
            let scale = GRAD_CLIP_NORM / (l2_norm + 1e-8);
            for w in self.l2.poids_mut() {
                *w *= scale;
            }
        }

        // L3: clipper + vérifier
        let mut l3_norm_sq = 0.0f64;
        for w in self.l3.poids_mut() {
            if !w.is_finite() {
                tracing::error!("LSTM L3: poids NaN/inf détecté");
                return false;
            }
            l3_norm_sq += *w * *w;
        }
        let l3_norm = l3_norm_sq.sqrt();
        if l3_norm > GRAD_CLIP_NORM {
            let scale = GRAD_CLIP_NORM / (l3_norm + 1e-8);
            for w in self.l3.poids_mut() {
                *w *= scale;
            }
        }

        // Vérifier couche sortie (weights et biais)
        for row in &self.sortie.poids {
            for &w in row {
                if !w.is_finite() {
                    tracing::error!("LSTM Sortie: poids NaN/inf détecté");
                    return false;
                }
            }
        }
        for &b in &self.sortie.biais {
            if !b.is_finite() {
                tracing::error!("LSTM Sortie: biais NaN/inf détecté");
                return false;
            }
        }

        true
    }
}
