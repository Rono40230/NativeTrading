//! Moteur de replay bougie par bougie.
//!
//! Dispatche vers l'adapter correspondant à la stratégie configurée,
//! puis calcule les métriques globales sur la liste de trades résultante.

use anyhow::Result;
use common::Candle;

use crate::{
    metriques::{
        calculer_capital_min, calculer_drawdown_max, calculer_perf_annualisee,
        calculer_profit_factor, calculer_series_max, calculer_sharpe, calculer_stats_par_heure,
        calculer_stats_par_jour, calculer_win_rate, identifier_fenetres_propices,
    },
    BacktestConfig, BacktestResult, StrategieType, TradeBacktest,
};

/// Point d'entrée principal : rejoue les bougies selon la config et retourne
/// les métriques complètes.
///
/// Les bougies doivent être triées chronologiquement et appartenir à l'asset/TF
/// spécifié dans `config`. Les bougies hors de la fenêtre `debut..fin` sont filtrées
/// automatiquement.
pub fn rejouer(bougies: &[Candle], config: BacktestConfig) -> Result<BacktestResult> {
    // Filtrer la fenêtre temporelle
    let bougies_fenetre: Vec<&Candle> = bougies
        .iter()
        .filter(|b| b.timestamp >= config.debut && b.timestamp <= config.fin)
        .collect();

    if bougies_fenetre.len() < 30 {
        return Err(anyhow::anyhow!(
            "Données insuffisantes pour le backtest : {} bougies (minimum 30)",
            bougies_fenetre.len()
        ));
    }

    // Reconstituer en owned pour les adapters
    let bougies_owned: Vec<Candle> = bougies_fenetre.into_iter().cloned().collect();

    // Dispatcher vers l'adapter stratégie
    let trades: Vec<TradeBacktest> = match &config.strategie {
        StrategieType::Straddle => crate::straddle::rejouer_straddle(&bougies_owned, &config),
        StrategieType::Smc => crate::smc::rejouer_smc(&bougies_owned, &config),
        StrategieType::Rockets => crate::rockets::rejouer_rockets(&bougies_owned, &config),
    };

    Ok(assembler_resultat(trades, config))
}

/// Calcule toutes les métriques et assemble le `BacktestResult`.
pub(crate) fn assembler_resultat(
    trades: Vec<TradeBacktest>,
    config: BacktestConfig,
) -> BacktestResult {
    let nb_trades = trades.len();
    let win_rate = calculer_win_rate(&trades);
    let profit_factor = calculer_profit_factor(&trades);
    let pnl_total_r: f64 = trades.iter().map(|t| t.pnl_r).sum();
    let pnl_r_moyen = if nb_trades > 0 { pnl_total_r / nb_trades as f64 } else { 0.0 };

    // Série de capital pour drawdown + equity curve
    let mut capital = config.capital_initial;
    let risque_usd = capital * config.risque_par_trade;
    let equity_curve: Vec<f64> = trades
        .iter()
        .map(|t| {
            capital += t.pnl_r * risque_usd;
            capital
        })
        .collect();

    let capital_final = equity_curve.last().copied().unwrap_or(config.capital_initial);
    let drawdown_max = calculer_drawdown_max(&equity_curve);
    let capital_min = calculer_capital_min(&equity_curve, config.capital_initial);

    let nb_jours = (config.fin - config.debut).num_days().max(1) as u32;
    let perf_annualisee = calculer_perf_annualisee(config.capital_initial, capital_final, nb_jours);

    let (serie_gains_max, serie_pertes_max) = calculer_series_max(&trades);

    let pnl_serie: Vec<f64> = trades.iter().map(|t| t.pnl_r).collect();
    let sharpe = calculer_sharpe(&pnl_serie);

    let stats_par_heure = calculer_stats_par_heure(&trades);
    let stats_par_jour = calculer_stats_par_jour(&trades);

    // Métriques Straddle spécifiques
    let (double_sl_rate, double_win_rate, fenetres_propices) = if config.strategie == StrategieType::Straddle {
        let n = nb_trades as f64;
        if n > 0.0 {
            let dsl = trades.iter().filter(|t| t.categorie == "double_sl").count() as f64 / n;
            let dwn = trades
                .iter()
                .filter(|t| t.categorie == "double_win")
                .count() as f64
                / n;
            let fp = identifier_fenetres_propices(&trades);
            (Some(dsl), Some(dwn), Some(fp))
        } else {
            (Some(0.0), Some(0.0), Some(vec![]))
        }
    } else {
        (None, None, None)
    };

    BacktestResult {
        config,
        nb_trades,
        win_rate,
        profit_factor,
        sharpe,
        drawdown_max,
        capital_final,
        pnl_total_r,
        pnl_r_moyen,
        perf_annualisee,
        capital_min,
        serie_pertes_max,
        serie_gains_max,
        double_sl_rate,
        double_win_rate,
        stats_par_heure,
        stats_par_jour,
        equity_curve,
        fenetres_propices,
        trades,
    }
}
