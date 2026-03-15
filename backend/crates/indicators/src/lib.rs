use common::Candle;

pub mod tendance;

/// Résultat MACD
#[derive(Debug, Clone)]
pub struct Macd {
    pub ligne: Vec<f64>,  // MACD line (EMA rapide - EMA lente)
    pub signal: Vec<f64>, // Signal line (EMA du MACD)
    pub histogramme: Vec<f64>,
}

/// Résultat Bandes de Bollinger
#[derive(Debug, Clone)]
pub struct Bollinger {
    pub superieure: Vec<f64>,
    pub milieu: Vec<f64>, // SMA
    pub inferieure: Vec<f64>,
}

// ─── SMA ─────────────────────────────────────────────────────────────────────

/// Moyenne mobile simple (SMA) sur les prix de clôture
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

// ─── EMA ─────────────────────────────────────────────────────────────────────

/// Moyenne mobile exponentielle (EMA) sur les prix de clôture
pub fn calculer_ema(bougies: &[Candle], periode: usize) -> Vec<f64> {
    if bougies.len() < periode || periode == 0 {
        return vec![f64::NAN; bougies.len()];
    }
    let k = 2.0 / (periode as f64 + 1.0);
    let closes: Vec<f64> = bougies.iter().map(|b| b.close).collect();
    let mut ema = vec![f64::NAN; closes.len()];

    // Seed : SMA sur les `periode` premières valeurs
    let seed: f64 = closes[..periode].iter().sum::<f64>() / periode as f64;
    ema[periode - 1] = seed;

    for i in periode..closes.len() {
        ema[i] = closes[i] * k + ema[i - 1] * (1.0 - k);
    }
    ema
}

// ─── RSI ─────────────────────────────────────────────────────────────────────

/// Relative Strength Index (Wilder, pédiode standard = 14)
pub fn calculer_rsi(bougies: &[Candle], periode: usize) -> Vec<f64> {
    if bougies.len() <= periode || periode == 0 {
        return vec![f64::NAN; bougies.len()];
    }
    let closes: Vec<f64> = bougies.iter().map(|b| b.close).collect();
    let n = closes.len();
    let mut rsi = vec![f64::NAN; n];

    // Calcul gains/pertes
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

    // Moyennes initiales (SMA sur `periode`)
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

    // Lissage de Wilder
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

// ─── ATR ─────────────────────────────────────────────────────────────────────

/// Average True Range (Wilder, période standard = 14)
pub fn calculer_atr(bougies: &[Candle], periode: usize) -> Vec<f64> {
    if bougies.len() <= periode || periode == 0 {
        return vec![f64::NAN; bougies.len()];
    }
    let n = bougies.len();
    let mut atr = vec![f64::NAN; n];

    // True Range pour chaque bougie (sauf la première)
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

// ─── MACD ────────────────────────────────────────────────────────────────────

/// MACD standard (12, 26, 9)
pub fn calculer_macd(bougies: &[Candle], rapide: usize, lent: usize, signal_p: usize) -> Macd {
    let ema_rapide = calculer_ema(bougies, rapide);
    let ema_lente = calculer_ema(bougies, lent);
    let n = bougies.len();

    let ligne: Vec<f64> = (0..n)
        .map(|i| {
            if ema_rapide[i].is_nan() || ema_lente[i].is_nan() {
                f64::NAN
            } else {
                ema_rapide[i] - ema_lente[i]
            }
        })
        .collect();

    // EMA du MACD (signal) — on crée des pseudo-bougies avec close = macd
    let pseudo: Vec<Candle> = bougies
        .iter()
        .enumerate()
        .map(|(i, b)| Candle {
            close: if ligne[i].is_nan() { 0.0 } else { ligne[i] },
            ..*b
        })
        .collect();
    let signal_line = calculer_ema(&pseudo, signal_p);

    let histogramme: Vec<f64> = (0..n)
        .map(|i| {
            if ligne[i].is_nan() || signal_line[i].is_nan() {
                f64::NAN
            } else {
                ligne[i] - signal_line[i]
            }
        })
        .collect();

    Macd {
        ligne,
        signal: signal_line,
        histogramme,
    }
}

// ─── BOLLINGER ────────────────────────────────────────────────────────────────

/// Bandes de Bollinger (période = 20, écart-type = 2.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie(close: f64, high: f64, low: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: close,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    fn bougies_simples(n: usize) -> Vec<Candle> {
        (1..=n)
            .map(|i| {
                bougie(
                    i as f64 * 100.0,
                    i as f64 * 100.0 + 5.0,
                    i as f64 * 100.0 - 5.0,
                )
            })
            .collect()
    }

    #[test]
    fn atr_taille_correcte() {
        let b = bougies_simples(20);
        let atr = calculer_atr(&b, 14);
        assert_eq!(atr.len(), 20);
    }

    #[test]
    fn atr_valeurs_positives() {
        let b = bougies_simples(20);
        let atr = calculer_atr(&b, 14);
        // Les dernières valeurs doivent être > 0
        let valides: Vec<f64> = atr.iter().copied().filter(|v| !v.is_nan()).collect();
        assert!(!valides.is_empty());
        assert!(valides.iter().all(|&v| v > 0.0));
    }

    #[test]
    fn rsi_dans_intervalle() {
        let b = bougies_simples(30);
        let rsi = calculer_rsi(&b, 14);
        let valides: Vec<f64> = rsi.iter().copied().filter(|v| !v.is_nan()).collect();
        assert!(!valides.is_empty());
        assert!(valides.iter().all(|&v| (0.0..=100.0).contains(&v)));
    }

    #[test]
    fn ema_moins_nan_que_bougies() {
        let b = bougies_simples(30);
        let ema = calculer_ema(&b, 9);
        let valides = ema.iter().filter(|v| !v.is_nan()).count();
        assert_eq!(valides, 30 - 9 + 1); // ema[periode-1..] sont valides
    }
}
