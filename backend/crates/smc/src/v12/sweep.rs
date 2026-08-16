//! MODULE 5 — Sweep (manipulation de liquidité).
//!
//! Reproduit MODULE 5 Pine (lignes 730-816) — machine 5 phases :
//!
//! 1. **Armé**     : `low < dernierEQL_level` (sweepH) ou `high > dernierEQH_level` (sweepB).
//! 2. **Expire**   : si `bar_index - sweepH_bar > i_maxSwpB (=3)` ⇒ désarmé sans confirmation.
//! 3. **Confirmé** : `close > sweepH_level` (haussier) ou `close < sweepB_level` (baissier).
//! 4. **Consommé** : `dernierEQL_level := na`, niveau marqué `swept` dans `liqPool`.
//! 5. **Fraîcheur** : `SWEEP_FRESH_BARS = max(1, round(4500/_tfSec))` — un sweep ne
//!    compte au scoring que s'il est récent (Phase 5.1 Pine).
//!
//! L'armement se fait sur le niveau EQL/EQH le plus récent (MODULE 4). Le détecteur
//! mute donc le `LiquiditesDetector` (lire `dernier_eql/eqh_level`, les clearer, et
//! marquer le pool sweepé via `mark_swept`).

use super::liquidites::LiquiditesDetector;
use super::types::{BarInput, SweepEvent};

/// `i_maxSwpB` (Pine ligne 733) : fenêtre d'expiration de l'armement.
const MAX_SWP_B: i64 = 3;

/// Détecteur de sweep avec machine 5 phases.
#[derive(Clone)]
pub struct SweepDetector {
    /// `sweepH_bar` (Pine) — bar d'armement du sweep haussier (sur EQL).
    sweep_h_bar: Option<usize>,
    /// `sweepH_level` (Pine) — niveau EQL armé.
    sweep_h_level: Option<f64>,
    /// `sweepB_bar` (Pine) — bar d'armement du sweep baissier (sur EQH).
    sweep_b_bar: Option<usize>,
    /// `sweepB_level` (Pine) — niveau EQH armé.
    sweep_b_level: Option<f64>,

    /// `dernierSweepH_level/bar` (Pine lignes 760-763) — dernier sweep haussier confirmé.
    dernier_sweep_h_level: Option<f64>,
    dernier_sweep_h_bar: Option<usize>,
    /// `dernierSweepB_level/bar` (Pine) — dernier sweep baissier confirmé.
    dernier_sweep_b_level: Option<f64>,
    dernier_sweep_b_bar: Option<usize>,

    /// `_tfSec` (Pine) — timeframe en secondes, pour SWEEP_FRESH_BARS.
    tf_sec: i64,
    /// Index de bar courant (Pine `bar_index`).
    bar_count: usize,
    last_event: SweepEvent,
}

impl SweepDetector {
    /// `tf_sec` = timeframe en secondes (ex. 900 pour M15).
    pub fn new(tf_sec: i64) -> Self {
        Self {
            sweep_h_bar: None,
            sweep_h_level: None,
            sweep_b_bar: None,
            sweep_b_level: None,
            dernier_sweep_h_level: None,
            dernier_sweep_h_bar: None,
            dernier_sweep_b_level: None,
            dernier_sweep_b_bar: None,
            tf_sec,
            bar_count: 0,
            last_event: SweepEvent::default(),
        }
    }

    /// `SWEEP_FRESH_BARS = max(1, round(4500/_tfSec))` (Pine ligne 799).
    pub fn sweep_fresh_bars(&self) -> i64 {
        if self.tf_sec <= 0 {
            5
        } else {
            let raw = (4500.0 / self.tf_sec as f64).round();
            1_i64.max(raw as i64)
        }
    }

    /// Traite une bar. `liquidites` est muté (clear dernier niveau + mark_swept au
    /// sweep confirmé). `atr14` sert à calculer `tolEq = 0.20 × ATR14`.
    pub fn update(
        &mut self,
        bar: &BarInput,
        liquidites: &mut LiquiditesDetector,
        atr14: f64,
    ) -> SweepEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;
        let tol_eq = 0.20 * atr14;
        let fresh = self.sweep_fresh_bars();

        // ── 1. ARMÉ (Pine lignes 744-749) ──
        // sweepH : low casse le niveau EQL, pas déjà armé.
        if self.sweep_h_bar.is_none() {
            if let Some(eql) = liquidites.dernier_eql_level() {
                if bar.low < eql {
                    self.sweep_h_bar = Some(cur_idx);
                    self.sweep_h_level = Some(eql);
                }
            }
        }
        // sweepB : high casse le niveau EQH, pas déjà armé.
        if self.sweep_b_bar.is_none() {
            if let Some(eqh) = liquidites.dernier_eqh_level() {
                if bar.high > eqh {
                    self.sweep_b_bar = Some(cur_idx);
                    self.sweep_b_level = Some(eqh);
                }
            }
        }

        // ── 2. EXPIRE (Pine lignes 750-755) ──
        if let Some(sb) = self.sweep_h_bar {
            if (cur_idx as i64 - sb as i64) > MAX_SWP_B {
                self.sweep_h_bar = None;
                self.sweep_h_level = None;
            }
        }
        if let Some(sb) = self.sweep_b_bar {
            if (cur_idx as i64 - sb as i64) > MAX_SWP_B {
                self.sweep_b_bar = None;
                self.sweep_b_level = None;
            }
        }

        // ── 3. CONFIRMÉ (Pine lignes 757-758) ──
        let sweep_h_level_now = self.sweep_h_level;
        let sweep_haussier = match (self.sweep_h_bar, self.sweep_h_level) {
            (Some(_), Some(lvl)) => bar.close > lvl,
            _ => false,
        };
        let sweep_b_level_now = self.sweep_b_level;
        let sweep_baissier = match (self.sweep_b_bar, self.sweep_b_level) {
            (Some(_), Some(lvl)) => bar.close < lvl,
            _ => false,
        };

        // ── 4. CONSOMMÉ (Pine lignes 765-791) ──
        if sweep_haussier {
            self.dernier_sweep_h_level = sweep_h_level_now;
            self.dernier_sweep_h_bar = Some(cur_idx);
            self.sweep_h_bar = None;
            self.sweep_h_level = None;
            liquidites.clear_dernier_eql(); // `dernierEQL_level := na`
            if let Some(lvl) = sweep_h_level_now {
                liquidites.mark_swept(false, lvl, tol_eq); // marque le niveau EQL correspondant
            }
        }
        if sweep_baissier {
            self.dernier_sweep_b_level = sweep_b_level_now;
            self.dernier_sweep_b_bar = Some(cur_idx);
            self.sweep_b_bar = None;
            self.sweep_b_level = None;
            liquidites.clear_dernier_eqh(); // `dernierEQH_level := na`
            if let Some(lvl) = sweep_b_level_now {
                liquidites.mark_swept(true, lvl, tol_eq); // marque le niveau EQH correspondant
            }
        }

        // ── 5. FRAÎCHEUR (Pine lignes 800-801) ──
        let sweep_bull_frais = match self.dernier_sweep_h_bar {
            Some(db) => (cur_idx as i64 - db as i64) <= fresh,
            None => false,
        };
        let sweep_bear_frais = match self.dernier_sweep_b_bar {
            Some(db) => (cur_idx as i64 - db as i64) <= fresh,
            None => false,
        };

        let ev = SweepEvent {
            sweep_haussier,
            sweep_baissier,
            sweep_h_level: sweep_h_level_now.filter(|_| sweep_haussier),
            sweep_h_bar: if sweep_haussier { Some(cur_idx) } else { None },
            sweep_b_level: sweep_b_level_now.filter(|_| sweep_baissier),
            sweep_b_bar: if sweep_baissier { Some(cur_idx) } else { None },
            sweep_h_armed: self.sweep_h_bar.is_some(),
            sweep_b_armed: self.sweep_b_bar.is_some(),
            dernier_sweep_h_level: self.dernier_sweep_h_level,
            dernier_sweep_h_bar: self.dernier_sweep_h_bar,
            dernier_sweep_b_level: self.dernier_sweep_b_level,
            dernier_sweep_b_bar: self.dernier_sweep_b_bar,
            sweep_bull_frais,
            sweep_bear_frais,
            sweep_fresh_bars: fresh,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> SweepEvent {
        self.last_event.clone()
    }
    pub fn dernier_sweep_h_level(&self) -> Option<f64> {
        self.dernier_sweep_h_level
    }
    pub fn dernier_sweep_b_level(&self) -> Option<f64> {
        self.dernier_sweep_b_level
    }
}

impl Default for SweepDetector {
    fn default() -> Self {
        Self::new(900)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v12::types::BarInput;

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

    #[test]
    fn sweep_fresh_bars_m15_vaut_5() {
        let det = SweepDetector::new(900);
        assert_eq!(det.sweep_fresh_bars(), 5, "M15 ⇒ round(4500/900)=5");
    }

    #[test]
    fn sweep_fresh_bars_m5_vaut_15() {
        let det = SweepDetector::new(300);
        assert_eq!(det.sweep_fresh_bars(), 15, "M5 ⇒ round(4500/300)=15");
    }

    #[test]
    fn sweep_fresh_bars_planche_a_1() {
        let det = SweepDetector::new(10_000);
        assert_eq!(det.sweep_fresh_bars(), 1, "round(4500/10000)=0 plafonné à 1");
    }

    /// Sweep haussier complet : armement sur EQL, puis close revient au-dessus.
    #[test]
    fn sweep_haussier_arme_puis_confirme_et_consomme_eql() {
        let mut liq = LiquiditesDetector::new();
        // Injecte un niveau EQL à 90 et dernier_eql_level = 90.
        liq.set_dernier_eql_for_test(90.0);
        let mut det = SweepDetector::new(900);

        // Bar 0 : low=85 < 90 ⇒ armé. close=88 < 90 ⇒ pas encore confirmé.
        let ev0 = det.update(&bar(0, 95.0, 85.0, 88.0), &mut liq, 10.0);
        assert!(ev0.sweep_h_armed, "low<90 ⇒ sweepH armé");
        assert!(!ev0.sweep_haussier, "close=88 < 90 ⇒ pas confirmé");
        assert_eq!(liq.dernier_eql_level(), Some(90.0), "niveau non consommé tant que non confirmé");

        // Bar 1 : close=92 > 90 ⇒ confirmé.
        let ev1 = det.update(&bar(1, 96.0, 89.0, 92.0), &mut liq, 10.0);
        assert!(ev1.sweep_haussier, "close=92 > sweepH_level=90 ⇒ sweep haussier");
        assert!(!ev1.sweep_h_armed, "armement consommé après confirmation");
        assert_eq!(liq.dernier_eql_level(), None, "dernierEQL_level := na après sweep");
        assert!(ev1.sweep_bull_frais, "sweep récent ⇒ frais");
    }

    /// Sweep armé puis expiré sans confirmation (i_maxSwpB=3).
    #[test]
    fn sweep_expire_sans_confirmation_apres_3_bars() {
        let mut liq = LiquiditesDetector::new();
        liq.set_dernier_eql_for_test(90.0);
        let mut det = SweepDetector::new(900);

        // Bar 0 : armé (low=85).
        det.update(&bar(0, 95.0, 85.0, 88.0), &mut liq, 10.0);
        assert!(det.last_event().sweep_h_armed);
        // Bars 1..3 : close reste sous 90, armement persiste.
        for i in 1..=3usize {
            det.update(&bar(i, 92.0, 88.0, 89.0), &mut liq, 10.0);
        }
        assert!(det.last_event().sweep_h_armed, "armement tient sur 3 bars");
        // Bar 4 : (4-0)=4 > 3 ⇒ expire AVANT confirm.
        let ev = det.update(&bar(4, 96.0, 89.0, 92.0), &mut liq, 10.0);
        assert!(!ev.sweep_h_armed, "armement expiré");
        assert!(!ev.sweep_haussier, "expiré ⇒ pas de confirmation");
    }

    /// Sweep baissier sur EQH.
    #[test]
    fn sweep_baissier_sur_eqh() {
        let mut liq = LiquiditesDetector::new();
        liq.set_dernier_eqh_for_test(110.0);
        let mut det = SweepDetector::new(900);
        // Bar 0 : high=115 > 110 ⇒ armé. close=108 < 110 ⇒ confirmé (same bar).
        let ev = det.update(&bar(0, 115.0, 105.0, 108.0), &mut liq, 10.0);
        assert!(ev.sweep_baissier, "high>110 ET close<110 même bar ⇒ sweep baissier");
        assert_eq!(liq.dernier_eqh_level(), None, "EQH consommé");
        assert!(ev.sweep_bear_frais);
    }

    /// Pas d'armement si dernierEQL_level est na.
    #[test]
    fn pas_d_armement_sans_niveau() {
        let mut liq = LiquiditesDetector::new();
        // Aucun niveau injecté.
        let mut det = SweepDetector::new(900);
        let ev = det.update(&bar(0, 95.0, 50.0, 60.0), &mut liq, 10.0);
        assert!(!ev.sweep_h_armed && !ev.sweep_haussier);
    }

    /// Fraîcheur expire après SWEEP_FRESH_BARS.
    #[test]
    fn fraicheur_expire_apres_5_bars_m15() {
        let mut liq = LiquiditesDetector::new();
        liq.set_dernier_eql_for_test(90.0);
        let mut det = SweepDetector::new(900);
        // Bar 0 : armé+confirmé (low=85, close=92).
        det.update(&bar(0, 96.0, 85.0, 92.0), &mut liq, 10.0);
        assert!(det.last_event().sweep_bull_frais);
        // Bars 1..5 : frais (<=5).
        for i in 1..=5usize {
            let ev = det.update(&bar(i, 95.0, 88.0, 90.0), &mut liq, 10.0);
            assert!(ev.sweep_bull_frais, "bar {i} encore frais (<=5)");
        }
        // Bar 6 : (6-0)=6 > 5 ⇒ plus frais.
        let ev = det.update(&bar(6, 95.0, 88.0, 90.0), &mut liq, 10.0);
        assert!(!ev.sweep_bull_frais, "bar 6 > SWEEP_FRESH_BARS(5) ⇒ plus frais");
    }
}
