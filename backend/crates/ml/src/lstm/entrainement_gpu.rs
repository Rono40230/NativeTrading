//! Entraînement LSTM GPU via tch::nn (cuDNN LSTM accéléré sur CUDA).
//!
//! Remplace le BPTT CPU manuel par un entraînement natif cuDNN.
//! Gain attendu : ×50 à ×100 sur RTX 3090 vs CPU pur Rust.
//! Compilé uniquement avec `--features cuda`.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use tch::{
    nn::{self, Module as _, OptimizerConfig as _, RNN as _},
    Device, Kind, Tensor,
};

use super::{ModeleHybrideLstm, PoidsCouches, L1, L2, L3};

const BATCH: i64 = 128;

/// Entraîne le LSTM 3 couches (128→64→32) sur GPU CUDA.
///
/// Retourne l'accuracy OOS (validation 20%). Les poids sont
/// automatiquement rapatriés dans `modele` CPU après entraînement.
pub(crate) fn entrainer_sur_gpu(
    modele: &mut ModeleHybrideLstm,
    sequences: &[Vec<Vec<f64>>],
    labels: &[f64],
    epochs: usize,
    lr: f64,
) -> Result<f64> {
    if sequences.is_empty() {
        return Ok(0.0);
    }

    let dev = Device::Cuda(0);
    let vs = nn::VarStore::new(dev);
    let root = vs.root();

    let n_feat = sequences[0][0].len() as i64;
    // RNNConfig : batch_first=true par défaut dans tch 0.16
    let cfg_h = nn::RNNConfig { dropout: 0.1, ..Default::default() };
    let cfg_o = nn::RNNConfig::default();
    let l1 = nn::lstm(&root / "l1", n_feat, L1 as i64, cfg_h.clone());
    let l2 = nn::lstm(&root / "l2", L1 as i64, L2 as i64, cfg_h);
    let l3 = nn::lstm(&root / "l3", L2 as i64, L3 as i64, cfg_o);
    let fc = nn::linear(&root / "fc", L3 as i64, 2, Default::default());

    let mut opt = nn::Adam::default().build(&vs, lr)?;

    let n = sequences.len();
    let split = (n * 80 / 100).max(1);
    let x_all = seqs_vers_tenseur(sequences, dev);   // [N, T, F]
    let y_all = labels_vers_tenseur(labels, dev);     // [N]
    let n_train = split as i64;
    let n_val = (n - split) as i64;
    let x_train = x_all.narrow(0, 0, n_train);
    let y_train = y_all.narrow(0, 0, n_train);
    let x_val = x_all.narrow(0, n_train, n_val.max(1));
    let y_val = y_all.narrow(0, n_train, n_val.max(1));

    let mut best_val_acc = 0.0f64;
    let mut patience = 0usize;

    for epoch in 0..epochs {
        let batches = (n_train + BATCH - 1) / BATCH;
        for b in 0..batches {
            let start = b * BATCH;
            let len = BATCH.min(n_train - start);
            let xb = x_train.narrow(0, start, len);
            let yb = y_train.narrow(0, start, len);
            let logits = forward_net(&l1, &l2, &l3, &fc, &xb);
            let loss = logits.cross_entropy_for_logits(&yb);
            opt.backward_step(&loss);
        }

        let val_acc = tch::no_grad(|| {
            let logits = forward_net(&l1, &l2, &l3, &fc, &x_val);
            let preds = logits.argmax(1, false);
            let correct = preds.eq_tensor(&y_val).to_kind(Kind::Float).mean(Kind::Float);
            f64::try_from(correct).unwrap_or(0.0)
        });

        tracing::debug!(
            "LSTM GPU epoch {}/{}: val_acc={:.1}%",
            epoch + 1, epochs, val_acc * 100.0
        );

        if val_acc > best_val_acc {
            best_val_acc = val_acc;
            patience = 0;
        } else {
            patience += 1;
            if patience >= 3 {
                tracing::info!(
                    "LSTM GPU early stopping epoch {}: best_val={:.1}%",
                    epoch + 1, best_val_acc * 100.0
                );
                break;
            }
        }
    }

    let vars = vs.variables();
    let poids = extraire_par_couches(&vars, n_feat as usize)?;
    modele.appliquer_poids_depuis_gpu(poids);
    Ok(best_val_acc)
}

fn forward_net(
    l1: &nn::LSTM,
    l2: &nn::LSTM,
    l3: &nn::LSTM,
    fc: &nn::Linear,
    x: &Tensor,
) -> Tensor {
    let (h1, _) = l1.seq(x);
    let (h2, _) = l2.seq(&h1);
    let (h3, _) = l3.seq(&h2);
    let t = h3.size()[1];
    let last = h3.select(1, t - 1); // [B, L3]
    fc.forward(&last)               // [B, 2]
}

fn seqs_vers_tenseur(seqs: &[Vec<Vec<f64>>], dev: Device) -> Tensor {
    let n = seqs.len() as i64;
    let t = seqs[0].len() as i64;
    let f = seqs[0][0].len() as i64;
    let flat: Vec<f32> = seqs
        .iter()
        .flat_map(|s| s.iter().flat_map(|r| r.iter().map(|&x| x as f32)))
        .collect();
    Tensor::from_slice(&flat).reshape(&[n, t, f]).to_device(dev)
}

fn labels_vers_tenseur(lbls: &[f64], dev: Device) -> Tensor {
    let v: Vec<i64> = lbls.iter().map(|&l| if l >= 0.5 { 1 } else { 0 }).collect();
    Tensor::from_slice(&v).to_device(dev)
}

fn extraire_par_couches(vars: &HashMap<String, Tensor>, n_feat: usize) -> Result<PoidsCouches> {
    let c1 = fusionner_couche(vars, "l1")?;
    let c2 = fusionner_couche(vars, "l2")?;
    let c3 = fusionner_couche(vars, "l3")?;

    let fc_w = get_var(vars, "fc.weight")?; // [2, L3]
    let fc_b = get_var(vars, "fc.bias")?;   // [2]
    let sortie_poids: Vec<Vec<f64>> = (0..2)
        .map(|r| (0..L3).map(|c| fc_w.double_value(&[r as i64, c as i64])).collect())
        .collect();
    let sortie_biais: Vec<f64> = (0..2).map(|i| fc_b.double_value(&[i as i64])).collect();

    Ok(PoidsCouches {
        l1_poids: c1.0, l1_biais: c1.1,
        l1_in: n_feat, l1_h: L1,
        l2_poids: c2.0, l2_biais: c2.1,
        l2_in: L1, l2_h: L2,
        l3_poids: c3.0, l3_biais: c3.1,
        l3_in: L2, l3_h: L3,
        sortie_poids, sortie_biais,
    })
}

/// Fusionne weight_ih [4H,I] + weight_hh [4H,H] → poids [4H, I+H]
/// et (bias_ih + bias_hh) → biais [4H].
fn fusionner_couche(vars: &HashMap<String, Tensor>, pfx: &str) -> Result<(Vec<f64>, Vec<f64>)> {
    let wih = get_var(vars, &format!("{pfx}.weight_ih_l0"))?;
    let whh = get_var(vars, &format!("{pfx}.weight_hh_l0"))?;
    let bih = get_var(vars, &format!("{pfx}.bias_ih_l0"))?;
    let bhh = get_var(vars, &format!("{pfx}.bias_hh_l0"))?;

    let w = Tensor::cat(&[wih, whh], 1).contiguous().flatten(0, -1);
    let b = (bih + bhh).contiguous().flatten(0, -1);
    let poids: Vec<f64> = (0..w.numel() as i64).map(|i| w.double_value(&[i])).collect();
    let biais: Vec<f64> = (0..b.numel() as i64).map(|i| b.double_value(&[i])).collect();
    Ok((poids, biais))
}

fn get_var(vars: &HashMap<String, Tensor>, name: &str) -> Result<Tensor> {
    // Recherche exacte puis recherche par suffixe (robustesse nommage tch)
    if let Some(t) = vars.get(name) {
        return Ok(t.to_device(Device::Cpu).to_kind(Kind::Double));
    }
    for (k, t) in vars {
        if k.ends_with(name) {
            return Ok(t.to_device(Device::Cpu).to_kind(Kind::Double));
        }
    }
    let keys: Vec<&String> = vars.keys().collect();
    Err(anyhow!("Variable GPU '{}' introuvable. Disponibles: {:?}", name, keys))
}
