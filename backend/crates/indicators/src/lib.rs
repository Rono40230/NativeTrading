mod atr;
mod bollinger;
mod macd;
mod rsi;
mod sma_ema;

pub use atr::calculer_atr;
pub use bollinger::{calculer_bollinger, calculer_bollinger_avance, Bollinger};
pub use macd::{calculer_macd, Macd};
pub use rsi::calculer_rsi;
pub use sma_ema::{calculer_ema, calculer_sma};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

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
        assert_eq!(valides, 30 - 9 + 1);
    }

    #[test]
    fn macd_taille_correcte() {
        let b = bougies_simples(50);
        let m = calculer_macd(&b, 12, 26, 9);
        assert_eq!(m.ligne.len(), 50);
        assert_eq!(m.signal.len(), 50);
        assert_eq!(m.histogramme.len(), 50);
    }

    #[test]
    fn macd_valeurs_non_toutes_nan() {
        let b = bougies_simples(50);
        let m = calculer_macd(&b, 12, 26, 9);
        let ligne_valides = m.ligne.iter().filter(|v| !v.is_nan()).count();
        let histo_valides = m.histogramme.iter().filter(|v| !v.is_nan()).count();
        assert!(
            ligne_valides > 0,
            "ligne MACD doit avoir des valeurs non-NaN"
        );
        assert!(
            histo_valides > 0,
            "histogramme MACD doit avoir des valeurs non-NaN"
        );
    }

    #[test]
    fn bollinger_bandes_coherentes() {
        let b = bougies_simples(30);
        let bb = calculer_bollinger(&b, 20, 2.0);
        assert_eq!(bb.superieure.len(), 30);
        // Les valeurs valides (non-NaN) : supérieure >= milieu >= inférieure
        let valides: Vec<usize> = (0..30).filter(|&i| !bb.milieu[i].is_nan()).collect();
        assert!(!valides.is_empty());
        for i in valides {
            assert!(bb.superieure[i] >= bb.milieu[i], "sup >= milieu");
            assert!(bb.milieu[i] >= bb.inferieure[i], "milieu >= inf");
        }
    }

    #[test]
    fn bollinger_milieu_egal_sma() {
        // Prix constants → SMA = prix → bandes symétriques autour du prix
        let prix = 100.0;
        let b: Vec<Candle> = (0..25)
            .map(|_| bougie(prix, prix + 1.0, prix - 1.0))
            .collect();
        let bb = calculer_bollinger(&b, 20, 2.0);
        let milieu_valides: Vec<f64> = bb.milieu.iter().copied().filter(|v| !v.is_nan()).collect();
        assert!(!milieu_valides.is_empty());
        for m in &milieu_valides {
            assert!(
                (m - prix).abs() < 1e-9,
                "milieu doit égaler le prix constant"
            );
        }
    }
}
