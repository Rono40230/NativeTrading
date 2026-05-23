//! Adapter Straddle pour le backtest — fidèle au live.
//!
//! Détecte les mêmes signaux que `StraddleStrategy` (ATR > seuil × moy_ATR)
//! et simule les deux jambes (LONG + SHORT) via `simulateur::simuler_position`.
//!
//! Catégories : "tp1_sl2" | "sl1_tp2" | "double_sl" | "autre"

use common::{Candle, Direction};
use indicators::calculer_atr;
use strategies::position_tracking::PositionConfig;

use crate::{
    simulateur::simuler_position, BacktestConfig, ParamsStraddle, ResultatTrade, StrategieParams,
    TradeBacktest,
};

/// Rejoue la stratégie Straddle sur la série de bougies.
pub fn rejouer_straddle(bougies: &[Candle], config: &BacktestConfig) -> Vec<TradeBacktest> {
    let p = match &config.params {
        StrategieParams::Straddle(p) => p.clone(),
        _ => ParamsStraddle::default(),
    };

    let risque_usd = config.capital_initial * config.risque_par_trade;
    let mut trades: Vec<TradeBacktest> = Vec::new();
    let min_bougies = (p.atr_periode * 2).max(30);
    let atr = calculer_atr(bougies, p.atr_periode);

    let mut i = min_bougies;
    while i < bougies.len().saturating_sub(1) {
        let atr_courant = atr[i];
        if atr_courant.is_nan() || atr_courant <= 0.0 {
            i += 1;
            continue;
        }

        let debut = i.saturating_sub(p.atr_periode);
        let valides: Vec<f64> = atr[debut..i]
            .iter()
            .copied()
            .filter(|v| !v.is_nan() && *v > 0.0)
            .collect();
        if valides.is_empty() {
            i += 1;
            continue;
        }
        let atr_moyen = valides.iter().sum::<f64>() / valides.len() as f64;

        if atr_courant < atr_moyen * p.atr_seuil {
            i += 1;
            continue;
        }

        let bougie = &bougies[i];
        let entree = bougie.close;
        let sl_dist = atr_courant * p.sl_mult;
        let tp1_l = entree + atr_courant * p.tp_mult_1;
        let tp2_l = entree + atr_courant * p.tp_mult_2;
        let tp1_s = entree - atr_courant * p.tp_mult_1;
        let tp2_s = entree - atr_courant * p.tp_mult_2;

        let cfg_long = PositionConfig {
            is_long: true,
            prix_entree: entree,
            stop_loss: entree - sl_dist,
            tp1: tp1_l,
            tp2: tp2_l,
            atr: atr_courant,
            trailing_coeff: p.trailing_atr,
            vente_partielle_active: p.vente_partielle,
            pct_cloture_tp1: p.pct_cloture_tp1,
            pct_cloture_tp2: p.pct_cloture_tp2,
        };
        let cfg_short = PositionConfig {
            is_long: false,
            prix_entree: entree,
            stop_loss: entree + sl_dist,
            tp1: tp1_s,
            tp2: tp2_s,
            atr: atr_courant,
            trailing_coeff: p.trailing_atr,
            vente_partielle_active: p.vente_partielle,
            pct_cloture_tp1: p.pct_cloture_tp1,
            pct_cloture_tp2: p.pct_cloture_tp2,
        };

        let suite = &bougies[i + 1..];
        let res_long = simuler_position(&cfg_long, suite);
        let res_short = simuler_position(&cfg_short, suite);

        let pnl_r = res_long.pnl_r + res_short.pnl_r;

        let res_gagnant = if !matches!(
            res_long.resultat,
            ResultatTrade::StopLoss | ResultatTrade::NonFerme
        ) {
            Some(&res_long.resultat)
        } else if !matches!(
            res_short.resultat,
            ResultatTrade::StopLoss | ResultatTrade::NonFerme
        ) {
            Some(&res_short.resultat)
        } else {
            None
        };

        // Catégorie = TP atteint par la jambe gagnante, ou double_sl
        let categorie = match res_gagnant {
            Some(ResultatTrade::Tp3) => "Tp3",
            Some(ResultatTrade::Tp2) => "Tp2",
            Some(ResultatTrade::Tp1) => "Tp1",
            None => "double_sl",
            _ => "Tp1",
        };

        let heure = bougie
            .timestamp
            .format("%H")
            .to_string()
            .parse::<u8>()
            .unwrap_or(0);
        let ferme_a = res_long.ferme_a.or(res_short.ferme_a);

        trades.push(TradeBacktest {
            ouvert_a: bougie.timestamp,
            ferme_a,
            direction: Direction::Both,
            prix_entree: entree,
            prix_sortie: None,
            stop_loss: entree - sl_dist,
            take_profit_1: tp1_l,
            take_profit_2: Some(tp2_l),
            take_profit_3: Some(entree + atr_courant * p.tp_mult_3),
            resultat: if pnl_r >= 0.0 {
                ResultatTrade::Tp1
            } else {
                ResultatTrade::StopLoss
            },
            pnl_r,
            pnl_usd: pnl_r * risque_usd,
            heure_ouverture: heure,
            categorie: categorie.to_string(),
        });

        let bougies_consommees = res_long
            .ferme_a
            .or(res_short.ferme_a)
            .and_then(|ts| bougies[i + 1..].iter().position(|b| b.timestamp >= ts))
            .unwrap_or(suite.len().saturating_sub(1));

        i += bougies_consommees + 2;
    }

    trades
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::{Asset, Timeframe};

    fn bougie(ts_offset_min: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc::now() + chrono::Duration::minutes(ts_offset_min),
            open,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    fn serie_spike_atr() -> Vec<Candle> {
        let mut v: Vec<Candle> = (0..40)
            .map(|i| bougie(i as i64, 100.0, 100.5, 99.5, 100.0))
            .collect();
        v.push(bougie(40, 100.0, 120.0, 80.0, 110.0));
        for i in 0..30 {
            v.push(bougie(41 + i as i64, 110.0, 150.0, 108.0, 140.0));
        }
        v
    }

    #[test]
    fn straddle_detecte_signal_sur_spike_atr() {
        let bougies = serie_spike_atr();
        let config = BacktestConfig {
            asset: Asset::XAUUSD,
            timeframe: Timeframe::M15,
            debut: bougies.first().unwrap().timestamp,
            fin: bougies.last().unwrap().timestamp,
            strategie: crate::StrategieType::Straddle,
            capital_initial: 10_000.0,
            risque_par_trade: 0.01,
            params: StrategieParams::Straddle(ParamsStraddle::default()),
        };
        let trades = rejouer_straddle(&bougies, &config);
        assert!(
            !trades.is_empty(),
            "Aucun trade Straddle détecté sur le spike ATR"
        );
    }
}
