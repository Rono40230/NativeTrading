//! Adapter SMC Directionnel pour le backtest — fidèle au live.
//!
//! Utilise le scorer SMC réel, vérifie `sweep_detecte`, applique le filtre
//! Kill Zone si `params.kill_zone_filtre`, et simule via `simulateur::simuler_position`.

use common::{Candle, Direction};
use indicators::calculer_atr;
use strategies::position_tracking::PositionConfig;

use crate::{
    simulateur::simuler_position, BacktestConfig, ParamsSmc, ResultatTrade,
    StrategieParams, TradeBacktest,
};

/// Rejoue la stratégie SMC Directionnel sur la série de bougies.
pub fn rejouer_smc(bougies: &[Candle], config: &BacktestConfig) -> Vec<TradeBacktest> {
    let p = match &config.params {
        StrategieParams::Smc(p) => p.clone(),
        _ => ParamsSmc::default(),
    };

    let risque_usd = config.capital_initial * config.risque_par_trade;
    let mut trades: Vec<TradeBacktest> = Vec::new();
    let min_bougies = (p.atr_periode * 3).max(50);
    let atr = calculer_atr(bougies, p.atr_periode);

    let mut i = min_bougies;
    while i < bougies.len().saturating_sub(1) {
        let atr_courant = atr[i];
        if atr_courant.is_nan() || atr_courant <= 0.0 {
            i += 1;
            continue;
        }

        // Score SMC réel
        let score = match smc::scorer(&bougies[..=i]) {
            Some(s) if s.total >= p.score_min && s.sweep_detecte => s,
            _ => { i += 1; continue; }
        };

        if score.direction == Direction::Both {
            i += 1;
            continue;
        }

        // Filtre Kill Zone si activé
        if p.kill_zone_filtre {
            let b = &bougies[i];
            if !smc::kill_zone::est_en_kill_zone(b.timestamp) {
                i += 1;
                continue;
            }
        }

        let bougie = &bougies[i];
        let entree = bougie.close;
        let is_long = matches!(score.direction, Direction::Long);

        let (sl, tp1, tp2) = if is_long {(
            entree - atr_courant * p.atr_sl,
            entree + atr_courant * p.atr_tp1,
            entree + atr_courant * p.atr_tp2,
        )} else {(
            entree + atr_courant * p.atr_sl,
            entree - atr_courant * p.atr_tp1,
            entree - atr_courant * p.atr_tp2,
        )};
        let tp3 = if is_long {
            Some(entree + atr_courant * p.atr_tp3)
        } else {
            Some(entree - atr_courant * p.atr_tp3)
        };

        let cfg = PositionConfig {
            is_long,
            prix_entree: entree,
            stop_loss:   sl,
            tp1,
            tp2,
            atr:         atr_courant,
            trailing_coeff: 1.5,
            vente_partielle_active: p.vente_partielle,
            pct_cloture_tp1: p.pct_cloture_tp1,
            pct_cloture_tp2: p.pct_cloture_tp2,
        };

        let suite = &bougies[i + 1..];
        let res = simuler_position(&cfg, suite);

        let heure = bougie.timestamp.format("%H").to_string().parse::<u8>().unwrap_or(0);
        let direction = score.direction;

        // Catégorie lisible (TP1 / TP2 / TP3 / SL)
        let categorie = match res.resultat {
            ResultatTrade::Tp3     => "tp3",
            ResultatTrade::Tp2     => "tp2",
            ResultatTrade::Tp1     => "tp1",
            ResultatTrade::StopLoss   => "sl",
            ResultatTrade::NonFerme   => "non_ferme",
        };

        trades.push(TradeBacktest {
            ouvert_a:     bougie.timestamp,
            ferme_a:      res.ferme_a,
            direction,
            prix_entree:  entree,
            prix_sortie:  None,
            stop_loss:    sl,
            take_profit_1: tp1,
            take_profit_2: Some(tp2),
            take_profit_3: tp3,
            resultat:     res.resultat,
            pnl_r:        res.pnl_r,
            pnl_usd:      res.pnl_r * risque_usd,
            heure_ouverture: heure,
            categorie:    categorie.to_string(),
        });

        let bougies_consommees = res.ferme_a
            .and_then(|ts| bougies[i + 1..].iter().position(|b| b.timestamp >= ts))
            .unwrap_or(suite.len().saturating_sub(1));

        i += bougies_consommees + 2;
    }

    trades
}
