use crate::calculs::{SortieType, TradeDirection, TradeSimule};
use crate::BacktestResults;
use common::Result;

pub(crate) fn resultats_vides(capital_initial: f64) -> BacktestResults {
    BacktestResults {
        total_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        win_rate: 0.0,
        capital_initial,
        capital_final: capital_initial,
        roi_pct: 0.0,
        profit_net: 0.0,
        sharpe_ratio: 0.0,
        max_drawdown_pct: 0.0,
        profit_factor: 0.0,
        nb_tp1: 0,
        nb_tp2: 0,
        nb_sl: 0,
        nb_expirations: 0,
        nb_straddles: 0,
        equity_curve: Vec::new(),
    }
}
pub(crate) fn calculer_resultats(
    trades: Vec<TradeSimule>,
    equity: Vec<f64>,
    capital_initial: f64,
    capital_final: f64,
    capital_max: f64,
) -> Result<BacktestResults> {
    let total = trades.len() as u32;

    let gagnants = trades
        .iter()
        .filter(|t| match t.direction {
            TradeDirection::Long => t.prix_sortie > t.prix_entree,
            TradeDirection::Short => t.prix_sortie < t.prix_entree,
        })
        .count() as u32;

    // ── Statistiques pyramidalisation ──────────────────────────────────────────
    let trades_avec_sortie: Vec<_> = trades.iter().filter(|t| t.sortie.is_some()).collect();
    let nb_tp1 = trades_avec_sortie
        .iter()
        .filter(|t| matches!(t.sortie, Some(SortieType::Tp1)))
        .count() as u32;
    let nb_tp2 = trades_avec_sortie
        .iter()
        .filter(|t| matches!(t.sortie, Some(SortieType::Tp2)))
        .count() as u32;
    let nb_sl = trades_avec_sortie
        .iter()
        .filter(|t| matches!(t.sortie, Some(SortieType::Sl)))
        .count() as u32;
    let nb_exp = trades_avec_sortie
        .iter()
        .filter(|t| matches!(t.sortie, Some(SortieType::Expiration)))
        .count() as u32;

    let win_rate = if total > 0 {
        gagnants as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let roi_pct = (capital_final - capital_initial) / capital_initial * 100.0;
    let profit_net = capital_final - capital_initial;

    let min_equity = equity.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_drawdown_pct = if capital_max > 0.0 {
        (capital_max - min_equity) / capital_max * 100.0
    } else {
        0.0
    };

    let (profits_bruts, pertes_brutes) = trades.iter().fold((0.0f64, 0.0f64), |(p, l), t| {
        let pnl = match t.direction {
            TradeDirection::Long => t.prix_sortie - t.prix_entree,
            TradeDirection::Short => t.prix_entree - t.prix_sortie,
        };
        if pnl > 0.0 {
            (p + pnl, l)
        } else {
            (p, l + pnl.abs())
        }
    });
    let profit_factor = if pertes_brutes > 0.0 {
        profits_bruts / pertes_brutes
    } else {
        profits_bruts
    };

    let sharpe = calculer_sharpe(&equity);

    tracing::info!(
        "Backtest: {} trades ({} straddles, win={:.1}%, gains={}, pertes={}) ROI={:.2}% Sharpe={:.2} MaxDD={:.1}% | TP1={} TP2={} SL={} Exp={}",
        total, total / 2, win_rate, gagnants, total - gagnants,
        roi_pct, sharpe, max_drawdown_pct,
        nb_tp1, nb_tp2, nb_sl, nb_exp
    );

    Ok(BacktestResults {
        total_trades: total,
        winning_trades: gagnants,
        losing_trades: total - gagnants,
        win_rate,
        capital_initial,
        capital_final,
        roi_pct,
        profit_net,
        sharpe_ratio: sharpe,
        max_drawdown_pct,
        profit_factor,
        nb_tp1,
        nb_tp2,
        nb_sl,
        nb_expirations: nb_exp,
        nb_straddles: total / 2,
        equity_curve: Vec::new(),
    })
}

fn calculer_sharpe(equity: &[f64]) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let rendements: Vec<f64> = equity
        .windows(2)
        .map(|w| (w[1] - w[0]) / w[0].max(1e-10))
        .collect();
    let n = rendements.len() as f64;
    let moy = rendements.iter().sum::<f64>() / n;
    let var = rendements.iter().map(|r| (r - moy).powi(2)).sum::<f64>() / n.max(1.0);
    let std = var.sqrt();
    if std < 1e-10 {
        0.0
    } else {
        (moy / std) * 252f64.sqrt()
    }
}
