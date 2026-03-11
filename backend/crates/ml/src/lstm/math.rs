// Fonctions mathématiques utilitaires et états intermédiaires BPTT

pub(super) fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

pub(super) fn d_sigmoid(s: f64) -> f64 {
    s * (1.0 - s)
}

pub(super) fn d_tanh(t: f64) -> f64 {
    1.0 - t * t
}

pub(super) fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&v| (v - max).exp()).collect();
    let sum: f64 = exps.iter().sum::<f64>().max(1e-10);
    exps.iter().map(|&e| e / sum).collect()
}

/// Générateur LCG (pas de dépendance externe rand)
pub(super) fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 11) as f64 / (1u64 << 53) as f64
}

pub(super) fn xavier(n: usize, m: usize, g: &mut u64) -> Vec<f64> {
    let scale = (6.0 / (n + m) as f64).sqrt();
    (0..n * m)
        .map(|_| (lcg_next(g) * 2.0 - 1.0) * scale)
        .collect()
}

/// États intermédiaires stockés pour le BPTT
pub(super) struct EtatLstm {
    pub(super) x: Vec<f64>,      // entrée (features ou h précédent)
    pub(super) h_prev: Vec<f64>, // état caché précédent
    pub(super) c_prev: Vec<f64>, // état cellulaire précédent
    pub(super) i: Vec<f64>,      // porte d'entrée (post-sigmoid)
    pub(super) f: Vec<f64>,      // porte d'oubli (post-sigmoid)
    pub(super) g: Vec<f64>,      // porte de cell (post-tanh)
    pub(super) o: Vec<f64>,      // porte de sortie (post-sigmoid)
    pub(super) c: Vec<f64>,      // état cellulaire (post-update)
    pub(super) h: Vec<f64>,      // état caché (post-update)
}
