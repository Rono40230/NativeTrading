//! Helpers purs du moteur BSZones (Pine lignes 3046, 3063-3148, 3227-3234).
//!
//! Fonctions de scoring (socle /16 + dyn /11 + conversion /10) et helpers
//! d'historique rolling (`[1]..[20]`). Sans état, sans panic.

use super::types::{BarInput, FvgZone, SmcOutput};

/// `i_dispMult` (Pine ligne 3060) — displacement min en × ATR (15/10 = 1.5).
pub(super) const DISP_MULT: f64 = 1.5;
/// `i_minScore` (Pine ligne 3058) — score min affichage (informatif).
pub const MIN_SCORE: i32 = 5;

/// Fenêtre sweep fresh BSZones : `max(2, round(9000/tf_sec))` (Pine 3046).
pub(super) fn sweep_fresh_bars(tf_sec: i64) -> i64 {
    if tf_sec <= 0 {
        10
    } else {
        ((9000.0 / tf_sec as f64).round() as i64).max(2)
    }
}

// ============================================================================
// Fonctions de scoring BSZONES (Pine lignes 3098-3148) — portées, pures.
// ============================================================================

/// `_dispScore` (Pine 3098-3102).
pub(super) fn disp_score(body: f64, atr: f64, body3: f64) -> i32 {
    let r1 = if atr > 0.0 { body / atr } else { 0.0 };
    let r3 = if atr > 0.0 { body3 / atr } else { 0.0 };
    let ratio = r1.max(r3);
    if ratio >= 3.5 {
        3
    } else if ratio >= 2.5 {
        2
    } else {
        1
    }
}

/// `_bodyRangeScore` (Pine 3104).
pub(super) fn body_range_score(br: f64) -> i32 {
    if br >= 0.7 {
        1
    } else {
        0
    }
}

/// `_volScore` (Pine 3106-3107).
pub(super) fn vol_score(vol: f64, sma: f64) -> i32 {
    if sma > 0.0 {
        if vol >= 2.0 * sma {
            2
        } else if vol >= 1.5 * sma {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// `_sweepScore` (Pine 3109-3111).
pub(super) fn sweep_score(sweep_bar: Option<usize>, bar_index: usize) -> i32 {
    match sweep_bar {
        Some(b) => {
            let ago = bar_index as i64 - b as i64;
            if ago <= 2 {
                2
            } else if ago <= 5 {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

/// `_mitScore` (Pine 3113-3114).
pub(super) fn mit_score(st: u8) -> i32 {
    match st {
        0 => 3,
        1 => 1,
        _ => 0,
    }
}

/// `_proxScore` (Pine 3116-3119) — proximity = distance à la zone (top/bot).
pub(super) fn prox_score(price: f64, top: f64, bot: f64, atr: f64) -> i32 {
    let dist = (price - top).abs().min((price - bot).abs());
    let ratio = if atr > 0.0 { dist / atr } else { 999.0 };
    if ratio <= 1.0 {
        2
    } else if ratio <= 3.0 {
        1
    } else {
        0
    }
}

/// `_fvgScore` (Pine 3121-3122).
pub(super) fn fvg_score(has_fvg: bool) -> i32 {
    if has_fvg {
        1
    } else {
        0
    }
}

/// `_bsPdScore` (Pine 3124-3130).
pub(super) fn bs_pd_score(is_bull: bool, close: f64, eq: Option<f64>) -> i32 {
    match eq {
        Some(e) => {
            if is_bull {
                if close < e {
                    1
                } else {
                    0
                }
            } else if close > e {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

/// `_bsKzScore` (Pine 3132-3133) — dead-zone force KZ à 0.
pub(super) fn bs_kz_score(in_kz: bool, in_dead_zone: bool) -> i32 {
    if in_dead_zone {
        0
    } else if in_kz {
        1
    } else {
        0
    }
}

/// `_bosScore` (Pine 3135-3136) — BOS dans les 3 dernières barres.
pub(super) fn bos_score(bos_bar: Option<usize>, bar_index: usize) -> i32 {
    match bos_bar {
        Some(b) if bar_index as i64 - b as i64 <= 3 => 3,
        _ => 0,
    }
}

/// `_chochScore` (Pine 3138-3139) — CHOCH dans les 5 dernières barres.
pub(super) fn choch_score(choch_bar: Option<usize>, bar_index: usize) -> i32 {
    match choch_bar {
        Some(b) if bar_index as i64 - b as i64 <= 5 => 2,
        _ => 0,
    }
}

/// `_bsHtfScore` (Pine 3141-3143).
pub(super) fn bs_htf_score(is_bull: bool, h1: i32, h4: i32) -> i32 {
    let d = if is_bull { 1 } else { -1 };
    if h1 == d && h4 == d {
        2
    } else if h1 == d || h4 == d {
        1
    } else {
        0
    }
}

/// `_oteScore` (Pine 3145-3146).
pub(super) fn ote_score(in_ote: bool) -> i32 {
    if in_ote {
        1
    } else {
        0
    }
}

/// `_coeurScore` (Pine 3148) — zone-cœur = FVG ∩ OTE.
pub(super) fn coeur_score(has_fvg: bool, in_ote: bool) -> i32 {
    if has_fvg && in_ote {
        3
    } else {
        0
    }
}

/// `_toForce10` (Pine 3150-3152) — conversion raw → /10 (borné [0,10]).
pub(super) fn to_force10(raw: i32) -> i32 {
    let sc = (raw as f64 * 10.0 / 27.0).round() as i32;
    sc.clamp(0, 10)
}

/// Bundle du score dynamique /11 (Pine 3246 / 3350).
/// `in_ote` = OTE courant (naissance) ou figé (lifecycle). `top/bot` = zone (prox).
#[allow(clippy::too_many_arguments)]
pub(super) fn dyn_score(
    is_bull: bool,
    close: f64,
    atr: f64,
    top: f64,
    bot: f64,
    has_fvg: bool,
    in_ote: bool,
    out: &SmcOutput,
    in_dead_zone: bool,
) -> i32 {
    prox_score(close, top, bot, atr)
        + fvg_score(has_fvg)
        + bs_pd_score(is_bull, close, out.premium_discount.equilibrium)
        + bs_kz_score(out.kill_zone.in_kz, in_dead_zone)
        + bs_htf_score(is_bull, out.mtf.h1.trend, out.mtf.h4.trend)
        + ote_score(in_ote)
        + coeur_score(has_fvg, in_ote)
}

// ============================================================================
// Helpers historique / contexte
// ============================================================================

/// Corps de la bougie[k] (k≥1) SI la bougie courante ET la bougie[k] sont dans
/// le sens `is_bull` (Pine 3232-3233 / 3277-3278). Sinon 0.0. Sans panic.
pub(super) fn body_delta(history: &[BarInput], k: usize, bar: &BarInput, is_bull: bool) -> f64 {
    let n = history.len();
    if n <= k {
        return 0.0;
    }
    let p = &history[n - 1 - k];
    if is_bull {
        if bar.close > bar.open && p.close > p.open {
            p.close - p.open
        } else {
            0.0
        }
    } else if bar.close < bar.open && p.close < p.open {
        p.open - p.close
    } else {
        0.0
    }
}

/// `close[k]` Pine (k≥1) depuis l'historique rolling (dernier = bar courante).
pub(super) fn bar_close_ago(history: &[BarInput], k: usize) -> Option<f64> {
    let n = history.len();
    (n > k).then(|| history[n - 1 - k].close)
}
pub(super) fn bar_high_ago(history: &[BarInput], k: usize) -> Option<f64> {
    let n = history.len();
    (n > k).then(|| history[n - 1 - k].high)
}
pub(super) fn bar_low_ago(history: &[BarInput], k: usize) -> Option<f64> {
    let n = history.len();
    (n > k).then(|| history[n - 1 - k].low)
}
pub(super) fn bar_volume_ago(history: &[BarInput], k: usize) -> f64 {
    let n = history.len();
    if n > k {
        history[n - 1 - k].volume
    } else {
        0.0
    }
}

/// `volSma20 = sma(volume, 20)[1]` (Pine 3063) — moyenne des volumes jusqu'à bar[1].
pub(super) fn vol_sma_20(history: &[BarInput]) -> f64 {
    let n = history.len();
    if n < 2 {
        return 0.0;
    }
    // bar[1] = history[n-2] ; on remonte 20 barres : indices [n-21 .. n-2] inclus.
    let start = n.saturating_sub(21);
    let end = n - 1; // exclus (jusqu'à bar[1] = n-2 inclus)
    let count = end.saturating_sub(start);
    if count == 0 {
        return 0.0;
    }
    let sum: f64 = history[start..end].iter().map(|b| b.volume).sum();
    sum / count as f64
}

/// `_obBodyRange1` (Pine 3064) — body/range de la bougie[1].
pub(super) fn body_range_ago1(history: &[BarInput]) -> f64 {
    let n = history.len();
    if n < 2 {
        return 0.0;
    }
    let p = &history[n - 2];
    let range = p.high - p.low;
    if range > 0.0 {
        (p.close - p.open).abs() / range
    } else {
        0.0
    }
}

pub(super) fn in_dead_zone_safe(bar: &BarInput) -> bool {
    let mins = bar.timestamp.rem_euclid(86400) / 60;
    (960..1080).contains(&mins)
}

pub(super) fn sweep_recent(sweep_bar: Option<usize>, bar_index: usize, fresh: i64) -> bool {
    match sweep_bar {
        Some(b) => (bar_index as i64 - b as i64) <= fresh,
        None => false,
    }
}

/// Chevauchement FVG/OB (Pine `f_znHasFVG` 2990-2996).
pub(super) fn zn_has_fvg(fvg: &[FvgZone], ob_top: f64, ob_bot: f64) -> bool {
    fvg.iter().any(|f| f.top > ob_bot && f.bot < ob_top)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_force10_bornes() {
        assert_eq!(to_force10(0), 0);
        assert_eq!(to_force10(27), 10);
        // raw=14 → round(140/27=5.18) = 5
        assert_eq!(to_force10(14), 5);
        // raw=20 → round(200/27=7.4) = 7
        assert_eq!(to_force10(20), 7);
        assert_eq!(to_force10(-5), 0, "négatif clampé à 0");
        assert_eq!(to_force10(100), 10, ">27 clampé à 10");
    }

    #[test]
    fn disp_score_par_bande() {
        assert_eq!(disp_score(3.6, 1.0, 0.0), 3, "ratio>=3.5 → 3");
        assert_eq!(disp_score(2.5, 1.0, 0.0), 2, "ratio>=2.5 → 2");
        assert_eq!(disp_score(1.0, 1.0, 0.0), 1, "sinon → 1");
        // body3 l'emporte si > body.
        assert_eq!(disp_score(1.0, 1.0, 3.6), 3);
    }

    #[test]
    fn vol_score_bandes() {
        assert_eq!(vol_score(200.0, 100.0), 2, "2× sma → 2");
        assert_eq!(vol_score(150.0, 100.0), 1, "1.5× sma → 1");
        assert_eq!(vol_score(100.0, 100.0), 0, "== sma → 0");
        assert_eq!(vol_score(100.0, 0.0), 0, "sma=0 → 0");
    }

    #[test]
    fn prox_score_bandes() {
        assert_eq!(
            prox_score(100.0, 101.0, 99.0, 1.0),
            2,
            "dist 1, ratio<=1 → 2"
        );
        // dist = min(|100-103|, |100-97|) = 3 → ratio 3 <= 3 → 1.
        assert_eq!(
            prox_score(100.0, 103.0, 97.0, 1.0),
            1,
            "dist 3, ratio<=3 → 1"
        );
        assert_eq!(
            prox_score(100.0, 110.0, 90.0, 1.0),
            0,
            "dist 10, ratio>3 → 0"
        );
    }

    #[test]
    fn mit_score_etats() {
        assert_eq!(mit_score(0), 3);
        assert_eq!(mit_score(1), 1);
        assert_eq!(mit_score(2), 0);
    }

    #[test]
    fn bos_choch_score_fenetres() {
        let bi = 100usize;
        assert_eq!(bos_score(Some(98), bi), 3, "2 bars avant → 3");
        assert_eq!(bos_score(Some(96), bi), 0, "4 bars avant → 0");
        assert_eq!(choch_score(Some(95), bi), 2, "5 bars avant → 2");
        assert_eq!(choch_score(Some(94), bi), 0, "6 bars avant → 0");
    }

    #[test]
    fn dead_zone_force_kz_zero() {
        assert_eq!(bs_kz_score(true, true), 0, "dead-zone annule KZ");
        assert_eq!(bs_kz_score(true, false), 1, "KZ hors dead-zone");
        assert_eq!(bs_kz_score(false, false), 0);
    }

    #[test]
    fn body_delta_bull_exige_memes_sens() {
        // bar courante bull, bar[1] bull → corps bar[1].
        let cur = BarInput {
            timestamp: 3,
            open: 10.0,
            high: 14.0,
            low: 9.0,
            close: 13.0,
            volume: 0.0,
        };
        let b1 = BarInput {
            timestamp: 2,
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.5,
            volume: 0.0,
        };
        let b0 = BarInput {
            timestamp: 1,
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.0,
            volume: 0.0,
        };
        let hist = vec![b0, b1, cur];
        assert!((body_delta(&hist, 1, &cur, true) - 1.5).abs() < 1e-9);
        // bar courante bear → 0 même si bar[1] bull.
        let cur_bear = BarInput {
            timestamp: 3,
            open: 13.0,
            high: 14.0,
            low: 9.0,
            close: 10.0,
            volume: 0.0,
        };
        assert_eq!(body_delta(&hist, 1, &cur_bear, true), 0.0);
    }
}
