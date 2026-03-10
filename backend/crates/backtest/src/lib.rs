use calculs::{calculer_resultats, simuler_sortie, TradeDirection, TradeSimule};
use common::{Candle, Direction, Result};
use serde::{Deserialize, Serialize};
use strategies::Strategy;

mod calculs;

/// Résultats complets d'un backtest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResults {
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f64,
    pub capital_initial: f64,
    pub capital_final: f64,
    pub roi_pct: f64,
    pub profit_net: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown_pct: f64,
    pub profit_factor: f64,
}

/// Moteur de backtesting — rejoue les bougies et simule les trades
pub struct BacktestEngine {
    pub capital_initial: f64,
    /// Spread + slippage simulés (% du prix)
    pub cout_friction_pct: f64,
    /// Risk par trade en % du capital
    pub risk_par_trade_pct: f64,
}

impl BacktestEngine {
    pub fn new(capital_initial: f64) -> Self {
        Self {
            capital_initial,
            cout_friction_pct: 0.0003,
            risk_par_trade_pct: 0.02,
        }
    }

    /// Lance un backtest walk-forward sur les bougies fournies.
    pub fn run(&self, bougies: &[Candle], strategy: &dyn Strategy) -> Result<BacktestResults> {
        if bougies.len() < 62 {
            return Ok(self.resultats_vides());
        }

        let mut capital = self.capital_initial;
        let mut equity: Vec<f64> = vec![capital];
        let mut trades: Vec<TradeSimule> = Vec::new();
        let mut capital_max = capital;
        let fenetre = 60usize;

        for i in fenetre..bougies.len().saturating_sub(1) {
            let slice = &bougies[i.saturating_sub(fenetre)..i];

            let signal = match strategy.analyze(slice) {
                Ok(Some(s)) => s,
                Ok(None) => continue,
                Err(e) => {
                    tracing::debug!("Backtest candle {}: {}", i, e);
                    continue;
                }
            };

            let prochaine = &bougies[i];
            let friction = self.cout_friction_pct;
            let horizon = (i + 5).min(bougies.len() - 1);
            let horizon_bougies = &bougies[i..=horizon];

            let directions: &[TradeDirection] = match signal.direction {
                Direction::Long => &[TradeDirection::Long],
                Direction::Short => &[TradeDirection::Short],
                Direction::Both => &[TradeDirection::Long, TradeDirection::Short],
            };

            for dir in directions {
                let prix_entree = match dir {
                    TradeDirection::Long => prochaine.open * (1.0 + friction),
                    TradeDirection::Short => prochaine.open * (1.0 - friction),
                };

                let (tp, sl) = match dir {
                    TradeDirection::Long => (signal.take_profit, signal.stop_loss),
                    TradeDirection::Short => {
                        let dist = signal.entry_price - signal.stop_loss;
                        (
                            prix_entree - (signal.take_profit - signal.entry_price),
                            prix_entree + dist,
                        )
                    }
                };

                let prix_sortie =
                    simuler_sortie(horizon_bougies, dir, tp, sl, prochaine.close);
                let dist_sl = (prix_entree - sl).abs().max(1e-10);
                let taille_pos = (capital * self.risk_par_trade_pct) / dist_sl;

                let pnl = match dir {
                    TradeDirection::Long => (prix_sortie - prix_entree) * taille_pos,
                    TradeDirection::Short => (prix_entree - prix_sortie) * taille_pos,
                };

                capital = (capital + pnl).max(0.0);
                if capital > capital_max {
                    capital_max = capital;
                }
                equity.push(capital);
                trades.push(TradeSimule {
                    prix_entree,
                    prix_sortie,
                    direction: dir.clone(),
                });
            }
        }

        calculer_resultats(trades, equity, self.capital_initial, capital, capital_max)
    }

    fn resultats_vides(&self) -> BacktestResults {
        BacktestResults {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            capital_initial: self.capital_initial,
            capital_final: self.capital_initial,
            roi_pct: 0.0,
            profit_net: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown_pct: 0.0,
            profit_factor: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

    fn bougie(close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: close,
            high: close * 1.01,
            low: close * 0.99,
            close,
            volume: 1000.0,
        }
    }

    struct StrategieVide;
    impl strategies::Strategy for StrategieVide {
        fn analyze(&self, _: &[Candle]) -> common::Result<Option<strategies::Signal>> {
            Ok(None)
        }
    }

    #[test]
    fn backtest_sans_trades_retourne_capital_initial() {
        let bougies: Vec<Candle> = (1..=70).map(|i| bougie(i as f64 * 100.0)).collect();
        let engine = BacktestEngine::new(2000.0);
        let resultats = engine.run(&bougies, &StrategieVide).unwrap();
        assert_eq!(resultats.total_trades, 0);
        assert!((resultats.capital_final - 2000.0).abs() < 1e-10);
        assert!((resultats.roi_pct).abs() < 1e-10);
    }

    #[test]
    fn backtest_peu_de_bougies_retourne_vide() {
        let bougies: Vec<Candle> = (1..=10).map(|i| bougie(i as f64 * 100.0)).collect();
        let engine = BacktestEngine::new(2000.0);
        let resultats = engine.run(&bougies, &StrategieVide).unwrap();
        assert_eq!(resultats.total_trades, 0);
    }
}
