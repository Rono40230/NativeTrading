use calculs::{
    calculer_resultats, simuler_sortie, simuler_sortie_pyramidal, TradeDirection, TradeSimule,
};
use common::{Candle, Direction, Result};
use serde::{Deserialize, Serialize};
use strategies::Strategy;
mod calculs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: i64,
    pub capital: f64,
}
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
    /// Trades SMC sortis exactement à TP1 (⅓ fermé, SL → BE, reste annulé)
    pub nb_tp1: u32,
    /// Trades SMC sortis exactement à TP2 (⅔ fermé)
    pub nb_tp2: u32,
    /// Trades SMC sortis à TP3 complet
    pub nb_tp3: u32,
    /// Trades stoppés (SL ou BE après TP1)
    pub nb_sl: u32,
    /// Trades fermés à l'expiration de l'horizon (ni TP ni SL atteints)
    pub nb_expirations: u32,
    /// Nombre de Straddles posés (= total_trades / 2 car Long+Short par signal)
    pub nb_straddles: u32,
    pub equity_curve: Vec<EquityPoint>,
}

/// Données de feedback d'un trade simulé pour raffinement du pipeline ML.
/// Contient l'index de la bougie d'entrée et le résultat (gagné/perdu).
pub struct FeedbackTrade {
    pub indice_entree: usize,
    pub gagne: bool,
}
/// Moteur de backtesting — rejoue les bougies et simule les trades
pub struct BacktestEngine {
    pub capital_initial: f64,
    /// Spread + slippage simulés (% du prix)
    pub cout_friction_pct: f64,
    /// Risk par trade en % du capital
    pub risk_par_trade_pct: f64,
    /// Nombre de bougies APRÈS l'entrée formant l'horizon d'expiration.
    /// Calculé côté handler depuis `horizon_minutes / timeframe.minutes()`.
    /// Défaut : 5 bougies (compatible M5 = 25 min, proche du créneau Straddle).
    pub horizon_bougies: usize,
}

impl BacktestEngine {
    pub fn new(capital_initial: f64) -> Self {
        Self {
            capital_initial,
            cout_friction_pct: 0.0003,
            risk_par_trade_pct: 0.02,
            horizon_bougies: 5,
        }
    }

    /// Lance un backtest walk-forward sur les bougies fournies.
    pub fn run(&self, bougies: &[Candle], strategy: &dyn Strategy) -> Result<BacktestResults> {
        self.run_interne(bougies, strategy).map(|(r, _)| r)
    }

    /// Identique à `run()` mais retourne aussi le feedback par trade
    /// (index bougie + résultat gagnant/perdant) pour raffiner le pipeline ML.
    pub fn run_avec_feedback(
        &self,
        bougies: &[Candle],
        strategy: &dyn Strategy,
    ) -> Result<(BacktestResults, Vec<FeedbackTrade>)> {
        self.run_interne(bougies, strategy)
    }

    fn run_interne(
        &self,
        bougies: &[Candle],
        strategy: &dyn Strategy,
    ) -> Result<(BacktestResults, Vec<FeedbackTrade>)> {
        if bougies.len() < 62 {
            return Ok((self.resultats_vides(), Vec::new()));
        }

        let fenetre = 60usize;
        let mut capital = self.capital_initial;
        let mut equity: Vec<f64> = vec![capital];
        let mut equity_curve = vec![EquityPoint {
            timestamp: bougies[fenetre - 1].timestamp.timestamp(),
            capital,
        }];
        let mut trades: Vec<TradeSimule> = Vec::new();
        let mut feedback: Vec<FeedbackTrade> = Vec::new();
        let mut capital_max = capital;

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
            // Horizon commençant à i+1 pour éviter le look-ahead sur la bougie d'entrée.
            // Straddle (Both) : horizon illimité → chaque jambe scanne jusqu'à toucher un niveau.
            // SMC (Long/Short) : horizon limité à horizon_bougies (ex. 4h).
            let horizon_bougies: &[Candle] = match signal.direction {
                Direction::Both => &bougies[i + 1..],
                _ => {
                    let horizon = (i + 1 + self.horizon_bougies).min(bougies.len());
                    &bougies[i + 1..horizon]
                }
            };

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

                // Recalcul des niveaux TP/SL depuis l'entrée réelle (open ± friction).
                // Straddle (Both) : TPs fournis en perspective Long → mirroir pour Short.
                // SMC (Long/Short) : TPs déjà direction-ajustés → offset direct.
                let (tp1, sl) = match signal.direction {
                    Direction::Both => match dir {
                        TradeDirection::Long => (signal.take_profit, signal.stop_loss),
                        TradeDirection::Short => {
                            let dist_tp = signal.take_profit - signal.prix_entree;
                            let dist_sl = signal.prix_entree - signal.stop_loss;
                            (prix_entree - dist_tp, prix_entree + dist_sl)
                        }
                    },
                    _ => {
                        let tp = prix_entree + (signal.take_profit - signal.prix_entree);
                        let sl = prix_entree + (signal.stop_loss - signal.prix_entree);
                        (tp, sl)
                    }
                };

                // Sortie pyramidale si TP2/TP3 disponibles (SMC), sinon sortie simple (Straddle)
                let (prix_sortie, sortie_type) = match (signal.take_profit_2, signal.take_profit_3)
                {
                    (Some(tp2_sig), Some(tp3_sig)) => {
                        // Les niveaux stockés sont les offsets Long (> prix_entree).
                        // Pour la jambe Short (Direction::Both), les TP doivent être sous le prix d'entrée.
                        let dist_tp2 = tp2_sig - signal.prix_entree;
                        let dist_tp3 = tp3_sig - signal.prix_entree;
                        let (tp2, tp3) = match (&signal.direction, &dir) {
                            (Direction::Both, TradeDirection::Short) => {
                                (prix_entree - dist_tp2, prix_entree - dist_tp3)
                            }
                            _ => (prix_entree + dist_tp2, prix_entree + dist_tp3),
                        };
                        simuler_sortie_pyramidal(
                            horizon_bougies,
                            dir,
                            prix_entree,
                            tp1,
                            tp2,
                            tp3,
                            sl,
                        )
                    }
                    _ => simuler_sortie(horizon_bougies, dir, tp1, sl, prochaine.close),
                };
                let dist_sl = (prix_entree - sl).abs().max(1e-10);
                let taille_pos = (capital * self.risk_par_trade_pct) / dist_sl;

                let pnl = match dir {
                    TradeDirection::Long => (prix_sortie - prix_entree) * taille_pos,
                    TradeDirection::Short => (prix_entree - prix_sortie) * taille_pos,
                };

                let gagne = pnl > 0.0;
                capital = (capital + pnl).max(0.0);
                if capital > capital_max {
                    capital_max = capital;
                }
                equity.push(capital);
                trades.push(TradeSimule {
                    prix_entree,
                    prix_sortie,
                    direction: dir.clone(),
                    sortie: Some(sortie_type),
                });
                feedback.push(FeedbackTrade {
                    indice_entree: i,
                    gagne,
                });
            }

            if trades.last().is_some() {
                equity_curve.push(EquityPoint {
                    timestamp: prochaine.timestamp.timestamp(),
                    capital,
                });
            }
        }

        if equity_curve.len() == 1 {
            equity_curve.push(EquityPoint {
                timestamp: bougies.last().map(|b| b.timestamp.timestamp()).unwrap_or_default(),
                capital,
            });
        }

        let mut resultats = calculer_resultats(trades, equity, self.capital_initial, capital, capital_max)?;
        resultats.equity_curve = equity_curve;
        Ok((resultats, feedback))
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
            nb_tp1: 0,
            nb_tp2: 0,
            nb_tp3: 0,
            nb_sl: 0,
            nb_expirations: 0,
            nb_straddles: 0,
            equity_curve: Vec::new(),
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
