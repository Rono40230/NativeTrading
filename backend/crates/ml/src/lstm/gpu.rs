//! Inférence LSTM accélérée GPU via tch (libtorch CUDA).
//!
//! Ce module ne compile qu'avec `--features cuda`. Il convertit les poids
//! CPU du [`ModeleHybrideLstm`] sérialisable en tenseurs f32 sur CUDA:0,
//! puis exécute la passe avant des 3 couches LSTM + softmax sur GPU.
//!
//! Usage :
//! ```ignore
//! let gpu = LstmGpu::depuis_modele_cpu(&pipeline.lstm);
//! if let Some(g) = gpu {
//!     let p_long = g.predire(&sequence);
//! }
//! ```

use tch::{Device, Kind, Tensor};

use super::{ModeleHybrideLstm, PoidsCouches};

// ─── Couche LSTM GPU ──────────────────────────────────────────────────────────

struct LstmGpuCouche {
    /// Matrice combinée [4*H, I+H] — f32 CUDA
    w: Tensor,
    /// Biais [4*H] — f32 CUDA
    b: Tensor,
    cachee: i64,
}

// ─── Accélérateur GPU ────────────────────────────────────────────────────────

/// LSTM 3 couches (128→64→32) inférence sur GPU CUDA.
///
/// Non-sérialisable — reconstruit depuis les poids CPU à chaque démarrage
/// ou après chaque entraînement, via [`LstmGpu::depuis_modele_cpu`].
pub struct LstmGpu {
    l1: LstmGpuCouche,
    l2: LstmGpuCouche,
    l3: LstmGpuCouche,
    /// Couche de sortie [2, L3] — f32 CUDA
    w_out: Tensor,
    b_out: Tensor,
    device: Device,
}

// SAFETY: LstmGpu est protégé par Arc<RwLock<PipelineML>> — jamais accédé
// concurrentiellement. tch::Tensor utilise des raw ptr CUDA thread-local safe.
unsafe impl Send for LstmGpu {}
unsafe impl Sync for LstmGpu {}

impl LstmGpu {
    /// Transfert des poids CPU vers tenseurs CUDA.
    ///
    /// Retourne `None` si CUDA n'est pas disponible sur la machine.
    pub fn depuis_modele_cpu(modele: &ModeleHybrideLstm) -> Option<Self> {
        if !tch::Cuda::is_available() {
            tracing::warn!("GPU LSTM: CUDA non disponible — inférence CPU conservée");
            return None;
        }
        let dev = Device::Cuda(0);
        let p: PoidsCouches = modele.extraire_poids_gpu();

        let n_out = p.sortie_poids.len() as i64;
        let m_out = p.sortie_poids.first().map_or(0, |r| r.len()) as i64;
        let w_out_flat: Vec<f32> = p
            .sortie_poids
            .iter()
            .flat_map(|r| r.iter().map(|&x| x as f32))
            .collect();
        let b_out_flat: Vec<f32> = p.sortie_biais.iter().map(|&x| x as f32).collect();

        Some(LstmGpu {
            l1: Self::build_couche(&p.l1_poids, &p.l1_biais, p.l1_in, p.l1_h, dev),
            l2: Self::build_couche(&p.l2_poids, &p.l2_biais, p.l2_in, p.l2_h, dev),
            l3: Self::build_couche(&p.l3_poids, &p.l3_biais, p.l3_in, p.l3_h, dev),
            w_out: Tensor::from_slice(&w_out_flat)
                .reshape(&[n_out, m_out])
                .to_device(dev),
            b_out: Tensor::from_slice(&b_out_flat).to_device(dev),
            device: dev,
        })
    }

    fn build_couche(
        poids: &[f64],
        biais: &[f64],
        input: usize,
        cachee: usize,
        dev: Device,
    ) -> LstmGpuCouche {
        let poids_f32: Vec<f32> = poids.iter().map(|&x| x as f32).collect();
        let biais_f32: Vec<f32> = biais.iter().map(|&x| x as f32).collect();
        LstmGpuCouche {
            w: Tensor::from_slice(&poids_f32)
                .reshape(&[4 * cachee as i64, (input + cachee) as i64])
                .to_device(dev),
            b: Tensor::from_slice(&biais_f32).to_device(dev),
            cachee: cachee as i64,
        }
    }

    /// Passe avant d'une couche LSTM — retourne la séquence des états cachés [T, H].
    fn forward_couche(&self, couche: &LstmGpuCouche, seq: &Tensor) -> Tensor {
        let t = seq.size()[0];
        let h = couche.cachee;
        let mut hidden = Tensor::zeros(&[1, h], (Kind::Float, self.device));
        let mut cell = Tensor::zeros(&[1, h], (Kind::Float, self.device));
        let mut states: Vec<Tensor> = Vec::with_capacity(t as usize);

        for step in 0..t {
            let x = seq.select(0, step).unsqueeze(0); // [1, I]
            let xh = Tensor::cat(&[x, hidden.copy()], 1); // [1, I+H]
            let pre = xh.mm(&couche.w.transpose(0, 1)) + &couche.b; // [1, 4H]

            let i_g = pre.narrow(1, 0, h).sigmoid();
            let f_g = pre.narrow(1, h, h).sigmoid();
            let g_g = pre.narrow(1, 2 * h, h).tanh();
            let o_g = pre.narrow(1, 3 * h, h).sigmoid();

            cell = f_g * cell + i_g * g_g;
            hidden = o_g * cell.tanh();
            states.push(hidden.squeeze_dim(0)); // [H]
        }
        Tensor::stack(&states, 0) // [T, H]
    }

    /// Inférence GPU : P(Long) ∈ [0,1].
    ///
    /// Retourne `None` si la séquence est vide ou en cas d'erreur GPU
    /// (le pipeline bascule alors automatiquement sur CPU).
    pub fn predire(&self, seq: &[Vec<f64>]) -> Option<f64> {
        if seq.is_empty() || seq[0].is_empty() {
            return None;
        }
        // Capture d'un éventuel panic tch pour garantir zero-panic en production
        let résultat = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tch::no_grad(|| self.predire_interne(seq))
        }));
        match résultat {
            Ok(p) => Some(p),
            Err(_) => {
                tracing::warn!("Inférence GPU LSTM: erreur tch inattendue — fallback CPU");
                None
            }
        }
    }

    fn predire_interne(&self, seq: &[Vec<f64>]) -> f64 {
        let t = seq.len() as i64;
        let i_dim = seq[0].len() as i64;
        let flat: Vec<f32> = seq
            .iter()
            .flat_map(|r| r.iter().map(|&x| x as f32))
            .collect();
        let seq_t = Tensor::from_slice(&flat)
            .reshape(&[t, i_dim])
            .to_device(self.device); // [T, NB_FEATURES]

        let h1 = self.forward_couche(&self.l1, &seq_t); // [T, 128]
        let h2 = self.forward_couche(&self.l2, &h1); // [T, 64]
        let h3 = self.forward_couche(&self.l3, &h2); // [T, 32]

        // Dernier état caché → couche de sortie
        let h3_last = h3.select(0, t - 1).unsqueeze(0); // [1, 32]
        let logits = h3_last.mm(&self.w_out.transpose(0, 1)) + &self.b_out; // [1, 2]
        let proba = logits.softmax(-1, Kind::Float); // [1, 2]
        proba.double_value(&[0, 1]) // P(Long)
    }
}
