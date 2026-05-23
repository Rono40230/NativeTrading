//! Adapter Rockets VCP pour le backtest.
//!
//! Détecte les patterns de contraction de volatilité (VCP) sur la série de bougies
//! et simule les entrées avec trailing stop ATR.

use common::{Candle, Direction};
use indicators::calculer_atr;
use strategies::position_tracking::PositionConfig;

use crate::{simulateur::simuler_position, BacktestConfig, TradeBacktest};

/// Rejoue la stratégie Rockets VCP sur la série de bougies.
///
/// Détection VCP simplifiée pour le backtest :
/// - Contraction : ATR actuel < 60% de l'ATR moyen sur 20 périodes
/// - Breakout : close dépasse le high des 10 dernières bougies
///
/// SL = dernier low des 5 bougies, TP = 2×(entree - SL)
pub fn rejouer_rockets(bougies: &[Candle], config: &BacktestConfig) -> Vec<TradeBacktest> {
    let risque_usd = config.capital_initial * config.risque_par_trade;
    let mut trades: Vec<TradeBacktest> = Vec::new();
    let min_bougies = 30usize;
    let atr = calculer_atr(bougies, 14);

    let mut i = min_bougies;
    while i < bougies.len().saturating_sub(1) {
        let atr_courant = atr[i];
        if atr_courant.is_nan() || atr_courant <= 0.0 {
            i += 1;
            continue;
        }

        // ATR moyen sur 20 périodes pour détecter la contraction
        let debut_fenetre = i.saturating_sub(20);
        let atr_moyen: f64 = atr[debut_fenetre..i]
            .iter()
            .filter(|v| !v.is_nan() && **v > 0.0)
            .copied()
            .sum::<f64>()
            / (i - debut_fenetre) as f64;

        if atr_moyen <= 0.0 {
            i += 1;
            continue;
        }

        // Détection contraction VCP : ATR compressé
        let contraction = atr_courant < atr_moyen * 0.6;
        if !contraction {
            i += 1;
            continue;
        }

        // Détection breakout : close > high des 10 dernières bougies
        let debut_range = i.saturating_sub(10);
        let high_range = bougies[debut_range..i]
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);

        let bougie_signal = &bougies[i];
        if bougie_signal.close <= high_range {
            i += 1;
            continue;
        }

        // Signal breakout détecté
        let prix_entree = bougie_signal.close;

        // SL = low des 5 dernières bougies (swing low récent)
        let debut_sl = i.saturating_sub(5);
        let sl = bougies[debut_sl..i]
            .iter()
            .map(|b| b.low)
            .fold(f64::INFINITY, f64::min);

        let dist_sl = (prix_entree - sl).abs();
        if dist_sl <= 0.0 {
            i += 1;
            continue;
        }

        let tp1 = prix_entree + dist_sl * 2.0;
        let tp2 = prix_entree + dist_sl * 3.0;

        let cfg = PositionConfig {
            is_long: true,
            prix_entree,
            stop_loss:    sl,
            tp1,
            tp2,
            atr:          atr_courant,
            trailing_coeff: 1.5,
            vente_partielle_active: false,
            pct_cloture_tp1: 0.0,
            pct_cloture_tp2: 0.0,
        };

        let suite = &bougies[i + 1..];
        let res = simuler_position(&cfg, suite);

        let heure = bougie_signal.timestamp.format("%H").to_string().parse::<u8>().unwrap_or(0);

        trades.push(TradeBacktest {
            ouvert_a: bougie_signal.timestamp,
            ferme_a: res.ferme_a,
            direction: Direction::Long,
            prix_entree,
            prix_sortie: None,
            stop_loss: sl,
            take_profit_1: tp1,
            take_profit_2: Some(tp2),
            take_profit_3: None,
            resultat:  res.resultat,
            pnl_r:     res.pnl_r,
            pnl_usd:   res.pnl_r * risque_usd,
            heure_ouverture: heure,
            categorie: "breakout_vcp".to_string(),
        });

        let bougies_consommees = res.ferme_a
            .and_then(|ts| bougies[i + 1..].iter().position(|b| b.timestamp >= ts))
            .unwrap_or(suite.len().saturating_sub(1));

        i += bougies_consommees + 2;
    }

    trades
}
