use calculs::{
    simuler_sortie, simuler_sortie_pyramidal, OptionsGestion, ParamsPyramidal, TradeDirection,
    TradeSimule,
};
use common::{Candle, Direction, Result};
use indicators::calculer_atr;
use stats::calculer_resultats;
use straddle_hybride::{preparer_jambes_straddle, JambeStraddle, NiveauxSignalStraddle, ParamsMoteurStraddle};
use strategies::Strategy;
mod calculs;
mod stats;
mod straddle_hybride;
mod types;
pub use types::{BacktestResults, EquityPoint, FeedbackTrade};

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
    /// Trailing stop Straddle : SL remonte à peak - ATR × mult (None = désactivé).
    pub trailing_atr_mult: Option<f64>,
    /// Break-even Straddle : quand gain > ATR × mult, SL → prix d'entrée (None = désactivé).
    pub be_atr_mult: Option<f64>,
    /// true (défaut) = vente partielle (⅓ à TP1, ⅓ à TP2, ⅓ au trailing).
    /// false = lot entier sorti au trailing, SL déplacé seulement.
    pub vente_partielle: bool,
}

impl BacktestEngine {
    pub fn new(capital_initial: f64) -> Self {
        Self {
            capital_initial,
            cout_friction_pct: 0.0003,
            risk_par_trade_pct: 0.02,
            horizon_bougies: 5,
            trailing_atr_mult: None,
            be_atr_mult: None,
            vente_partielle: true,
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

            // ATR courant — calculé une fois, partagé entre straddle hybride et boucle directionnelle
            let atr_courant = {
                let atr_vals = calculer_atr(slice, 14);
                atr_vals.last().copied().unwrap_or(0.0)
            };

            // Straddle hybride : Long + Short simultanés, jambe survivante bascule en SMC
            if matches!(signal.direction, Direction::Both) {
                if let Some(tp2_sig) = signal.take_profit_2 {
                    let jambes = preparer_jambes_straddle(
                        horizon_bougies,
                        NiveauxSignalStraddle {
                            take_profit: signal.take_profit,
                            stop_loss: signal.stop_loss,
                            prix_entree: signal.prix_entree,
                            tp2: tp2_sig,
                        },
                        ParamsMoteurStraddle {
                            open: prochaine.open,
                            friction,
                            atr: atr_courant,
                            trailing_mult: self.trailing_atr_mult.unwrap_or(1.5),
                            be: self.be_atr_mult.map(|m| (atr_courant, m)),
                            vente_partielle: self.vente_partielle,
                        },
                    );
                    for JambeStraddle { prix_entree, prix_sortie, direction, sortie, sl_ref } in jambes {
                        let dist = (prix_entree - sl_ref).abs().max(1e-10);
                        let taille = (capital * self.risk_par_trade_pct) / dist;
                        let pnl = match direction {
                            calculs::TradeDirection::Long => (prix_sortie - prix_entree) * taille,
                            calculs::TradeDirection::Short => (prix_entree - prix_sortie) * taille,
                        };
                        capital = (capital + pnl).max(0.0);
                        if capital > capital_max { capital_max = capital; }
                        equity.push(capital);
                        trades.push(TradeSimule { prix_entree, prix_sortie, direction, sortie: Some(sortie) });
                        feedback.push(FeedbackTrade { indice_entree: i, gagne: pnl > 0.0 });
                    }
                    equity_curve.push(EquityPoint { timestamp: prochaine.timestamp.timestamp(), capital });
                    continue;
                }
            }

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

                // Sortie pyramidale si TP2/TP3 disponibles (SMC), sinon sortie simple
                let (prix_sortie, sortie_type) = match (signal.take_profit_2, signal.take_profit_3)
                {
                    (Some(tp2_sig), Some(_)) => {
                        let dist_tp2 = tp2_sig - signal.prix_entree;
                        let tp2 = match (&signal.direction, dir) {
                            (Direction::Both, TradeDirection::Short) => prix_entree - dist_tp2,
                            _ => prix_entree + dist_tp2,
                        };
                        let trailing_mult = self.trailing_atr_mult.unwrap_or(1.5);
                        simuler_sortie_pyramidal(
                            horizon_bougies,
                            dir,
                            ParamsPyramidal {
                                prix_entree,
                                tp1,
                                tp2,
                                trailing_tp3: (atr_courant, trailing_mult),
                                sl_initial: sl,
                                vente_partielle: self.vente_partielle,
                            },
                        )
                    }
                    _ => {
                        let trailing = self.trailing_atr_mult.map(|mult| (atr_courant, mult));
                        let be = self.be_atr_mult.map(|mult| (atr_courant, mult));
                        simuler_sortie(
                            horizon_bougies,
                            dir,
                            tp1,
                            sl,
                            prochaine.close,
                            OptionsGestion {
                                prix_entree,
                                trailing_atr: trailing,
                                be_atr: be,
                            },
                        )
                    }
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
                timestamp: bougies
                    .last()
                    .map(|b| b.timestamp.timestamp())
                    .unwrap_or_default(),
                capital,
            });
        }

        let mut resultats =
            calculer_resultats(trades, equity, self.capital_initial, capital, capital_max)?;
        resultats.equity_curve = equity_curve;
        Ok((resultats, feedback))
    }

    fn resultats_vides(&self) -> BacktestResults {
        stats::resultats_vides(self.capital_initial)
    }
}

#[cfg(test)]
mod lib_tests;
