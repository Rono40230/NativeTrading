use chrono::{Datelike, NaiveDate, Timelike};
use common::Candle;
use std::collections::HashSet;

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
    #[allow(dead_code)]
    pub wins: usize,
    pub profit_factor: f64,
    pub win_rate: f64,
    pub max_drawdown_pct: f64,
    pub esperance_pct: f64,          // gain attendu par trade en % du capital
    pub payoff_ratio: f64,           // gain moyen / perte moyenne
    pub serie_pertes_max: usize,     // max pertes consécutives
    pub direction_dominante: String, // "Long", "Short", "Équilibré"
    pub amplitude_moyenne: f64,      // amplitude moyenne du créneau (high-low)
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
    let mut gains_vals: Vec<f64> = vec![];
    let mut pertes_vals: Vec<f64> = vec![];
    let mut consec = 0usize;
    let mut max_consec = 0usize;
    let mut long_wins_count = 0usize;
    let mut short_wins_count = 0usize;
    let mut amplitudes: Vec<f64> = vec![];

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
        amplitudes.push(c.high - c.low);

        let long_win = simuler_direction(
            futures,
            entree + TP_MULT * atr,
            entree - SL_MULT * atr,
            true,
        );
        let short_win = simuler_direction(
            futures,
            entree - TP_MULT * atr,
            entree + SL_MULT * atr,
            false,
        );

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
            gains_vals.push(pnl);
            consec = 0;
            if long_win {
                long_wins_count += 1;
            } else if short_win {
                short_wins_count += 1;
            }
        } else {
            gross_losses += pnl.abs();
            pertes_vals.push(pnl.abs());
            consec += 1;
            if consec > max_consec {
                max_consec = consec;
            }
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
    let gain_moyen = if !gains_vals.is_empty() {
        gains_vals.iter().sum::<f64>() / gains_vals.len() as f64
    } else {
        0.0
    };
    let perte_moyenne = if !pertes_vals.is_empty() {
        pertes_vals.iter().sum::<f64>() / pertes_vals.len() as f64
    } else {
        0.0
    };
    let wr = win_rate / 100.0;
    let esperance_pct = (wr * gain_moyen - (1.0 - wr) * perte_moyenne) / capital.max(1.0) * 100.0;
    let payoff_ratio = if perte_moyenne > 0.0 {
        gain_moyen / perte_moyenne
    } else {
        gain_moyen
    };
    let amplitude_moyenne = if !amplitudes.is_empty() {
        amplitudes.iter().sum::<f64>() / amplitudes.len() as f64
    } else {
        0.0
    };
    let direction_dominante = if long_wins_count > short_wins_count + 2 {
        "Long".to_string()
    } else if short_wins_count > long_wins_count + 2 {
        "Short".to_string()
    } else {
        "Équilibré".to_string()
    };

    SlotBacktestResult {
        total_trades: total,
        wins,
        profit_factor,
        win_rate,
        max_drawdown_pct: max_dd,
        esperance_pct,
        payoff_ratio,
        serie_pertes_max: max_consec,
        direction_dominante,
        amplitude_moyenne,
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

fn parse_timing(s: &str) -> Option<(u32, u32)> {
    let mut it = s.splitn(2, ':');
    let h: u32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    Some((h, m))
}

/// Backteste un timing précis avec fenêtre centrée `[timing - avant_min, timing + apres_min]`.
/// Utilise des bougies M5 — résolution bien supérieure au backtest H1 créneau large.
/// Valeurs recommandées : avant_min = 15, apres_min = 30 (fenêtre de 45 min).
pub fn backtest_slot_fenetre(
    candles_m5: &[Candle],
    jour_semaine: Option<i64>,
    timing: &str,
    avant_min: u32,
    apres_min: u32,
    capital: f64,
) -> SlotBacktestResult {
    let (th, tm) = match parse_timing(timing) {
        Some(v) => v,
        None => {
            return SlotBacktestResult {
                total_trades: 0,
                wins: 0,
                profit_factor: 0.0,
                win_rate: 0.0,
                max_drawdown_pct: 0.0,
                esperance_pct: 0.0,
                payoff_ratio: 0.0,
                serie_pertes_max: 0,
                direction_dominante: "—".into(),
                amplitude_moyenne: 0.0,
            }
        }
    };
    let ref_min = th * 60 + tm;
    let debut_min = ref_min.saturating_sub(avant_min);
    let fin_min = ref_min + apres_min;

    let mut equity = capital;
    let mut peak = capital;
    let mut max_dd = 0.0f64;
    let mut gross_profits = 0.0f64;
    let mut gross_losses = 0.0f64;
    let mut total = 0usize;
    let mut wins = 0usize;
    let mut gains_vals: Vec<f64> = vec![];
    let mut pertes_vals: Vec<f64> = vec![];
    let mut consec = 0usize;
    let mut max_consec = 0usize;
    let mut long_wins_count = 0usize;
    let mut short_wins_count = 0usize;
    let mut amplitudes: Vec<f64> = vec![];
    let mut dates_traitees: HashSet<NaiveDate> = HashSet::new();

    for i in 14..candles_m5.len() {
        let c = &candles_m5[i];
        let slot_min = c.timestamp.hour() * 60 + c.timestamp.minute();
        if slot_min < debut_min || slot_min >= fin_min {
            continue;
        }
        let date = c.timestamp.date_naive();
        if dates_traitees.contains(&date) {
            continue;
        }
        if let Some(j) = jour_semaine {
            if c.timestamp.weekday().num_days_from_monday() as i64 != j {
                continue;
            }
        }
        dates_traitees.insert(date);

        let atr = calculer_atr(&candles_m5[i.saturating_sub(14)..i]);
        if atr <= 0.0 {
            continue;
        }

        let entree = c.close;
        // Look-ahead : toutes les M5 dans la fenêtre du même jour
        let fin_idx = candles_m5[i + 1..]
            .iter()
            .position(|fc| {
                let d = fc.timestamp.date_naive();
                let m = fc.timestamp.hour() * 60 + fc.timestamp.minute();
                d != date || m >= fin_min
            })
            .map(|p| i + 1 + p)
            .unwrap_or(candles_m5.len());
        let futures = &candles_m5[i + 1..fin_idx];
        amplitudes.push(c.high - c.low);

        let long_win = simuler_direction(
            futures,
            entree + TP_MULT * atr,
            entree - SL_MULT * atr,
            true,
        );
        let short_win = simuler_direction(
            futures,
            entree - TP_MULT * atr,
            entree + SL_MULT * atr,
            false,
        );

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
            gains_vals.push(pnl);
            consec = 0;
            if long_win {
                long_wins_count += 1;
            } else if short_win {
                short_wins_count += 1;
            }
        } else {
            gross_losses += pnl.abs();
            pertes_vals.push(pnl.abs());
            consec += 1;
            if consec > max_consec {
                max_consec = consec;
            }
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
    let gain_moyen = if !gains_vals.is_empty() {
        gains_vals.iter().sum::<f64>() / gains_vals.len() as f64
    } else {
        0.0
    };
    let perte_moyenne = if !pertes_vals.is_empty() {
        pertes_vals.iter().sum::<f64>() / pertes_vals.len() as f64
    } else {
        0.0
    };
    let wr = win_rate / 100.0;
    let esperance_pct = (wr * gain_moyen - (1.0 - wr) * perte_moyenne) / capital.max(1.0) * 100.0;
    let payoff_ratio = if perte_moyenne > 0.0 {
        gain_moyen / perte_moyenne
    } else {
        gain_moyen
    };
    let amplitude_moyenne = if !amplitudes.is_empty() {
        amplitudes.iter().sum::<f64>() / amplitudes.len() as f64
    } else {
        0.0
    };
    let direction_dominante = if long_wins_count > short_wins_count + 2 {
        "Long".to_string()
    } else if short_wins_count > long_wins_count + 2 {
        "Short".to_string()
    } else {
        "Équilibré".to_string()
    };
    SlotBacktestResult {
        total_trades: total,
        wins,
        profit_factor,
        win_rate,
        max_drawdown_pct: max_dd,
        esperance_pct,
        payoff_ratio,
        serie_pertes_max: max_consec,
        direction_dominante,
        amplitude_moyenne,
    }
}
