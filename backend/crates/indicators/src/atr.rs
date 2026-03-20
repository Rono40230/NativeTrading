use common::Candle;

/// Average True Range (Wilder, periode standard = 14)
pub fn calculer_atr(bougies: &[Candle], periode: usize) -> Vec<f64> {
    if bougies.len() <= periode || periode == 0 {
        return vec![f64::NAN; bougies.len()];
    }
    let n = bougies.len();
    let mut atr = vec![f64::NAN; n];

    let true_ranges: Vec<f64> = (1..n)
        .map(|i| {
            let hl = bougies[i].high - bougies[i].low;
            let hc = (bougies[i].high - bougies[i - 1].close).abs();
            let lc = (bougies[i].low - bougies[i - 1].close).abs();
            hl.max(hc).max(lc)
        })
        .collect();

    // SMA initiale
    let sma_init: f64 = true_ranges[..periode].iter().sum::<f64>() / periode as f64;
    atr[periode] = sma_init;

    // Lissage Wilder
    for i in (periode + 1)..n {
        atr[i] = (atr[i - 1] * (periode as f64 - 1.0) + true_ranges[i - 1]) / periode as f64;
    }
    atr
}
