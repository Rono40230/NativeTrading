use crate::BacktestResults;
use common::Candle;
use common::Result;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TradeDirection {
    Long,
    Short,
}

/// Type de sortie d'un trade — utilisé pour les stats de pyramidalisation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SortieType {
    Tp1,
    Tp2,
    Tp3,
    Sl,
    Expiration, // horizon expiré
}

pub(crate) struct TradeSimule {
    pub prix_entree: f64,
    pub prix_sortie: f64,
    pub direction: TradeDirection,
    /// Type de sortie pour les stats de pyramidalisation (None sur sortie simple)
    pub sortie: Option<SortieType>,
}

/// Sortie simple (Straddle) — retourne (prix, type_sortie).
pub(crate) fn simuler_sortie(
    bougies: &[Candle],
    direction: &TradeDirection,
    tp: f64,
    sl: f64,
    prix_defaut: f64,
) -> (f64, SortieType) {
    for b in bougies {
        match direction {
            TradeDirection::Long => {
                if b.low <= sl {
                    return (sl, SortieType::Sl);
                }
                if b.high >= tp {
                    return (tp, SortieType::Tp1);
                }
            }
            TradeDirection::Short => {
                if b.high >= sl {
                    return (sl, SortieType::Sl);
                }
                if b.low <= tp {
                    return (tp, SortieType::Tp1);
                }
            }
        }
    }
    // Expiration : sortie au close de la dernière bougie de l'horizon (pas de la bougie d'entrée)
    let close_final = bougies.last().map(|b| b.close).unwrap_or(prix_defaut);
    (close_final, SortieType::Expiration)
}

/// Simulation sortie pyramidale : ⅓ à TP1 (SL → BE), ⅓ à TP2, ⅓ à TP3.
/// Retourne (prix_sortie_pondéré, type_sortie_final).
/// Les TPs doivent être déjà direction-ajustés (Long : au-dessus, Short : en-dessous).
pub(crate) fn simuler_sortie_pyramidal(
    bougies: &[Candle],
    direction: &TradeDirection,
    prix_entree: f64,
    tp1: f64,
    tp2: f64,
    tp3: f64,
    sl_initial: f64,
) -> (f64, SortieType) {
    let mut lots_hit = 0u8;
    let mut sl_courant = sl_initial;
    let mut somme_prix = 0.0f64;

    for b in bougies {
        // SL d'abord (approche pessimiste)
        let sl_touche = match direction {
            TradeDirection::Long => b.low <= sl_courant,
            TradeDirection::Short => b.high >= sl_courant,
        };
        if sl_touche {
            let lots_restants = 3 - lots_hit;
            somme_prix += sl_courant * lots_restants as f64;
            return (somme_prix / 3.0, SortieType::Sl);
        }

        // TP1 — 1er tiers de position
        if lots_hit < 1 {
            let tp1_touche = match direction {
                TradeDirection::Long => b.high >= tp1,
                TradeDirection::Short => b.low <= tp1,
            };
            if tp1_touche {
                somme_prix += tp1;
                lots_hit = 1;
                sl_courant = prix_entree; // SL déplacé à BE
            }
        }

        // TP2 — 2e tiers (peut arriver même bougie que TP1)
        if lots_hit == 1 {
            let tp2_touche = match direction {
                TradeDirection::Long => b.high >= tp2,
                TradeDirection::Short => b.low <= tp2,
            };
            if tp2_touche {
                somme_prix += tp2;
                lots_hit = 2;
                sl_courant = tp1; // SL déplacé à TP1 — le dernier ⅓ est en profit garanti
            }
        }

        // TP3 — 3e tiers (peut arriver même bougie que TP2)
        if lots_hit == 2 {
            let tp3_touche = match direction {
                TradeDirection::Long => b.high >= tp3,
                TradeDirection::Short => b.low <= tp3,
            };
            if tp3_touche {
                somme_prix += tp3;
                return (somme_prix / 3.0, SortieType::Tp3);
            }
        }
    }

    // Horizon expiré — sortir les lots restants au dernier close
    let prix_defaut = bougies.last().map(|b| b.close).unwrap_or(prix_entree);
    let lots_restants = 3 - lots_hit;
    somme_prix += prix_defaut * lots_restants as f64;
    // Sortie partielle : rapport le niveau atteint
    let sortie_finale = match lots_hit {
        0 => SortieType::Expiration,
        1 => SortieType::Tp1,
        _ => SortieType::Tp2,
    };
    (somme_prix / 3.0, sortie_finale)
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
    let nb_tp3 = trades_avec_sortie
        .iter()
        .filter(|t| matches!(t.sortie, Some(SortieType::Tp3)))
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
        "Backtest: {} trades ({} straddles, win={:.1}%, gains={}, pertes={}) ROI={:.2}% Sharpe={:.2} MaxDD={:.1}% | TP1={} TP2={} TP3={} SL={} Exp={}",
        total, total / 2, win_rate, gagnants, total - gagnants,
        roi_pct, sharpe, max_drawdown_pct,
        nb_tp1, nb_tp2, nb_tp3, nb_sl, nb_exp
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
        nb_tp3,
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
