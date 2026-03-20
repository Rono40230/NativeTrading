use crate::sma_ema::{calculer_ema, calculer_sma};
use common::Candle;

/// Resultat Bandes de Bollinger
#[derive(Debug, Clone)]
pub struct Bollinger {
    pub superieure: Vec<f64>,
    pub milieu: Vec<f64>,
    pub inferieure: Vec<f64>,
}

/// Bandes de Bollinger (base SMA fixe)
pub fn calculer_bollinger(bougies: &[Candle], periode: usize, nb_ecarts: f64) -> Bollinger {
    let n = bougies.len();
    let mut superieure = vec![f64::NAN; n];
    let mut milieu = vec![f64::NAN; n];
    let mut inferieure = vec![f64::NAN; n];

    for i in periode..=n {
        let fenetre: Vec<f64> = bougies[i - periode..i].iter().map(|b| b.close).collect();
        let sma = fenetre.iter().sum::<f64>() / periode as f64;
        let variance = fenetre.iter().map(|&c| (c - sma).powi(2)).sum::<f64>() / periode as f64;
        let ecart = variance.sqrt();
        milieu[i - 1] = sma;
        superieure[i - 1] = sma + nb_ecarts * ecart;
        inferieure[i - 1] = sma - nb_ecarts * ecart;
    }
    Bollinger {
        superieure,
        milieu,
        inferieure,
    }
}

/// Bandes de Bollinger avancees — `ma_type` accepte "ema" ou toute autre valeur = SMA.
pub fn calculer_bollinger_avance(
    bougies: &[Candle],
    periode: usize,
    nb_ecarts: f64,
    ma_type: &str,
) -> Bollinger {
    let n = bougies.len();
    if n < periode || periode == 0 {
        return Bollinger {
            superieure: vec![f64::NAN; n],
            milieu: vec![f64::NAN; n],
            inferieure: vec![f64::NAN; n],
        };
    }
    let base: Vec<f64> = if ma_type == "ema" {
        calculer_ema(bougies, periode)
    } else {
        calculer_sma(bougies, periode)
    };
    let mut superieure = vec![f64::NAN; n];
    let mut milieu = vec![f64::NAN; n];
    let mut inferieure = vec![f64::NAN; n];

    for i in periode..=n {
        let mean = base[i - 1];
        if !mean.is_finite() {
            continue;
        }
        let fenetre: Vec<f64> = bougies[i - periode..i].iter().map(|b| b.close).collect();
        let variance = fenetre.iter().map(|&c| (c - mean).powi(2)).sum::<f64>() / periode as f64;
        let ecart = variance.sqrt();
        milieu[i - 1] = mean;
        superieure[i - 1] = mean + nb_ecarts * ecart;
        inferieure[i - 1] = mean - nb_ecarts * ecart;
    }
    Bollinger {
        superieure,
        milieu,
        inferieure,
    }
}
