use common::Candle;

/// Moyenne mobile simple (SMA) sur les prix de cloture
pub fn calculer_sma(bougies: &[Candle], periode: usize) -> Vec<f64> {
    let n = bougies.len();
    if n < periode || periode == 0 {
        return vec![f64::NAN; n];
    }
    let closes: Vec<f64> = bougies.iter().map(|b| b.close).collect();
    let mut sma = vec![f64::NAN; n];
    for i in (periode - 1)..n {
        sma[i] = closes[i + 1 - periode..=i].iter().sum::<f64>() / periode as f64;
    }
    sma
}

/// Moyenne mobile exponentielle (EMA) sur les prix de cloture
pub fn calculer_ema(bougies: &[Candle], periode: usize) -> Vec<f64> {
    if bougies.len() < periode || periode == 0 {
        return vec![f64::NAN; bougies.len()];
    }
    let k = 2.0 / (periode as f64 + 1.0);
    let closes: Vec<f64> = bougies.iter().map(|b| b.close).collect();
    let mut ema = vec![f64::NAN; closes.len()];

    // Seed : SMA sur les `periode` premieres valeurs
    let seed: f64 = closes[..periode].iter().sum::<f64>() / periode as f64;
    ema[periode - 1] = seed;

    for i in periode..closes.len() {
        ema[i] = closes[i] * k + ema[i - 1] * (1.0 - k);
    }
    ema
}
