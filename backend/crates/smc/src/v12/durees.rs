//! Fonctions libres du moteur v12 — durées max trade/TP3 par TF et masque
//! BOS/MSS. Scindées de `mod.rs` (règle < 600 lignes).

use super::calibration::AssetCalibration;
use super::types::{BosEvent, MssEvent};

/// `_autoTradeMaxMins` (Pine 2374) — durée max trade en minutes selon le TF.
pub fn trade_max_mins(tf_mins: u32) -> i64 {
    match tf_mins {
        60 => 480,    // H1
        240 => 1920,  // H4
        1440 => 5760, // D1
        _ => 240,     // défaut (M1–M30)
    }
}

/// `_autoTp3Mins` (Pine 71-76) — durée max TP3 en minutes selon asset × TF.
pub fn tp3_max_mins(cal: &AssetCalibration, tf_mins: u32) -> i64 {
    let m15 = tf_mins == 15;
    let h1 = tf_mins == 60;
    if cal.is_xau {
        if m15 {
            60
        } else if h1 {
            240
        } else {
            60
        }
    } else if cal.is_xag {
        if m15 {
            45
        } else if h1 {
            180
        } else {
            60
        }
    } else if cal.is_nas || cal.is_spx {
        if m15 {
            30
        } else if h1 {
            120
        } else {
            60
        }
    } else if cal.is_btc {
        if m15 {
            90
        } else if h1 {
            360
        } else {
            60
        }
    } else if cal.is_dax {
        if m15 {
            30
        } else if h1 {
            120
        } else {
            60
        }
    } else {
        60
    }
}

/// Masque le BOS selon `bosHaussier and not mssHaussier` (Pine lignes 524-527, 540).
///
/// Renvoie un `BosEvent` dont les flags directionnels sont annulés lorsqu'un MSS
/// s'est produit sur la même bar. Le level/bar_index sont conservés si le flag reste.
pub fn mask_bos_by_mss(bos: &BosEvent, mss: &MssEvent) -> BosEvent {
    let bullish = bos.bullish && !mss.mss_haussier;
    let bearish = bos.bearish && !mss.mss_baissier;
    BosEvent {
        bullish,
        bearish,
        level: if bullish || bearish { bos.level } else { None },
        bar_index: if bullish || bearish {
            bos.bar_index
        } else {
            None
        },
    }
}
