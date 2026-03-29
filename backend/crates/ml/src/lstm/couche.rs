use serde::{Deserialize, Serialize};

use super::math::{d_sigmoid, d_tanh, lcg_next, sigmoid, xavier, EtatLstm};

const N_CLASSES: usize = 2;

// ─── Couche LSTM ──────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct CoucheLstm {
    /// Matrice combinée [4*cachee, entree+cachee] — portes i/f/g/o
    poids: Vec<f64>,
    /// Biais [4*cachee] — forget gate initialisé à 1.0
    biais: Vec<f64>,
    pub(super) entree: usize,
    pub(super) cachee: usize,
}

impl CoucheLstm {
    pub(super) fn nouveau(entree: usize, cachee: usize, g: &mut u64) -> Self {
        let cols = entree + cachee;
        let poids = xavier(4 * cachee, cols, g);
        let mut biais = vec![0.0f64; 4 * cachee];
        // Forget gate bias = 1.0 (améliore la mémorisation long-terme)
        for b in biais[cachee..2 * cachee].iter_mut() {
            *b = 1.0;
        }
        Self {
            poids,
            biais,
            entree,
            cachee,
        }
    }

    /// Passe avant avec enregistrement des états intermédiaires (pour BPTT)
    pub(super) fn step_avec_etat(&self, x: &[f64], h: &[f64], c: &[f64]) -> EtatLstm {
        let hi = self.cachee;
        let cols = self.entree + hi;
        let xh: Vec<f64> = x.iter().chain(h.iter()).copied().collect();

        let mut pre = self.biais.clone();
        for (r, p) in pre.iter_mut().enumerate() {
            let base = r * cols;
            for (j, &xv) in xh.iter().enumerate() {
                *p += self.poids[base + j] * xv;
            }
        }

        let ig: Vec<f64> = pre[0..hi].iter().map(|&v| sigmoid(v)).collect();
        let fg: Vec<f64> = pre[hi..2 * hi].iter().map(|&v| sigmoid(v)).collect();
        let gg: Vec<f64> = pre[2 * hi..3 * hi].iter().map(|&v| v.tanh()).collect();
        let og: Vec<f64> = pre[3 * hi..4 * hi].iter().map(|&v| sigmoid(v)).collect();

        let c_new: Vec<f64> = (0..hi).map(|k| fg[k] * c[k] + ig[k] * gg[k]).collect();
        let tanh_c: Vec<f64> = c_new.iter().map(|&v| v.tanh()).collect();
        let h_new: Vec<f64> = (0..hi).map(|k| og[k] * tanh_c[k]).collect();

        EtatLstm {
            x: x.to_vec(),
            h_prev: h.to_vec(),
            c_prev: c.to_vec(),
            i: ig,
            f: fg,
            g: gg,
            o: og,
            c: c_new,
            h: h_new,
        }
    }

    pub(super) fn step(&self, x: &[f64], h: &[f64], c: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let e = self.step_avec_etat(x, h, c);
        (e.h, e.c)
    }

    /// Passe avant → tous les états intermédiaires
    pub(super) fn forward_complet(&self, seq: &[Vec<f64>]) -> Vec<EtatLstm> {
        let mut h = vec![0.0f64; self.cachee];
        let mut c = vec![0.0f64; self.cachee];
        let mut etats = Vec::with_capacity(seq.len());
        for x in seq {
            let etat = self.step_avec_etat(x, &h, &c);
            h = etat.h.clone();
            c = etat.c.clone();
            etats.push(etat);
        }
        etats
    }

    /// Passe avant → états cachés par timestep (inférence)
    pub(super) fn forward_etats(&self, seq: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut h = vec![0.0f64; self.cachee];
        let mut c = vec![0.0f64; self.cachee];
        let mut etats = Vec::with_capacity(seq.len());
        for x in seq {
            let (h_new, c_new) = self.step(x, &h, &c);
            h = h_new.clone();
            c = c_new;
            etats.push(h_new);
        }
        etats
    }

    /// BPTT : calcule et applique les gradients. Retourne (dh_prev, dc_prev).
    pub(super) fn bptt_step(
        &mut self,
        etat: &EtatLstm,
        dh_out: &[f64],
        dc_next: &[f64],
        lr: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let hi = self.cachee;
        let cols = self.entree + hi;

        let tanh_c: Vec<f64> = etat.c.iter().map(|&v| v.tanh()).collect();

        let do_: Vec<f64> = (0..hi)
            .map(|k| dh_out[k] * tanh_c[k] * d_sigmoid(etat.o[k]))
            .collect();
        let dc: Vec<f64> = (0..hi)
            .map(|k| dh_out[k] * etat.o[k] * d_tanh(tanh_c[k]) + dc_next[k])
            .collect();
        let df: Vec<f64> = (0..hi)
            .map(|k| dc[k] * etat.c_prev[k] * d_sigmoid(etat.f[k]))
            .collect();
        let di: Vec<f64> = (0..hi)
            .map(|k| dc[k] * etat.g[k] * d_sigmoid(etat.i[k]))
            .collect();
        let dg: Vec<f64> = (0..hi)
            .map(|k| dc[k] * etat.i[k] * d_tanh(etat.g[k]))
            .collect();

        let dc_prev: Vec<f64> = (0..hi).map(|k| dc[k] * etat.f[k]).collect();

        let d_portes: Vec<f64> = di
            .iter()
            .chain(df.iter())
            .chain(dg.iter())
            .chain(do_.iter())
            .copied()
            .collect();
        let xh: Vec<f64> = etat.x.iter().chain(etat.h_prev.iter()).copied().collect();

        let mut d_xh = vec![0.0f64; cols];
        for (r, &dp) in d_portes.iter().enumerate() {
            let base = r * cols;
            for (j, &xv) in xh.iter().enumerate() {
                self.poids[base + j] -= lr * dp * xv;
            }
            self.biais[r] -= lr * dp;
            for (j, dxv) in d_xh.iter_mut().enumerate() {
                *dxv += dp * self.poids[base + j];
            }
        }

        let d_input: Vec<f64> = d_xh[..self.entree].to_vec();
        (d_input, dc_prev)
    }

    /// Référence vers les poids combinés [4*H, I+H] — accès depuis le module parent.
    #[cfg(feature = "cuda")]
    pub(super) fn poids_ref(&self) -> &[f64] {
        &self.poids
    }

    /// Référence vers le biais [4*H] — accès depuis le module parent.
    #[cfg(feature = "cuda")]
    pub(super) fn biais_ref(&self) -> &[f64] {
        &self.biais
    }

    /// Référence mutable pour gradient clipping (CPU + GPU).
    pub(super) fn poids_mut(&mut self) -> &mut [f64] {
        &mut self.poids
    }
}

// ─── Couche linéaire de sortie ────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct CoucheLinSortie {
    pub poids: Vec<Vec<f64>>, // [N_CLASSES, L3]
    pub biais: Vec<f64>,
}

impl CoucheLinSortie {
    pub(super) fn nouveau(entree: usize, g: &mut u64) -> Self {
        let scale = (2.0 / entree as f64).sqrt();
        let poids = (0..N_CLASSES)
            .map(|_| {
                (0..entree)
                    .map(|_| (lcg_next(g) * 2.0 - 1.0) * scale)
                    .collect()
            })
            .collect();
        Self {
            poids,
            biais: vec![0.0; N_CLASSES],
        }
    }

    pub(super) fn avant(&self, x: &[f64]) -> Vec<f64> {
        self.poids
            .iter()
            .zip(self.biais.iter())
            .map(|(row, &b)| {
                row.iter()
                    .zip(x.iter())
                    .map(|(&w, &xi)| w * xi)
                    .sum::<f64>()
                    + b
            })
            .collect()
    }
}
