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
    Sl,
    Expiration, // fin de bougies disponibles
}

pub(crate) struct TradeSimule {
    pub prix_entree: f64,
    pub prix_sortie: f64,
    pub direction: TradeDirection,
    /// Type de sortie pour les stats de pyramidalisation (None sur sortie simple)
    pub sortie: Option<SortieType>,
}

/// Options de gestion du trade pour la sortie simple (Straddle).
pub(crate) struct OptionsGestion {
    pub prix_entree: f64,
    /// Trailing stop : SL remonte à peak - atr_val * mult (None = désactivé).
    pub trailing_atr: Option<(f64, f64)>,
    /// Break-even : quand gain > atr_val * mult, SL → prix_entree (None = désactivé).
    pub be_atr: Option<(f64, f64)>,
}

/// Sortie simple (Straddle) avec trailing stop et break-even optionnels.
pub(crate) fn simuler_sortie(
    bougies: &[Candle],
    direction: &TradeDirection,
    tp: f64,
    sl: f64,
    prix_defaut: f64,
    opts: OptionsGestion,
) -> (f64, SortieType) {
    let prix_entree = opts.prix_entree;
    let trailing_atr = opts.trailing_atr;
    let be_atr = opts.be_atr;
    let mut sl_courant = sl;
    let mut peak = match direction {
        TradeDirection::Long => sl,  // peak démarre bas (favorable = montée)
        TradeDirection::Short => sl, // symétrique
    };

    for b in bougies {
        // Mettre à jour le peak et SL trailing
        if let Some((atr_val, mult)) = trailing_atr {
            match direction {
                TradeDirection::Long => {
                    if b.high > peak {
                        peak = b.high;
                        let new_sl = peak - atr_val * mult;
                        if new_sl > sl_courant {
                            sl_courant = new_sl;
                        }
                    }
                }
                TradeDirection::Short => {
                    if b.low < peak {
                        peak = b.low;
                        let new_sl = peak + atr_val * mult;
                        if new_sl < sl_courant {
                            sl_courant = new_sl;
                        }
                    }
                }
            }
        }

        // Break-even : SL remonte au prix d'entrée dès que le gain > atr * mult
        if let Some((atr_val, mult)) = be_atr {
            let seuil = atr_val * mult;
            match direction {
                TradeDirection::Long => {
                    if b.high >= prix_entree + seuil {
                        sl_courant = sl_courant.max(prix_entree);
                    }
                }
                TradeDirection::Short => {
                    if b.low <= prix_entree - seuil {
                        sl_courant = sl_courant.min(prix_entree);
                    }
                }
            }
        }

        match direction {
            TradeDirection::Long => {
                if b.low <= sl_courant {
                    return (sl_courant, SortieType::Sl);
                }
                if b.high >= tp {
                    return (tp, SortieType::Tp1);
                }
            }
            TradeDirection::Short => {
                if b.high >= sl_courant {
                    return (sl_courant, SortieType::Sl);
                }
                if b.low <= tp {
                    return (tp, SortieType::Tp1);
                }
            }
        }
    }
    // Expiration : sortie au close de la dernière bougie de l'horizon
    let close_final = bougies.last().map(|b| b.close).unwrap_or(prix_defaut);
    (close_final, SortieType::Expiration)
}

/// Paramètres pour la simulation pyramidale (SMC + Straddle avec TP2/TP3).
pub(crate) struct ParamsPyramidal {
    pub prix_entree: f64,
    pub tp1: f64,
    pub tp2: f64,
    /// Trailing stop actif après TP2 : (atr_val, mult). Remplace le TP3 fixe.
    pub trailing_tp3: (f64, f64),
    pub sl_initial: f64,
    /// true = vente partielle (⅓ encaissé à TP1, ⅓ à TP2, ⅓ au trailing).
    /// false = lot entier sorti au trailing (SL déplacé uniquement à TP1, puis TP2).
    pub vente_partielle: bool,
}

/// Simulation sortie pyramidale : TP1 (SL→BE), TP2 (SL→TP1), puis trailing stop.
/// `vente_partielle` contrôle si ⅓ est encaissé à chaque TP ou si tout sort ensemble.
pub(crate) fn simuler_sortie_pyramidal(
    bougies: &[Candle],
    direction: &TradeDirection,
    params: ParamsPyramidal,
) -> (f64, SortieType) {
    let ParamsPyramidal {
        prix_entree,
        tp1,
        tp2,
        trailing_tp3: (atr_val, mult),
        sl_initial,
        vente_partielle,
    } = params;
    let mut lots_hit = 0u8; // 0 = aucun TP, 1 = TP1, 2 = TP2
    let mut sl_courant = sl_initial;
    let mut somme_prix = 0.0f64;
    let mut trailing_actif = false;
    let mut peak = prix_entree;

    for b in bougies {
        // 1. Mise à jour trailing après TP2
        if trailing_actif {
            match direction {
                TradeDirection::Long => {
                    if b.high > peak {
                        peak = b.high;
                        let new_sl = peak - atr_val * mult;
                        if new_sl > sl_courant {
                            sl_courant = new_sl;
                        }
                    }
                }
                TradeDirection::Short => {
                    if b.low < peak {
                        peak = b.low;
                        let new_sl = peak + atr_val * mult;
                        if new_sl < sl_courant {
                            sl_courant = new_sl;
                        }
                    }
                }
            }
        }

        // 2. Vérification SL
        let sl_touche = match direction {
            TradeDirection::Long => b.low <= sl_courant,
            TradeDirection::Short => b.high >= sl_courant,
        };
        if sl_touche {
            if vente_partielle {
                let lots_restants = 3 - lots_hit;
                somme_prix += sl_courant * lots_restants as f64;
                return (somme_prix / 3.0, SortieType::Sl);
            } else {
                return (sl_courant, SortieType::Sl);
            }
        }

        // 3. TP1 — premier tiers (ou BE sans vente)
        if lots_hit < 1 {
            let tp1_touche = match direction {
                TradeDirection::Long => b.high >= tp1,
                TradeDirection::Short => b.low <= tp1,
            };
            if tp1_touche {
                if vente_partielle {
                    somme_prix += tp1;
                }
                lots_hit = 1;
                sl_courant = prix_entree; // SL → BE
            }
        }

        // 4. TP2 — active le trailing (peut arriver même bougie que TP1)
        if lots_hit == 1 {
            let tp2_touche = match direction {
                TradeDirection::Long => b.high >= tp2,
                TradeDirection::Short => b.low <= tp2,
            };
            if tp2_touche {
                if vente_partielle {
                    somme_prix += tp2;
                }
                lots_hit = 2;
                sl_courant = tp1; // SL → TP1
                trailing_actif = true;
                // Peak initialisé au plus haut/bas atteint sur cette bougie
                peak = match direction {
                    TradeDirection::Long => b.high,
                    TradeDirection::Short => b.low,
                };
                // Calculer sl_courant initial depuis ce peak (ne descend pas sous tp1)
                let new_sl = match direction {
                    TradeDirection::Long => peak - atr_val * mult,
                    TradeDirection::Short => peak + atr_val * mult,
                };
                match direction {
                    TradeDirection::Long => {
                        if new_sl > sl_courant {
                            sl_courant = new_sl;
                        }
                    }
                    TradeDirection::Short => {
                        if new_sl < sl_courant {
                            sl_courant = new_sl;
                        }
                    }
                }
            }
        }
    }

    // Fin des bougies disponibles — sortie forcée au dernier close
    let prix_final = bougies.last().map(|b| b.close).unwrap_or(prix_entree);
    let sortie = match lots_hit {
        0 => SortieType::Expiration,
        1 => SortieType::Tp1,
        _ => SortieType::Tp2,
    };
    if vente_partielle {
        let lots_restants = 3 - lots_hit;
        somme_prix += prix_final * lots_restants as f64;
        (somme_prix / 3.0, sortie)
    } else {
        (prix_final, sortie)
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
