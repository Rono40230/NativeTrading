use common::Candle;

/// Relative Strength Index (Wilder, periode standard = 14)
pub fn calculer_rsi(bougies: &[Candle], periode: usize) -> Vec<f64> {
    if bougies.len() <= periode || periode == 0 {
        return vec![f64::NAN; bougies.len()];
    }
    let closes: Vec<f64> = bougies.iter().map(|b| b.close).collect();
    let n = closes.len();
    let mut rsi = vec![f64::NAN; n];

    let gains_pertes: Vec<(f64, f64)> = (1..n)
        .map(|i| {
            let diff = closes[i] - closes[i - 1];
            if diff > 0.0 {
                (diff, 0.0)
            } else {
                (0.0, -diff)
            }
        })
        .collect();

    let (mut avg_gain, mut avg_perte) = {
        let gains: f64 =
            gains_pertes[..periode].iter().map(|(g, _)| g).sum::<f64>() / periode as f64;
        let pertes: f64 =
            gains_pertes[..periode].iter().map(|(_, p)| p).sum::<f64>() / periode as f64;
        (gains, pertes)
    };

    rsi[periode] = if avg_perte == 0.0 {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + avg_gain / avg_perte)
    };

    for i in (periode + 1)..n {
        let (g, p) = gains_pertes[i - 1];
        avg_gain = (avg_gain * (periode as f64 - 1.0) + g) / periode as f64;
        avg_perte = (avg_perte * (periode as f64 - 1.0) + p) / periode as f64;
        rsi[i] = if avg_perte == 0.0 {
            100.0
        } else {
            100.0 - 100.0 / (1.0 + avg_gain / avg_perte)
        };
    }
    rsi
}
