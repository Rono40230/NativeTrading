//! Fonctions libres de construction des niveaux du signal v11 — make_trade,
//! clamps SL (_slMin/_slMax) et liquidité la plus proche (DoL TP3). Scindées
//! de `signals.rs` (règle < 600 lignes).

use super::calibration::AssetCalibration;
use super::trade::{Side, Trade, TradeSource};
use super::types::{BarInput, SmcOutput};

/// Construit un `Trade` (factoring du dispatch buy/sell).
#[allow(clippy::too_many_arguments)]
pub(crate) fn make_trade(
    id: u64,
    source: TradeSource,
    is_bull: bool,
    entry: f64,
    sl: f64,
    tp1: f64,
    tp2: f64,
    tp3: f64,
    score: i32,
    risk0: f64,
    bar: &BarInput,
    bar_index: usize,
    ob_key: Option<usize>,
) -> Trade {
    let side = if is_bull { Side::Buy } else { Side::Sell };
    let mut t = Trade::new_buy(
        id, source, entry, sl, tp1, tp2, tp3, score, risk0, bar, bar_index, ob_key,
    );
    t.side = side;
    t
}

/// `_slMin` / `_slMax` (Pine 2424-2435) — en × ATR14 (atr toujours présent après warmup).
pub(crate) fn sl_min_max(cal: &AssetCalibration, atr: f64) -> (f64, f64) {
    let sl_min = if cal.is_xau {
        0.5 * atr
    } else if cal.is_xag {
        0.6 * atr
    } else if cal.is_nas || cal.is_spx {
        0.5 * atr
    } else if cal.is_btc {
        0.8 * atr
    } else if cal.is_dax {
        0.5 * atr
    } else {
        0.0
    };
    let sl_max = if cal.is_xau {
        1.5 * atr
    } else if cal.is_xag {
        1.8 * atr
    } else if cal.is_nas || cal.is_spx {
        1.5 * atr
    } else if cal.is_btc {
        2.5 * atr
    } else if cal.is_dax {
        1.5 * atr
    } else {
        1e10
    };
    (sl_min, sl_max)
}

/// Liquidité la plus proche au-delà de l'entrée (Pine 3460-3470 / 3607-3617).
/// Bull : EQH/PDH/PWH > entry → min. Bear : EQL/PDL/PWL < entry → max.
/// `inclut_asian_hl` : + _ahHighDrawn/_ahLowDrawn (v11 seulement, Pine `_tAHH3`).
pub(crate) fn nearest_liq(out: &SmcOutput, entry: f64, is_bull: bool, inclut_asian_hl: bool) -> Option<f64> {
    let cands: Vec<f64> = if is_bull {
        [
            out.liquidite.dernier_eqh_level,
            out.liquidite.pdh_active,
            out.liquidite.pwh_active,
        ]
        .into_iter()
        .flatten()
        .chain(if inclut_asian_hl {
            out.asian_hl.high
        } else {
            None
        })
        .filter(|&v| v > entry)
        .collect()
    } else {
        [
            out.liquidite.dernier_eql_level,
            out.liquidite.pdl_active,
            out.liquidite.pwl_active,
        ]
        .into_iter()
        .flatten()
        .chain(if inclut_asian_hl {
            out.asian_hl.low
        } else {
            None
        })
        .filter(|&v| v < entry)
        .collect()
    };
    if cands.is_empty() {
        return None;
    }
    Some(if is_bull {
        cands.into_iter().fold(f64::INFINITY, f64::min)
    } else {
        cands.into_iter().fold(f64::NEG_INFINITY, f64::max)
    })
}
