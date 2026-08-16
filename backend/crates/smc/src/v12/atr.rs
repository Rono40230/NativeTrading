//! ATR14 (Wilder) — équivalent `ta.atr(14)` du Pine (ligne 421).
//!
//! `ta.atr(period)` dans Pine utilise le lissage de Wilder :
//!   - seed  = moyenne simple des `period` premiers True Range,
//!   - puis  ATR = (ATR_prev × (period-1) + TR) / period.
//!
//! True Range = max(high-low, |high-prev_close|, |low-prev_close|).
//! Le premier TR nécessite une close précédente → non défini sur la 1ʳᵉ bar.

use super::types::BarInput;

/// ATR14 avec lissage Wilder.
#[derive(Clone)]
pub struct Atr14 {
    period: usize,
    prev_close: Option<f64>,
    /// Historique des TR — utilisé uniquement pour le seed, puis libéré.
    trs: Vec<f64>,
    atr: f64,
    initialized: bool,
}

impl Atr14 {
    pub fn new() -> Self {
        Self {
            period: 14,
            prev_close: None,
            trs: Vec::with_capacity(14),
            atr: 0.0,
            initialized: false,
        }
    }

    pub fn update(&mut self, bar: &BarInput) {
        // Pas de TR sur la 1ʳᵉ bar (pas de close précédente).
        let pc = match self.prev_close {
            None => {
                self.prev_close = Some(bar.close);
                return;
            }
            Some(pc) => pc,
        };
        let tr = (bar.high - bar.low)
            .max((bar.high - pc).abs())
            .max((bar.low - pc).abs());
        self.prev_close = Some(bar.close);

        if !self.initialized {
            self.trs.push(tr);
            if self.trs.len() >= self.period {
                let sum: f64 = self.trs.iter().take(self.period).sum();
                self.atr = sum / self.period as f64;
                self.initialized = true;
                self.trs.clear(); // plus besoin de l'historique après le seed
            }
        } else {
            // Lissage Wilder.
            let p = self.period as f64;
            self.atr = (self.atr * (p - 1.0) + tr) / p;
        }
    }

    /// Valeur ATR courante (0.0 tant que le seed n'est pas calculé, i.e. < period+1 bars).
    pub fn value(&self) -> f64 {
        self.atr
    }

    /// Vrai dès que l'ATR est utilisable (≥ period+1 bars reçues).
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

impl Default for Atr14 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: i64, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: i,
            open: close,
            high,
            low,
            close,
            volume: 100.0,
        }
    }

    #[test]
    fn atr14_se_calcule_apres_14_bars() {
        let mut atr = Atr14::new();
        for i in 0..20 {
            atr.update(&bar(i, 101.0 + i as f64 * 0.1, 99.0, 100.5));
        }
        assert!(atr.value() > 0.0, "ATR doit être > 0 après 14+ bars");
        assert!(atr.value() < 5.0, "ATR doit rester raisonnable");
        assert!(atr.is_ready());
    }

    #[test]
    fn atr14_zero_avant_seed() {
        let mut atr = Atr14::new();
        for i in 0..10 {
            atr.update(&bar(i, 101.0, 99.0, 100.5));
        }
        assert!(!atr.is_ready());
        assert_eq!(atr.value(), 0.0, "ATR = 0 tant que le seed n'est pas calculé");
    }

    #[test]
    fn atr14_constant_sur_bars_identiques() {
        // Bars identiques (high-low = 2, gap close nul) → TR = 2 partout → ATR → 2.
        let mut atr = Atr14::new();
        for i in 0..40 {
            atr.update(&bar(i, 102.0, 100.0, 101.0));
        }
        let v = atr.value();
        assert!(
            (v - 2.0).abs() < 1e-9,
            "ATR doit converger vers 2.0 sur bars constants, got {v}"
        );
    }
}
