use chrono::{Datelike, Timelike};
use common::Candle;

// ── Backtest créneau Straddle ─────────────────────────────────────────────────
//
// Pour chaque occurrence du créneau (jour + heure), simule un straddle :
// - Long : TP = entrée + 2×ATR, SL = entrée - 0.5×ATR
// - Short : TP = entrée - 2×ATR, SL = entrée + 0.5×ATR
// - Win straddle = au moins une jambe atteint son TP avant son SL
// - Le gain net = +TP_MULT×risk - SL_MULT×risk (une jambe gagne, l'autre perd)
// - Si aucune jambe ne touche rien dans la fenêtre : les deux SL sont déclenchés

const TP_MULT: f64 = 2.0;
const SL_MULT: f64 = 0.5;
const RISK_PCT: f64 = 0.01; // 1% par direction

pub struct SlotBacktestResult {
    pub total_trades: usize,
    pub wins: usize,
    pub profit_factor: f64,
    pub win_rate: f64,
    pub max_drawdown_pct: f64,
}

/// Backteste un créneau (heure UTC + jour optionnel) sur l'historique H1.
/// `heure_debut` : heure UTC entière (0–23)
/// `jour_semaine` : 0=Lundi…4=Vendredi, None=tous les jours
pub fn backtest_slot(
    candles_h1: &[Candle],
    jour_semaine: Option<i64>,
    heure_debut: u32,
    capital: f64,
) -> SlotBacktestResult {
    let mut equity = capital;
    let mut peak = capital;
    let mut max_dd = 0.0f64;
    let mut gross_profits = 0.0f64;
    let mut gross_losses = 0.0f64;
    let mut total = 0usize;
    let mut wins = 0usize;

    for i in 14..candles_h1.len().saturating_sub(5) {
        let c = &candles_h1[i];
        if c.timestamp.hour() != heure_debut {
            continue;
        }
        if let Some(j) = jour_semaine {
            if c.timestamp.weekday().num_days_from_monday() as i64 != j {
                continue;
            }
        }

        let atr = calculer_atr(&candles_h1[i - 14..i]);
        if atr <= 0.0 {
            continue;
        }

        let entree = c.close;
        let futures = &candles_h1[i + 1..(i + 5).min(candles_h1.len())];

        let long_win =
            simuler_direction(futures, entree + TP_MULT * atr, entree - SL_MULT * atr, true);
        let short_win =
            simuler_direction(futures, entree - TP_MULT * atr, entree + SL_MULT * atr, false);

        // Gain net straddle : une jambe gagne TP, l'autre perd SL
        let position = equity * RISK_PCT;
        let pnl = if long_win || short_win {
            position * TP_MULT - position * SL_MULT
        } else {
            -(position * SL_MULT * 2.0)
        };

        equity += pnl;
        if pnl > 0.0 {
            wins += 1;
            gross_profits += pnl;
        } else {
            gross_losses += pnl.abs();
        }
        total += 1;

        if equity > peak {
            peak = equity;
        }
        let dd = (peak - equity) / peak.max(1.0) * 100.0;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    let win_rate = if total > 0 {
        wins as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let profit_factor = if gross_losses > 0.0 {
        gross_profits / gross_losses
    } else {
        gross_profits.max(0.0)
    };

    SlotBacktestResult {
        total_trades: total,
        wins,
        profit_factor,
        win_rate,
        max_drawdown_pct: max_dd,
    }
}

fn calculer_atr(candles: &[Candle]) -> f64 {
    if candles.len() < 2 {
        return 0.0;
    }
    let trs: Vec<f64> = candles
        .windows(2)
        .map(|w| {
            let hl = w[1].high - w[1].low;
            let hc = (w[1].high - w[0].close).abs();
            let lc = (w[1].low - w[0].close).abs();
            hl.max(hc).max(lc)
        })
        .collect();
    trs.iter().sum::<f64>() / trs.len() as f64
}

fn simuler_direction(futures: &[Candle], tp: f64, sl: f64, is_long: bool) -> bool {
    for c in futures {
        if is_long {
            if c.low <= sl {
                return false;
            }
            if c.high >= tp {
                return true;
            }
        } else {
            if c.high >= sl {
                return false;
            }
            if c.low <= tp {
                return true;
            }
        }
    }
    false
}
