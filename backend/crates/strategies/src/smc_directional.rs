use super::{Signal, Strategy};
use common::{Candle, Direction, Result};
use indicators::calculer_atr;
use smc::scorer;

/// Multiplicateurs ATR pour TP / SL
const ATR_TP1: f64 = 1.5;
const ATR_TP2: f64 = 3.0;
const ATR_TP3: f64 = 5.0;
const ATR_SL: f64 = 1.0;

/// Stratégie SMC Directionnelle — scoring confluence ≥70/100.
///
/// Déclencheur : Score SMC ≥ 70 (Tendance + OB + Imbalance + IFVG + Fibonacci).
/// Entrée : close actuel | SL : ATR×1 | TP1 : ATR×1.5 | TP2 : ATR×3
/// Risk : 1.5% par trade
pub struct SmcDirectionalStrategy;

impl Strategy for SmcDirectionalStrategy {
    fn analyze(&self, bougies: &[Candle]) -> Result<Option<Signal>> {
        if bougies.len() < 30 {
            return Ok(None);
        }

        let score = match scorer(bougies) {
            Some(s) if s.confluence => s,
            _ => return Ok(None),
        };

        let atr = calculer_atr(bougies, 14);
        let n = bougies.len();
        let atr_val = atr[n - 1];
        if atr_val.is_nan() || atr_val <= 0.0 {
            return Ok(None);
        }

        let entree = bougies[n - 1].close;
        let confiance = (score.total / 100.0).min(1.0);

        let (stop_loss, take_profit, take_profit_2, take_profit_3) = match score.direction {
            Direction::Long => (
                entree - atr_val * ATR_SL,
                entree + atr_val * ATR_TP1,
                Some(entree + atr_val * ATR_TP2),
                Some(entree + atr_val * ATR_TP3),
            ),
            Direction::Short => (
                entree + atr_val * ATR_SL,
                entree - atr_val * ATR_TP1,
                Some(entree - atr_val * ATR_TP2),
                Some(entree - atr_val * ATR_TP3),
            ),
            Direction::Both => return Ok(None),
        };

        tracing::info!(
            "SMC signal {:?} score={:.1} entry={:.2} sl={:.2} tp1={:.2} tp2={:.2} tp3={:.2}",
            score.direction,
            score.total,
            entree,
            stop_loss,
            take_profit,
            take_profit_2.unwrap_or(0.0),
            take_profit_3.unwrap_or(0.0),
        );

        Ok(Some(Signal {
            direction: score.direction,
            confidence: confiance,
            entry_price: entree,
            stop_loss,
            take_profit,
            take_profit_2,
            take_profit_3,
        }))
    }
}
