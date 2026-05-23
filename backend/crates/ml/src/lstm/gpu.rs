//! Inférence LSTM accélérée GPU via tch::nn::LSTM (cuDNN natif).
//!
//! Ce module ne compile qu'avec `--features cuda`. Il charge le modèle
//! depuis le fichier `.pt` sauvegardé par `entrainement_gpu` et exécute
//! la passe avant via cuDNN — aucune boucle cellule par cellule.
//!
//! Usage :
//! ```ignore
//! let gpu = LstmGpu::depuis_pt("data/modele_lstm.pt");
//! if let Some(g) = gpu {
//!     let p_long = g.predire(&sequence);
//! }
//! ```

use tch::{nn, nn::Module as _, nn::RNN as _, Device, Kind, Tensor};

use super::{L1, L2, L3};
use crate::features::NB_FEATURES;

// ─── Accélérateur GPU ────────────────────────────────────────────────────────

/// LSTM 3 couches (128→64→32) inférence cuDNN sur GPU CUDA.
///
/// Non-sérialisable — chargé depuis le fichier `.pt` après chaque entraînement,
/// via [`LstmGpu::depuis_pt`].
pub struct LstmGpu {
    l1: nn::LSTM,
    l2: nn::LSTM,
    l3: nn::LSTM,
    fc: nn::Linear,
    device: Device,
}

// SAFETY: LstmGpu est protégé par Arc<RwLock<PipelineML>> — jamais accédé
// concurrentiellement. tch::Tensor utilise des raw ptr CUDA thread-local safe.
unsafe impl Send for LstmGpu {}
unsafe impl Sync for LstmGpu {}

impl LstmGpu {
    /// Charge le modèle depuis le fichier `.pt` (format PyTorch tch VarStore).
    ///
    /// Retourne `None` si CUDA indisponible ou fichier absent / corrompu.
    pub fn depuis_pt(chemin: &str) -> Option<Self> {
        if !tch::Cuda::is_available() {
            tracing::warn!("GPU LSTM: CUDA non disponible — inférence CPU conservée");
            return None;
        }
        if !std::path::Path::new(chemin).exists() {
            tracing::debug!(
                "GPU LSTM: fichier .pt absent ('{}') — premier démarrage ?",
                chemin
            );
            return None;
        }
        match Self::charger(chemin) {
            Ok(g) => {
                tracing::info!("LSTM GPU: modèle chargé depuis '{}'", chemin);
                Some(g)
            }
            Err(e) => {
                tracing::warn!(
                    "GPU LSTM: échec chargement '{}': {} — fallback CPU",
                    chemin,
                    e
                );
                None
            }
        }
    }

    fn charger(chemin: &str) -> anyhow::Result<Self> {
        let dev = Device::Cuda(0);
        let vs = nn::VarStore::new(dev);
        let root = vs.root();
        // Même architecture et configs que l'entraînement (entrainement_gpu.rs)
        let cfg_h = nn::RNNConfig {
            dropout: 0.0,
            ..Default::default()
        };
        let l1 = nn::lstm(&root / "l1", NB_FEATURES as i64, L1 as i64, cfg_h);
        let l2 = nn::lstm(&root / "l2", L1 as i64, L2 as i64, cfg_h);
        let l3 = nn::lstm(&root / "l3", L2 as i64, L3 as i64, nn::RNNConfig::default());
        let fc = nn::linear(&root / "fc", L3 as i64, 2, Default::default());

        // Contournement : vs.load() utilise at_load_multi_with_device →
        // torch::jit::_load_parameters (format TorchScript), incompatible avec
        // vs.save() qui écrit en format pickle dict via at_save_multi.
        // Solution : Tensor::load_multi (torch::pickle_load, symétrique) +
        // copie manuelle via shallow_clone (même mémoire sous-jacente que les couches).
        let tenseurs = Tensor::load_multi(chemin)
            .map_err(|e| anyhow::anyhow!("Chargement tenseurs LSTM depuis '{}': {}", chemin, e))?;
        // shallow_clone → même stockage que l1/l2/l3/fc ; mut requis par copy_(&mut self)
        let mut vars = vs.variables();
        tch::no_grad(|| {
            for (nom, src) in &tenseurs {
                if let Some(dst) = vars.get_mut(nom) {
                    dst.copy_(&src.to_device(dev));
                } else {
                    tracing::debug!(
                        "GPU LSTM chargement: clé '{}' absente dans l'architecture",
                        nom
                    );
                }
            }
        });

        // vs dropped ici — l1/l2/l3/fc conservent leurs tenseurs via Arc
        Ok(LstmGpu {
            l1,
            l2,
            l3,
            fc,
            device: dev,
        })
    }

    /// Inférence GPU via cuDNN : P(Long) ∈ [0,1].
    ///
    /// Retourne `None` si la séquence est vide ou en cas d'erreur GPU
    /// (le pipeline bascule alors automatiquement sur CPU).
    pub fn predire(&self, seq: &[Vec<f64>]) -> Option<f64> {
        if seq.is_empty() || seq[0].is_empty() {
            return None;
        }
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
        let f = seq[0].len() as i64;
        let flat: Vec<f32> = seq
            .iter()
            .flat_map(|r| r.iter().map(|&x| x as f32))
            .collect();
        // [1, T, F] — batch=1, batch_first (même convention que l'entraînement)
        let x = Tensor::from_slice(&flat)
            .reshape([1, t, f])
            .to_device(self.device);

        let (h1, _) = self.l1.seq(&x); // [1, T, L1]
        let (h2, _) = self.l2.seq(&h1); // [1, T, L2]
        let (h3, _) = self.l3.seq(&h2); // [1, T, L3]

        let last = h3.select(1, t - 1); // [1, L3]
        let logits = self.fc.forward(&last); // [1, 2]
        logits.softmax(-1, Kind::Float).double_value(&[0, 1]) // P(Long)
    }
}
