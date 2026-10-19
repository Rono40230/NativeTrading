//! Fonctions libres de construction des niveaux du signal v11 — make_trade,
//! clamps SL (_slMin/_slMax) et liquidité la plus proche (DoL TP3). Scindées
//! de `signals.rs` (règle < 600 lignes).

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

/// Liquidité la plus proche au-delà de l'entrée (Pine 3460-3470 / 3607-3617).
/// Bull : EQH/PDH/PWH > entry → min. Bear : EQL/PDL/PWL < entry → max.
/// `inclut_asian_hl` : + _ahHighDrawn/_ahLowDrawn (v11 seulement, Pine `_tAHH3`).
/// Liquidité la plus LOINTAINE au-delà de l'entrée (réglage propriétaire TP3
/// « Liquidités ») — miroir de `nearest_liq` côté max au lieu de min.
pub(crate) fn farthest_liq(out: &SmcOutput, entry: f64, is_bull: bool, inclut_asian_hl: bool) -> Option<f64> {
    let cands: Vec<f64> = if is_bull {
        [
            out.liquidite.dernier_eqh_level,
            out.liquidite.pdh_active,
            out.liquidite.pwh_active,
        ]
        .into_iter()
        .flatten()
        .chain(if inclut_asian_hl { out.asian_hl.high } else { None })
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
        .chain(if inclut_asian_hl { out.asian_hl.low } else { None })
        .filter(|&v| v < entry)
        .collect()
    };
    if cands.is_empty() {
        return None;
    }
    Some(if is_bull {
        cands.into_iter().fold(f64::NEG_INFINITY, f64::max)
    } else {
        cands.into_iter().fold(f64::INFINITY, f64::min)
    })
}

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
