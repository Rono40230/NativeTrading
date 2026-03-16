pub mod tendance;
mod sma_ema;
mod rsi;
mod atr;
mod macd;
mod bollinger;

pub use sma_ema::{calculer_sma, calculer_ema};
pub use rsi::calculer_rsi;
pub use atr::calculer_atr;
pub use macd::{Macd, calculer_macd};
pub use bollinger::{Bollinger, calculer_bollinger, calculer_bollinger_avance};

#[cfg(test)]
mod tests {
    use super::*;
    use common::Candle;
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
}
