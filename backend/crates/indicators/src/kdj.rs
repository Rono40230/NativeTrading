use common::Candle;

/// KDJ — variante « RSV + double lissage » de l'étalon KDJ/Halftrend
/// (docs/reference/definition_kdj_halftrend.md §2.1).
/// k et d ∈ [0, 100] ; j = 3k − 2d peut sortir de cet intervalle.
/// NaN pendant la chauffe (moins de `period` bougies).
#[derive(Debug, Clone)]
pub struct Kdj {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
    pub j: Vec<f64>,
}

/// RSV = 100 × (close − lowest(low, period)) / (highest(high, period) − lowest(low, period)),
/// puis lissage type EMA de période `signal`, initialisé à 0 (`nz()` de l'étalon).
pub fn calculer_kdj(bougies: &[Candle], period: usize, signal: usize) -> Kdj {
    let n = bougies.len();
    let mut k = vec![f64::NAN; n];
    let mut d = vec![f64::NAN; n];
    if period == 0 || signal == 0 || n < period {
        return Kdj { k, d, j: vec![f64::NAN; n] };
    }

    // m = 1 dans l'étalon (poids de la valeur fraîche).
    let m = 1.0_f64;
    let sig = signal as f64;
    for i in (period - 1)..n {
        let fen = &bougies[i + 1 - period..=i];
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::INFINITY;
        for b in fen {
            hh = hh.max(b.high);
            ll = ll.min(b.low);
        }
        let rsv = if hh > ll {
            100.0 * (bougies[i].close - ll) / (hh - ll)
        } else {
            f64::NAN
        };
        // nz(x[1]) : NaN → 0 (première valeur ou division dégénérée précédente).
        let k_prec = if k[i - 1].is_nan() { 0.0 } else { k[i - 1] };
        let d_prec = if d[i - 1].is_nan() { 0.0 } else { d[i - 1] };
        k[i] = if rsv.is_nan() {
            f64::NAN
        } else {
            (m * rsv + (sig - m) * k_prec) / sig
        };
        d[i] = if k[i].is_nan() {
            f64::NAN
        } else {
            (m * k[i] + (sig - m) * d_prec) / sig
        };
    }

    let j = (0..n)
        .map(|i| {
            if k[i].is_nan() || d[i].is_nan() {
                f64::NAN
            } else {
                3.0 * k[i] - 2.0 * d[i]
            }
        })
        .collect();
    Kdj { k, d, j }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie(prix: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: prix,
            high: prix + 0.5,
            low: prix - 0.5,
            close: prix,
            volume: 1000.0,
        }
    }

    fn bougies(fermetures: &[f64]) -> Vec<Candle> {
        fermetures.iter().map(|&p| bougie(p)).collect()
    }

    #[test]
    fn tailles_alignees() {
        let b = bougies(&(60..120).map(|i| i as f64).collect::<Vec<_>>());
        let kdj = calculer_kdj(&b, 20, 7);
        assert_eq!(kdj.k.len(), b.len());
        assert_eq!(kdj.d.len(), b.len());
        assert_eq!(kdj.j.len(), b.len());
    }

    #[test]
    fn chauffe_nan_puis_valeurs() {
        let b = bougies(&(0..40).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
        let kdj = calculer_kdj(&b, 20, 7);
        for i in 0..19 {
            assert!(kdj.k[i].is_nan(), "k[{}] doit être NaN", i);
        }
        assert!(kdj.k[19].is_finite());
        // Fenêtre 100..=119 avec mèches ±0.5 : RSV = (119−99.5)/20 = 97.5.
        // Première valeur : RSV/7 (précédent nz → 0), d = k/7.
        assert!((kdj.k[19] - 97.5 / 7.0).abs() < 1e-9, "{} vs {}", kdj.k[19], 97.5 / 7.0);
        assert!((kdj.d[19] - 97.5 / 49.0).abs() < 1e-9);
    }

    #[test]
    fn montee_franches_rsv_cent() {
        // Montée strictement régulière : RSV = 100 à chaque barre,
        // k converge vers 100 depuis k[19] = 100/7.
        let b = bougies(&(0..60).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
        let kdj = calculer_kdj(&b, 20, 7);
        let valides: Vec<f64> = kdj.k.iter().copied().filter(|v| v.is_finite()).collect();
        for w in valides.windows(2) {
            assert!(w[1] > w[0], "k doit croître vers 100");
        }
        assert!(kdj.k[59] > 90.0 && kdj.k[59] < 100.0);
        // j = 3k − 2d et k > d en montée → j > k.
        assert!(kdj.j[59] > kdj.k[59]);
    }

    #[test]
    fn plage_parfaite_rsv_nan() {
        // high == low sur toute la fenêtre : RSV = 0/0 → NaN (Pine : na).
        let mut b = bougies(&vec![100.0; 30]);
        for c in &mut b {
            c.high = 100.0;
            c.low = 100.0;
            c.close = 100.0;
        }
        let kdj = calculer_kdj(&b, 20, 7);
        assert!(kdj.k.iter().all(|v| v.is_nan()));
    }
}
