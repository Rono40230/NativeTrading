//! MODULE 6 — FVG (Fair Value Gap).
//!
//! Reproduit MODULE 6 Pine (lignes 818-973).
//!
//! - `minGap = i_minFVG × atr14` avec `i_minFVG = 0.20`.
//! - `isFVGBull = (low - high[2]) > minGap`  (gap haussier 3-barres).
//! - `isFVGBear = (low[2] - high) > minGap`.
//! - Création : `top = low`, `bot = high[2]` (bull) / `top = low[2]`, `bot = high` (bear),
//!   `bar = bar_index[2]`, `state = 0`.
//! - FIFO 10 zones par sens (`i_maxFVG = 10`).
//! - Lifecycle bull : supprimé si `close < bot` OU âge > 50 ; partiel si `low < top` (state 0→1).
//! - Lifecycle bear : supprimé si `close > top` OU âge > 50 ; partiel si `high > bot` (state 0→1).
//! - Âge max `i_fvgMaxAge = 50` bars (`bar_index - fvgBar > 50`).

use super::types::{BarInput, FvgEvent, FvgState, FvgZone};

/// `i_minFVG` (Pine ligne 822) — taille minimale FVG en × ATR14.
pub const MIN_FVG: f64 = 0.20;
/// `i_maxFVG` (Pine ligne 823) — FVG actifs max par sens.
pub const MAX_FVG: usize = 10;
/// `i_fvgMaxAge` (Pine ligne 824).
pub const FVG_MAX_AGE: i64 = 50;

/// Détecteur de FVG (bull + bear) avec lifecycle 2 états + FIFO + âge max.
#[derive(Clone)]
pub struct FvgDetector {
    bull: Vec<FvgZone>,
    bear: Vec<FvgZone>,
    /// Bar il y a 1 (`[1]`).
    prev1: Option<BarInput>,
    /// Bar il y a 2 (`[2]`).
    prev2: Option<BarInput>,
    bar_count: usize,
    last_event: FvgEvent,
}

impl FvgDetector {
    pub fn new() -> Self {
        Self {
            bull: Vec::with_capacity(MAX_FVG + 1),
            bear: Vec::with_capacity(MAX_FVG + 1),
            prev1: None,
            prev2: None,
            bar_count: 0,
            last_event: FvgEvent::default(),
        }
    }

    /// Traite une bar. `atr14` sert au seuil `minGap = 0.20 × ATR14`.
    pub fn update(&mut self, bar: &BarInput, atr14: f64) -> FvgEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;

        // Bornes FVG potentielles (Pine `_fTop`/`_fBot`). Nécessite high[2]/low[2].
        let (is_fvg_bull, bull_top, bull_bot) = match self.prev2 {
            Some(p2) => {
                let top = bar.low; // low courant
                let bot = p2.high; // high[2]
                ((top - bot) > MIN_FVG * atr14, top, bot)
            }
            None => (false, 0.0, 0.0),
        };
        let (is_fvg_bear, bear_top, bear_bot) = match self.prev2 {
            Some(p2) => {
                let top = p2.low; // low[2]
                let bot = bar.high; // high courant
                ((top - bot) > MIN_FVG * atr14, top, bot)
            }
            None => (false, 0.0, 0.0),
        };

        // ── Création bull (f_fvgBullCreate, Pine lignes 847-868) ──
        let new_bull = if is_fvg_bull {
            if self.bull.len() >= MAX_FVG {
                self.bull.remove(0); // array.shift — éviction du plus ancien
            }
            let zone = FvgZone {
                top: bull_top,
                bot: bull_bot,
                state: FvgState::Fresh,
                bar: cur_idx.saturating_sub(2), // bar_index[2]
            };
            self.bull.push(zone);
            Some(zone)
        } else {
            None
        };

        // ── Création bear (f_fvgBearCreate, Pine lignes 871-892) ──
        let new_bear = if is_fvg_bear {
            if self.bear.len() >= MAX_FVG {
                self.bear.remove(0);
            }
            let zone = FvgZone {
                top: bear_top,
                bot: bear_bot,
                state: FvgState::Fresh,
                bar: cur_idx.saturating_sub(2),
            };
            self.bear.push(zone);
            Some(zone)
        } else {
            None
        };

        // ── Lifecycle bull (f_fvgBullLifecycle, Pine lignes 895-931) ──
        lifecycle_fvg_bull(&mut self.bull, bar, cur_idx);
        // ── Lifecycle bear (f_fvgBearLifecycle, Pine lignes 935-970) ──
        lifecycle_fvg_bear(&mut self.bear, bar, cur_idx);

        // Mémorise l'historique pour high[2]/low[2] de la prochaine bar.
        self.prev2 = self.prev1;
        self.prev1 = Some(*bar);

        let ev = FvgEvent {
            is_fvg_bull,
            is_fvg_bear,
            bull_top,
            bull_bot,
            bear_top,
            bear_bot,
            new_bull,
            new_bear,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> FvgEvent {
        self.last_event.clone()
    }
    pub fn bull_zones(&self) -> &[FvgZone] {
        &self.bull
    }
    pub fn bear_zones(&self) -> &[FvgZone] {
        &self.bear
    }
}

impl Default for FvgDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle FVG bull : suppression si `close < bot` ou âge > 50 ; partial si `low < top`.
fn lifecycle_fvg_bull(zones: &mut Vec<FvgZone>, bar: &BarInput, cur_idx: usize) {
    let mut del = Vec::new();
    for (i, z) in zones.iter_mut().enumerate() {
        let old = FVG_MAX_AGE > 0 && (cur_idx as i64 - z.bar as i64) > FVG_MAX_AGE;
        if bar.close < z.bot || old {
            del.push(i);
        } else if bar.low < z.top && z.state == FvgState::Fresh {
            z.state = FvgState::Partial;
        }
    }
    remove_descending(zones, &del);
}

/// Lifecycle FVG bear : suppression si `close > top` ou âge > 50 ; partial si `high > bot`.
fn lifecycle_fvg_bear(zones: &mut Vec<FvgZone>, bar: &BarInput, cur_idx: usize) {
    let mut del = Vec::new();
    for (i, z) in zones.iter_mut().enumerate() {
        let old = FVG_MAX_AGE > 0 && (cur_idx as i64 - z.bar as i64) > FVG_MAX_AGE;
        if bar.close > z.top || old {
            del.push(i);
        } else if bar.high > z.bot && z.state == FvgState::Fresh {
            z.state = FvgState::Partial;
        }
    }
    remove_descending(zones, &del);
}

/// Supprime les indices `del` (triés desc) — reproduit `array.sort(order.descending)` +
/// `array.remove` du Pine (nécessaire pour préserver les indices durant la boucle).
pub(crate) fn remove_descending<T>(v: &mut Vec<T>, del: &[usize]) {
    if del.is_empty() {
        return;
    }
    let mut sorted = del.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    for idx in sorted {
        if idx < v.len() {
            v.remove(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: usize, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: i as i64,
            open: close,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    /// FVG bull : bar0 high=100, bar1 (mèche), bar2 low=105 → gap (105-100)=5 > minGap.
    /// minGap = 0.20×10 = 2 → 5 > 2 ⇒ FVG bull créé à la bar 2.
    #[test]
    fn fvg_bull_cree_sur_gap_3_barres() {
        let mut det = FvgDetector::new();
        // bar0 : high=100, low=95 ; bar1 : transition ; bar2 : low=105.
        det.update(&bar(0, 100.0, 95.0, 99.0), 10.0);
        det.update(&bar(1, 102.0, 98.0, 101.0), 10.0);
        let ev = det.update(&bar(2, 108.0, 105.0, 107.0), 10.0);
        assert!(ev.is_fvg_bull, "low[2]=105 - high[0]=100 = 5 > minGap 2");
        let z = ev.new_bull.expect("FVG bull créé");
        assert_eq!(z.top, 105.0, "topB = low courant");
        assert_eq!(z.bot, 100.0, "botB = high[2]");
        assert_eq!(z.bar, 0, "bar = bar_index[2] = 0");
        assert_eq!(det.bull_zones().len(), 1);
    }

    /// FVG bear : bar2 high sous low[2].
    #[test]
    fn fvg_bear_cree_sur_gap_baissier() {
        let mut det = FvgDetector::new();
        det.update(&bar(0, 110.0, 100.0, 101.0), 10.0); // low[2]=100
        det.update(&bar(1, 108.0, 99.0, 100.0), 10.0);
        // bar2 : high=95 → low[2]-high = 100-95 = 5 > 2 ⇒ FVG bear.
        let ev = det.update(&bar(2, 95.0, 90.0, 92.0), 10.0);
        assert!(ev.is_fvg_bear);
        let z = ev.new_bear.unwrap();
        assert_eq!(z.top, 100.0, "topBr = low[2]");
        assert_eq!(z.bot, 95.0, "botBr = high courant");
    }

    /// Suppression si close < bot (FVG bull).
    #[test]
    fn fvg_bull_supprime_si_close_sous_bot() {
        let mut det = FvgDetector::new();
        det.update(&bar(0, 100.0, 95.0, 99.0), 10.0);
        det.update(&bar(1, 102.0, 98.0, 101.0), 10.0);
        det.update(&bar(2, 108.0, 105.0, 107.0), 10.0); // FVG bull bot=100
        assert_eq!(det.bull_zones().len(), 1);
        // Bar 3 : close=98 < bot=100 ⇒ supprimé.
        det.update(&bar(3, 99.0, 97.0, 98.0), 10.0);
        assert_eq!(det.bull_zones().len(), 0, "close<bot ⇒ FVG supprimé");
    }

    /// Partial si low < top (state 0→1), sans suppression (close reste > bot).
    #[test]
    fn fvg_bull_partiel_si_low_sous_top() {
        let mut det = FvgDetector::new();
        det.update(&bar(0, 100.0, 95.0, 99.0), 10.0);
        det.update(&bar(1, 102.0, 98.0, 101.0), 10.0);
        det.update(&bar(2, 108.0, 105.0, 107.0), 10.0); // FVG top=105 bot=100
                                                        // Bar 3 : low=103 < top=105 ET close=104 > bot=100 ⇒ partial, pas supprimé.
        det.update(&bar(3, 106.0, 103.0, 104.0), 10.0);
        assert_eq!(det.bull_zones().len(), 1);
        assert_eq!(det.bull_zones()[0].state, FvgState::Partial);
    }

    /// FIFO : au-delà de 10 FVG bull, le plus ancien est évicté.
    /// high constant=100, low croissant=100+i ⇒ chaque bar i≥2 crée un FVG bull
    /// (gap = low[i]-high[i-2] = i > minGap). close=100+i+5 reste > bot=100 ⇒ aucun
    /// FVG supprimé entre les créations, on sature donc le cap FIFO.
    #[test]
    fn fvg_fifo_limite_a_10_par_sens() {
        let mut det = FvgDetector::new();
        for i in 0..14usize {
            let b = bar(i, 100.0, 100.0 + i as f64, 100.0 + i as f64 + 5.0);
            det.update(&b, 1.0); // atr14=1 ⇒ minGap=0.2 ; gap=i (>0.2 dès i≥1)
        }
        assert_eq!(det.bull_zones().len(), MAX_FVG, "plafonné à 10 par FIFO");
    }

    /// Âge max : FVG supprimé après > 50 bars.
    #[test]
    fn fvg_supprime_apres_age_max_50() {
        let mut det = FvgDetector::new();
        det.update(&bar(0, 100.0, 95.0, 99.0), 10.0);
        det.update(&bar(1, 102.0, 98.0, 101.0), 10.0);
        det.update(&bar(2, 108.0, 105.0, 107.0), 10.0); // FVG bar=0, bot=100
        assert_eq!(det.bull_zones().len(), 1);
        // On maintient close > bot=100 pendant 50+ bars.
        for i in 3..=55usize {
            // close=106 > bot=100 ; low=105.5 > top=105 ⇒ ni supprimé ni partial.
            det.update(&bar(i, 109.0, 105.5, 106.0), 10.0);
        }
        // À la bar 55 : age = 55 - 0 = 55 > 50 ⇒ supprimé.
        assert_eq!(
            det.bull_zones().len(),
            0,
            "âge > 50 ⇒ FVG supprimé (age=55 à la bar 55)"
        );
    }

    /// Pas de FVG si gap insuffisant (< minGap).
    #[test]
    fn pas_de_fvg_si_gap_insuffisant() {
        let mut det = FvgDetector::new();
        det.update(&bar(0, 100.0, 95.0, 99.0), 10.0);
        det.update(&bar(1, 102.0, 98.0, 101.0), 10.0);
        // bar2 : low=100.5, high[0]=100 → gap 0.5 < minGap 2 ⇒ pas de FVG.
        let ev = det.update(&bar(2, 103.0, 100.5, 102.0), 10.0);
        assert!(!ev.is_fvg_bull);
        assert!(ev.new_bull.is_none());
    }

    /// Pas de FVG avant 3 bars (high[2] indisponible).
    #[test]
    fn pas_de_fvg_avant_3_bars() {
        let mut det = FvgDetector::new();
        let ev0 = det.update(&bar(0, 100.0, 95.0, 99.0), 10.0);
        let ev1 = det.update(&bar(1, 200.0, 105.0, 150.0), 10.0);
        assert!(
            !ev0.is_fvg_bull && !ev1.is_fvg_bull,
            "high[2]/low[2] absent"
        );
    }
}
