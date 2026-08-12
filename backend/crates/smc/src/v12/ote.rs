//! MODULE 13c — Fibonacci OTE (Optimal Trade Entry).
//!
//! Reproduit MODULE 13c Pine (lignes 2022-2110) :
//!
//! - Capture de la plage OTE au BOS :
//!   `if bosHaussier ... : _fibSHL=sl1, _fibSHH=sh1, _fibBullBar=bar_index`
//!   `if bosBaissier ... : _fibSBH=sh1, _fibSBL=sl1, _fibBearBar=bar_index`
//! - Expiration (Phase 5.2) : la plage expire `OTE_EXPIRY_BARS` bars après le BOS, avec
//!   `OTE_EXPIRY_BARS = max(1, round(10800/_tfSec))` (12 en M15).
//! - Zone OTE = Fibonacci 61.8 % - 78.6 % du range :
//!   Bull : `_oteTopBull = _fibSHH - rng*0.618 ; _oteBotBull = _fibSHH - rng*0.786`
//!   Bear : `_oteBotBear = _fibSBL + rng*0.618 ; _oteTopBear = _fibSBL + rng*0.786`
//! - `inOTE_bull = close ∈ [_oteBotBull, _oteTopBull]`
//! - `inOTE_bear = close ∈ [_oteBotBear, _oteTopBear]`
//!
//! La capture se fait sur le BOS **BRUT** (Pine `bosHaussier`/`bosBaissier` — jamais
//! réaffectés par le masque MSS). Le moteur passe donc le BOS brut.
//!
//! Ordre Pine (lignes 2040-2077) : capture au BOS → expiration → calcul des bornes.

use super::types::{BarInput, OteEvent};

/// `FIB_OTE_HIGH` (Pine ligne 2063) = 0.618.
pub const FIB_OTE_HIGH: f64 = 0.618;
/// `FIB_OTE_LOW` (Pine ligne 2064) = 0.786.
pub const FIB_OTE_LOW: f64 = 0.786;

/// Détecteur OTE : capture la plage au BOS, gère l'expiration, calcule la zone Fib.
pub struct OteDetector {
    /// `_fibSHL` (Pine) = sl1 au dernier BOS haussier (borne basse du range bull).
    fib_shl: Option<f64>,
    /// `_fibSHH` (Pine) = sh1 au dernier BOS haussier (borne haute du range bull).
    fib_shh: Option<f64>,
    /// `_fibSBH` (Pine) = sh1 au dernier BOS baissier.
    fib_sbh: Option<f64>,
    /// `_fibSBL` (Pine) = sl1 au dernier BOS baissier.
    fib_sbl: Option<f64>,
    /// `_fibBullBar` (Pine) = bar_index du BOS ayant figé la plage OTE bull.
    fib_bull_bar: Option<usize>,
    fib_bear_bar: Option<usize>,
    /// `OTE_EXPIRY_BARS` (Pine).
    expiry_bars: i64,
    bar_count: usize,
    last_event: OteEvent,
}

impl OteDetector {
    /// `tf_sec` : timeframe en secondes (Pine `_tfSec`).
    pub fn new(tf_sec: i64) -> Self {
        // `OTE_EXPIRY_BARS = max(1, round(10800/_tfSec))`. Si tf_sec <= 0 → 12 (Pine).
        let expiry_bars = if tf_sec <= 0 {
            12
        } else {
            let raw = (10800.0 / tf_sec as f64).round();
            // Borné à ≥ 1 ; raw ne peut pas être négatif (tf_sec > 0).
            let v = raw.max(1.0) as i64;
            v.max(1)
        };
        Self {
            fib_shl: None,
            fib_shh: None,
            fib_sbh: None,
            fib_sbl: None,
            fib_bull_bar: None,
            fib_bear_bar: None,
            expiry_bars,
            bar_count: 0,
            last_event: OteEvent::default(),
        }
    }

    pub fn expiry_bars(&self) -> i64 {
        self.expiry_bars
    }

    /// Traite une bar.
    ///
    /// - `bos_bull` / `bos_bear` : BOS **brut** (Pine `bosHaussier`/`bosBaissier`).
    /// - `sh1` / `sl1` : derniers swings.
    pub fn update(
        &mut self,
        bar: &BarInput,
        bos_bull: bool,
        bos_bear: bool,
        sh1: Option<f64>,
        sl1: Option<f64>,
    ) -> OteEvent {
        let cur = self.bar_count;
        self.bar_count += 1;

        // --- Capture au BOS (Pine lignes 2040-2047) ---
        if bos_bull {
            if let (Some(_), Some(h)) = (sl1, sh1) {
                self.fib_shl = sl1;
                self.fib_shh = Some(h);
                self.fib_bull_bar = Some(cur);
            }
        }
        if bos_bear {
            if let (Some(h), Some(_)) = (sh1, sl1) {
                self.fib_sbh = Some(h);
                self.fib_sbl = sl1;
                self.fib_bear_bar = Some(cur);
            }
        }

        // --- Expiration temporelle (Pine lignes 2050-2055) ---
        // Remarque : on capture PUIS on expire — sur la bar de BOS, l'écart vaut 0.
        if let Some(b) = self.fib_bull_bar {
            if (cur as i64 - b as i64) > self.expiry_bars {
                self.fib_shl = None;
                self.fib_shh = None;
            }
        }
        if let Some(b) = self.fib_bear_bar {
            if (cur as i64 - b as i64) > self.expiry_bars {
                self.fib_sbh = None;
                self.fib_sbl = None;
            }
        }

        // --- Calcul des bornes OTE (Pine lignes 2066-2074) ---
        let bull_top_bot = match (self.fib_shl, self.fib_shh) {
            (Some(l), Some(h)) if h > l => {
                let rng = h - l;
                let top = h - rng * FIB_OTE_HIGH;
                let bot = h - rng * FIB_OTE_LOW;
                Some((top, bot))
            }
            _ => None,
        };
        let bear_top_bot = match (self.fib_sbh, self.fib_sbl) {
            (Some(h), Some(l)) if h > l => {
                let rng = h - l;
                let bot = l + rng * FIB_OTE_HIGH;
                let top = l + rng * FIB_OTE_LOW;
                Some((top, bot))
            }
            _ => None,
        };

        // --- inOTE (Pine lignes 2076-2077) ---
        let close = bar.close;
        let in_ote_bull = match bull_top_bot {
            Some((top, bot)) => close <= top && close >= bot,
            None => false,
        };
        let in_ote_bear = match bear_top_bot {
            Some((top, bot)) => close >= bot && close <= top,
            None => false,
        };

        let ev = OteEvent {
            in_ote_bull,
            in_ote_bear,
            bull_top: bull_top_bot.map(|(t, _)| t),
            bull_bot: bull_top_bot.map(|(_, b)| b),
            bear_top: bear_top_bot.map(|(t, _)| t),
            bear_bot: bear_top_bot.map(|(_, b)| b),
            expiry_bars: self.expiry_bars,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> OteEvent {
        self.last_event.clone()
    }
    /// Bornes OTE bull (top, bot) — pour le ZoneCoeurDetector.
    pub fn bull_bounds(&self) -> Option<(f64, f64)> {
        match (self.last_event.bull_top, self.last_event.bull_bot) {
            (Some(t), Some(b)) => Some((t, b)),
            _ => None,
        }
    }
    /// Bornes OTE bear (top, bot) — pour le ZoneCoeurDetector.
    pub fn bear_bounds(&self) -> Option<(f64, f64)> {
        match (self.last_event.bear_top, self.last_event.bear_bot) {
            (Some(t), Some(b)) => Some((t, b)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(close: f64) -> BarInput {
        BarInput::new(close, close + 1.0, close - 1.0, close)
    }

    #[test]
    fn expiration_m15_vaut_12() {
        let det = OteDetector::new(900); // M15 = 900 s
        assert_eq!(det.expiry_bars(), 12);
    }

    #[test]
    fn expiration_m5_vaut_36() {
        let det = OteDetector::new(300); // M5 = 300 s
        assert_eq!(det.expiry_bars(), 36, "round(10800/300)=36");
    }

    #[test]
    fn expiration_h1_vaut_3() {
        let det = OteDetector::new(3600); // H1 = 3600 s
        assert_eq!(det.expiry_bars(), 3, "round(10800/3600)=3");
    }

    #[test]
    fn zone_bull_apres_bos_haussier() {
        let mut det = OteDetector::new(900);
        // BOS haussier : sh1=120, sl1=100 ⇒ rng=20.
        // oteTopBull = 120 - 20*0.618 = 120 - 12.36 = 107.64
        // oteBotBull = 120 - 20*0.786 = 120 - 15.72 = 104.28
        let ev = det.update(&bar(106.0), true, false, Some(120.0), Some(100.0));
        let top = 120.0 - 20.0 * 0.618;
        let bot = 120.0 - 20.0 * 0.786;
        assert!((ev.bull_top.unwrap() - top).abs() < 1e-9);
        assert!((ev.bull_bot.unwrap() - bot).abs() < 1e-9);
        // close=106 ∈ [104.28, 107.64] ⇒ inOTE_bull.
        assert!(ev.in_ote_bull, "close=106 dans la zone OTE bull");
        assert!(!ev.in_ote_bear);
    }

    #[test]
    fn zone_bear_apres_bos_baissier() {
        let mut det = OteDetector::new(900);
        // BOS baissier : sh1=120, sl1=100 ⇒ rng=20.
        // oteBotBear = 100 + 20*0.618 = 112.36
        // oteTopBear = 100 + 20*0.786 = 115.72
        let ev = det.update(&bar(114.0), false, true, Some(120.0), Some(100.0));
        assert!(ev.in_ote_bear, "close=114 dans la zone OTE bear");
        assert!(!ev.in_ote_bull);
    }

    #[test]
    fn zone_expire_apres_expiry_bars() {
        let mut det = OteDetector::new(900); // expiry = 12
        det.update(&bar(106.0), true, false, Some(120.0), Some(100.0));
        assert!(det.last_event().bull_top.is_some());
        // 12 bars plus tard (cur - bull_bar = 12, pas > 12) : encore actif.
        for _ in 0..12 {
            det.update(&bar(106.0), false, false, Some(120.0), Some(100.0));
        }
        assert!(det.last_event().bull_top.is_some(), "écart=12, pas encore expiré");
        // 1 bar de plus : écart=13 > 12 ⇒ expiration.
        det.update(&bar(106.0), false, false, Some(120.0), Some(100.0));
        assert!(det.last_event().bull_top.is_none(), "écart>expiry ⇒ expiré");
    }

    #[test]
    fn pas_de_zone_avant_bos() {
        let mut det = OteDetector::new(900);
        let ev = det.update(&bar(106.0), false, false, Some(120.0), Some(100.0));
        assert!(!ev.in_ote_bull && !ev.in_ote_bear);
        assert!(ev.bull_top.is_none());
    }
}
