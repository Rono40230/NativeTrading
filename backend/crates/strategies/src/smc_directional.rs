use super::{Signal, Strategy};
use common::{Candle, Direction, Result};
use db::strategies_params::SmcParams;
use indicators::calculer_atr;
use smc::{kill_zone, scorer, sweep};

/// Stratégie SMC Directionnelle — scoring confluence configurable (défaut ≥70/100).
///
/// Déclencheur : Score SMC ≥ params.score_min (Tendance + OB + Imbalance + IFVG + Fibonacci).
/// Entrée : close actuel | SL : ATR×atr_sl | TP1-3 : ATR×atr_tp1/2/3
/// Risk : 1.5% par trade
#[derive(Default)]
pub struct SmcDirectionalStrategy {
    pub params: SmcParams,
}

impl Strategy for SmcDirectionalStrategy {
    fn analyze(&self, bougies: &[Candle]) -> Result<Option<Signal>> {
        if bougies.len() < 30 {
            return Ok(None);
        }

        // Kill Zone ICT — désactivable via params (London 07h-10h / NY 13h30-16h30 UTC)
        let last_ts = match bougies.last() {
            Some(b) => b.timestamp,
            None => return Ok(None),
        };
        if self.params.kill_zone_filtre && !kill_zone::est_en_kill_zone(last_ts) {
            tracing::debug!(
                "SMC {}: hors Kill Zone à {} — signal ignoré",
                std::any::type_name::<Self>(),
                last_ts.format("%H:%M UTC")
            );
            return Ok(None);
        }

        // Liquidity Sweep — faux breakout d'un swing high/low requis
        if sweep::detecter_sweep(bougies).is_none() {
            return Ok(None);
        }

        let score = match scorer(bougies) {
            Some(s) if s.total >= self.params.score_min as f64 => s,
            _ => return Ok(None),
        };

        let atr = calculer_atr(bougies, self.params.atr_periode as usize);
        let n = bougies.len();
        let atr_val = atr[n - 1];
        if atr_val.is_nan() || atr_val <= 0.0 {
            return Ok(None);
        }

        let prix_entree = bougies[n - 1].close;
        let confiance = (score.total / 100.0).min(1.0);

        let (stop_loss, take_profit, take_profit_2, take_profit_3) = match score.direction {
            Direction::Long => (
                prix_entree - atr_val * self.params.atr_sl,
                prix_entree + atr_val * self.params.atr_tp1,
                Some(prix_entree + atr_val * self.params.atr_tp2),
                Some(prix_entree + atr_val * self.params.atr_tp3),
            ),
            Direction::Short => (
                prix_entree + atr_val * self.params.atr_sl,
                prix_entree - atr_val * self.params.atr_tp1,
                Some(prix_entree - atr_val * self.params.atr_tp2),
                Some(prix_entree - atr_val * self.params.atr_tp3),
            ),
            Direction::Both => return Ok(None),
        };

        tracing::debug!(
            "SMC signal {:?} score={:.1} entry={:.2} sl={:.2} tp1={:.2} tp2={:.2} tp3={:.2}",
            score.direction,
            score.total,
            prix_entree,
            stop_loss,
            take_profit,
            take_profit_2.unwrap_or(0.0),
            take_profit_3.unwrap_or(0.0),
        );

        Ok(Some(Signal {
            direction: score.direction,
            confiance,
            prix_entree,
            stop_loss,
            take_profit,
            take_profit_2,
            take_profit_3,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn b(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    #[test]
    fn analyze_none_si_moins_de_30_bougies() {
        let strat = SmcDirectionalStrategy::default();
        let bougies: Vec<Candle> = (0..29)
            .map(|i| {
                b(
                    i as f64 + 10.,
                    i as f64 + 11.,
                    i as f64 + 9.,
                    i as f64 + 10.5,
                )
            })
            .collect();
        let result = strat.analyze(&bougies);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "Moins de 30 bougies → None");
    }
}
