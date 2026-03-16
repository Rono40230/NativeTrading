use common::Candle;
use crate::sma_ema::calculer_ema;

/// Resultat MACD
#[derive(Debug, Clone)]
pub struct Macd {
    pub ligne: Vec<f64>,
    pub signal: Vec<f64>,
    pub histogramme: Vec<f64>,
}

/// MACD standard (12, 26, 9)
pub fn calculer_macd(bougies: &[Candle], rapide: usize, lent: usize, signal_p: usize) -> Macd {
    let ema_rapide = calculer_ema(bougies, rapide);
    let ema_lente  = calculer_ema(bougies, lent);
    let n = bougies.len();

    let ligne: Vec<f64> = (0..n)
        .map(|i| {
            if ema_rapide[i].is_nan() || ema_lente[i].is_nan() { f64::NAN }
            else { ema_rapide[i] - ema_lente[i] }
        })
        .collect();

    // EMA du MACD (signal) — pseudo-bougies avec close = macd
    let pseudo: Vec<Candle> = bougies
        .iter()
        .enumerate()
        .map(|(i, b)| Candle { close: if ligne[i].is_nan() { 0.0 } else { ligne[i] }, ..*b })
        .collect();
    let signal_line = calculer_ema(&pseudo, signal_p);

    let histogramme: Vec<f64> = (0..n)
        .map(|i| {
            if ligne[i].is_nan() || signal_line[i].is_nan() { f64::NAN }
            else { ligne[i] - signal_line[i] }
        })
        .collect();

    Macd { ligne, signal: signal_line, histogramme }
}
