//! MODULE 7 — Order Blocks.
//!
//! Reproduit MODULE 7 Pine (lignes 1016-1337). C'est le détecteur le plus complexe.
//!
//! **Détection (modèle ROC)** :
//! - `_rocCur = (high - low) / close × 10000` (bps, range complet mèches incluses).
//! - `_rocOk = _rocCur >= i_rocSeuil` (`i_rocSeuil = 5` bps, Pine `_autoRocSeuil`).
//! - `_obImpulseBull = (close > open) and (close[1] < open[1]) and _rocOk`
//!   (impulsion haussière précédée d'une bougie baissière).
//! - `_obImpulseBear = (open > close) and (close[1] > open[1]) and _rocOk`.
//! - OB = bougie précédant l'impulsion : `top = high[1]`, `bot = low[1]`,
//!   `impulse_bar = bar_index` (garde anti-suppression immédiate), `is_ib = ibBull[1]`.
//!
//! **Lifecycle 3 états** (Pine `f_obLifecycle`) :
//! - Invalidation (suppression) si `bar_index > impulse_bar` ET `low <= top` (bull) /
//!   `high >= bot` (bear). À l'invalidation, si `close < bot` (bull) / `close > top`
//!   (bear) → un **Breaker** est créé (cf. MODULE 8b).
//! - Sinon, transitions d'état (`if low <= top` / `if high >= bot`), qui ne se
//!   produisent qu'à la bar de création (garde `bar_index > impulse_bar`) :
//!   - `close <= mid` (bull) / `close >= mid` (bear) et `state < 2` → **Profond** (2).
//!   - `close > mid` (bull) / `close < mid` (bear) et `state == 0` → **Partiel** (1).
//!
//! FIFO 40 par sens (`i_maxOB = 40`). 11 arrays Pine → un seul `Vec<ObZone>` Rust.

use super::breaker::BreakerDetector;
use super::fvg::remove_descending;
use super::types::{BarInput, ObEvent, ObState, ObZone};

/// `i_maxOB` (Pine ligne 1019) — OB actifs max par sens.
pub const MAX_OB: usize = 40;

/// Détecteur d'Order Blocks (bull + bear) avec lifecycle 3 états + spawn de Breakers.
#[derive(Clone)]
pub struct ObDetector {
    bull: Vec<ObZone>,
    bear: Vec<ObZone>,
    /// Bougie précédente (`[1]` en Pine).
    prev_bar: Option<BarInput>,
    bar_count: usize,
    last_event: ObEvent,
}

impl ObDetector {
    pub fn new() -> Self {
        Self {
            bull: Vec::with_capacity(MAX_OB + 1),
            bear: Vec::with_capacity(MAX_OB + 1),
            prev_bar: None,
            bar_count: 0,
            last_event: ObEvent::default(),
        }
    }

    /// Traite une bar.
    ///
    /// - `roc_seuil` : seuil ROC en bps (Pine `i_rocSeuil = 5`).
    /// - `prev_ib_bull` / `prev_ib_bear` : `ibBull[1]` / `ibBear[1]` (Pine) — flag
    ///   d'imbalance de la bougie précédente (fourni par `ImbalanceDetector`).
    /// - `breaker` : reçoit les Breakers créés lors des invalidations (MODULE 8b).
    pub fn update(
        &mut self,
        bar: &BarInput,
        roc_seuil: f64,
        prev_ib_bull: bool,
        prev_ib_bear: bool,
        breaker: &mut BreakerDetector,
    ) -> ObEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;

        // --- ROC + impulsion (Pine lignes 1035-1042) ---
        let roc_cur = if bar.close != 0.0 {
            (bar.high - bar.low) / bar.close * 10000.0
        } else {
            0.0
        };
        let roc_ok = roc_cur >= roc_seuil;
        let (impulse_bull, impulse_bear) = match self.prev_bar {
            Some(p) => {
                let ib = bar.close > bar.open && p.close < p.open && roc_ok;
                let isr = bar.open > bar.close && p.close > p.open && roc_ok;
                (ib, isr)
            }
            None => (false, false),
        };

        // --- Création OB bull (f_newBullOB, Pine lignes 1097-1134) ---
        let new_bull = if impulse_bull {
            if let Some(p) = self.prev_bar {
                if self.bull.len() >= MAX_OB {
                    self.bull.remove(0); // array.shift sur les 11 arrays parallèles
                }
                let zone = ObZone {
                    top: p.high, // high[1]
                    bot: p.low,  // low[1]
                    state: ObState::Vierge,
                    impulse_bar: cur_idx,        // bar_index (garde anti-suppression)
                    ob_bar: cur_idx.saturating_sub(1), // bar_index[1]
                    timestamp: p.timestamp,      // int(time[1])
                    is_ib: prev_ib_bull,         // ibBull[1]
                };
                self.bull.push(zone);
                Some(zone)
            } else {
                None
            }
        } else {
            None
        };

        // --- Création OB bear (f_newBearOB, Pine lignes 1139-1176) ---
        let new_bear = if impulse_bear {
            if let Some(p) = self.prev_bar {
                if self.bear.len() >= MAX_OB {
                    self.bear.remove(0);
                }
                let zone = ObZone {
                    top: p.high,
                    bot: p.low,
                    state: ObState::Vierge,
                    impulse_bar: cur_idx,
                    ob_bar: cur_idx.saturating_sub(1),
                    timestamp: p.timestamp,
                    is_ib: prev_ib_bear, // ibBear[1]
                };
                self.bear.push(zone);
                Some(zone)
            } else {
                None
            }
        } else {
            None
        };

        // --- Lifecycle bull puis bear (f_obLifecycle, Pine lignes 1181-1334) ---
        let invalidated_bull = lifecycle_ob_bull(&mut self.bull, bar, cur_idx, breaker);
        let invalidated_bear = lifecycle_ob_bear(&mut self.bear, bar, cur_idx, breaker);

        self.prev_bar = Some(*bar);

        let ev = ObEvent {
            new_bull,
            new_bear,
            invalidated_bull,
            invalidated_bear,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> ObEvent {
        self.last_event.clone()
    }
    pub fn bull_zones(&self) -> &[ObZone] {
        &self.bull
    }
    pub fn bear_zones(&self) -> &[ObZone] {
        &self.bear
    }
}

impl Default for ObDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle OB bull (Pine lignes 1182-1257). Retourne les OBs invalidés cette bar.
fn lifecycle_ob_bull(
    zones: &mut Vec<ObZone>,
    bar: &BarInput,
    cur_idx: usize,
    breaker: &mut BreakerDetector,
) -> Vec<ObZone> {
    let mut del = Vec::new();
    let mut invalidated = Vec::new();
    for (i, z) in zones.iter_mut().enumerate() {
        let top = z.top;
        let bot = z.bot;
        let impulse_bar = z.impulse_bar;
        // 1) MITIGATION D'ABORD (MQL5 f_obLifecycle « NOTE BUG CORRIGÉ » :
        //    mitigation AVANT suppression, sur TOUTE barre touchante — le
        //    placement dans le `else` empêchait les transitions d'état et
        //    bloquait le ratchet du score → sur-scoring des zones éloignées).
        if bar.low <= top {
            let mid = (top + bot) * 0.5;
            let st = z.state;
            if bar.close <= mid && st < ObState::Profond {
                z.state = ObState::Profond; // close <= mid → DEEP (mitigation ≥ 50 %)
            } else if bar.close > mid && st == ObState::Vierge {
                z.state = ObState::Partiel; // close > mid → PARTIAL
            }
        }
        // 2) PUIS invalidation/suppression (Pine _invalB = low <= top).
        if cur_idx > impulse_bar && bar.low <= top {
            if bar.close < bot {
                breaker.push_bear(top, bot, cur_idx); // Bearish Breaker
            }
            invalidated.push(*z);
            del.push(i);
        }
    }
    remove_descending(zones, &del);
    invalidated
}

/// Lifecycle OB bear (Pine lignes 1259-1334). Retourne les OBs invalidés cette bar.
fn lifecycle_ob_bear(
    zones: &mut Vec<ObZone>,
    bar: &BarInput,
    cur_idx: usize,
    breaker: &mut BreakerDetector,
) -> Vec<ObZone> {
    let mut del = Vec::new();
    let mut invalidated = Vec::new();
    for (i, z) in zones.iter_mut().enumerate() {
        let top = z.top;
        let bot = z.bot;
        let impulse_bar = z.impulse_bar;
        // 1) MITIGATION D'ABORD (MQL5 : avant suppression, toute barre touchante).
        if bar.high >= bot {
            let mid = (top + bot) * 0.5;
            let st = z.state;
            if bar.close >= mid && st < ObState::Profond {
                z.state = ObState::Profond; // close >= mid → DEEP
            } else if bar.close < mid && st == ObState::Vierge {
                z.state = ObState::Partiel; // close < mid → PARTIAL
            }
        }
        // 2) PUIS invalidation/suppression (Pine _invalBr = high >= bot).
        if cur_idx > impulse_bar && bar.high >= bot {
            if bar.close > top {
                breaker.push_bull(top, bot, cur_idx); // Bullish Breaker
            }
            invalidated.push(*z);
            del.push(i);
        }
    }
    remove_descending(zones, &del);
    invalidated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v12::breaker::BreakerDetector;

    fn bar(i: usize, open: f64, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: i as i64,
            open,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    /// Impulsion haussière : bar1 baissière (close<open), bar2 haussière forte (ROC≥5).
    /// OB bull = bar1 (top=high[1]=102, bot=low[1]=98).
    #[test]
    fn ob_bull_cree_sur_impulsion_haussiere() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        // bar1 baissière : close=98 < open=100. high=102 low=98.
        det.update(&bar(1, 100.0, 102.0, 98.0, 98.0), 5.0, false, false, &mut brk);
        // bar2 haussière impulsive : close=110 > open=99, ROC=(112-99)/99*10000≈1313 ≥ 5.
        let ev = det.update(&bar(2, 99.0, 112.0, 99.0, 110.0), 5.0, false, false, &mut brk);
        let z = ev.new_bull.expect("OB bull créé");
        assert_eq!(z.top, 102.0, "top = high[1]");
        assert_eq!(z.bot, 98.0, "bot = low[1]");
        assert_eq!(z.impulse_bar, 2, "impulse_bar = bar_index courant");
        assert_eq!(det.bull_zones().len(), 1);
    }

    /// MQL5 f_obLifecycle « NOTE BUG CORRIGÉ » : la mitigation s'applique
    /// AVANT la suppression, sur TOUTE barre touchante. Une zone tuée
    /// post-création avec close ≤ mid doit mourir à l'état Profond (la
    /// transition précède l'invalidation) — avant le fix, elle mourait
    /// Vierge et le ratchet du score restait bloqué haut (sur-scoring).
    #[test]
    fn mitigation_avant_suppression_sur_barre_touchante_posterieure() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        det.update(&bar(1, 100.0, 102.0, 98.0, 98.0), 5.0, false, false, &mut brk);
        det.update(&bar(2, 99.0, 112.0, 99.0, 110.0), 5.0, false, false, &mut brk);
        // Bar 3 post-création : low 99 ≤ top 102 (toucher), close 99.5 ≤ mid 100
        // → mitigation Profond PUIS invalidation (cur 3 > impulse 2).
        let ev = det.update(&bar(3, 101.0, 103.0, 99.0, 99.5), 5.0, false, false, &mut brk);
        assert_eq!(ev.invalidated_bull.len(), 1, "zone tuée par toucher post-création");
        assert!(
            matches!(ev.invalidated_bull[0].state, ObState::Profond),
            "la zone meurt avec l'état de mitigation appliqué (Profond), pas Vierge"
        );
        assert!(det.bull_zones().is_empty());
    }

    /// Pas d'OB sans impulsion (ROC insuffisant). ROC = (high-low)/close×10000 bps.
    /// Pour ROC < 5 bps il faut un range très petit (< 0.0005×close).
    #[test]
    fn pas_d_ob_si_roc_insuffisant() {        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 100.5, 99.5, 100.0), 5.0, false, false, &mut brk);
        // bar1 baissière : close=99.6 < open=100.
        det.update(&bar(1, 100.0, 100.5, 99.5, 99.6), 5.0, false, false, &mut brk);
        // bar2 haussière (close>open) mais range minuscule : ROC≈1 bps < 5 ⇒ pas d'impulsion.
        let ev = det.update(&bar(2, 99.6, 99.61, 99.6, 99.605), 5.0, false, false, &mut brk);
        assert!(ev.new_bull.is_none() && ev.new_bear.is_none());
    }

    /// Invalidation d'un OB bull (low <= top après la bar de création) → supprimé,
    /// et Breaker bear créé si close < bot.
    #[test]
    fn ob_bull_invalide_cree_breaker_bear() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        det.update(&bar(1, 100.0, 102.0, 98.0, 98.0), 5.0, false, false, &mut brk);
        det.update(&bar(2, 99.0, 112.0, 99.0, 110.0), 5.0, false, false, &mut brk);
        // OB bull : top=102 bot=98 impulse_bar=2.
        // bar3 : low=100 <= top=102 (cur_idx=3 > impulse_bar=2) ET close=97 < bot=98
        //        ⇒ invalidation + Breaker bear.
        let ev = det.update(&bar(3, 99.0, 101.0, 100.0, 97.0), 5.0, false, false, &mut brk);
        assert_eq!(det.bull_zones().len(), 0, "OB bull invalidé ⇒ supprimé");
        assert_eq!(ev.invalidated_bull.len(), 1);
        assert_eq!(brk.bear_zones().len(), 1, "Breaker bear créé (close<bot)");
        assert_eq!(brk.bear_zones()[0].top, 102.0);
        assert_eq!(brk.bear_zones()[0].bot, 98.0);
    }

    /// Invalidation sans Breaker : low <= top mais close >= bot (pas de cassure directionnelle).
    #[test]
    fn ob_bull_invalide_sans_breaker_si_close_dans_zone() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        det.update(&bar(1, 100.0, 102.0, 98.0, 98.0), 5.0, false, false, &mut brk);
        det.update(&bar(2, 99.0, 112.0, 99.0, 110.0), 5.0, false, false, &mut brk);
        // bar3 : low=100 <= top=102 ET close=100 (> bot=98) ⇒ invalidé, PAS de Breaker.
        det.update(&bar(3, 101.0, 103.0, 100.0, 100.0), 5.0, false, false, &mut brk);
        assert_eq!(det.bull_zones().len(), 0, "invalidé (low<=top)");
        assert_eq!(brk.bear_zones().len(), 0, "close>=bot ⇒ pas de Breaker");
    }

    /// Garde anti-suppression : à la bar de création, low <= top n'invalide pas
    /// (fait seulement une transition d'état).
    #[test]
    fn garde_anti_suppression_a_la_creation() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        // bar1 baissière avec une grande mèche basse (low=95) : OB top=101 bot=95.
        det.update(&bar(1, 100.0, 101.0, 95.0, 98.0), 5.0, false, false, &mut brk);
        // bar2 impulsive : close=110, low=96 <= top=101 MAIS c'est la bar de création
        // (impulse_bar=2 == cur_idx=2) ⇒ pas d'invalidation, transition d'état possible.
        det.update(&bar(2, 99.0, 112.0, 96.0, 110.0), 5.0, false, false, &mut brk);
        assert_eq!(det.bull_zones().len(), 1, "garde anti-suppression ⇒ OB conservé");
        // mid = (101+95)/2 = 98. close=110 > mid ⇒ Partiel (state 0→1).
        assert_eq!(det.bull_zones()[0].state, ObState::Partiel);
    }

    /// OB bear créé sur impulsion baissière.
    #[test]
    fn ob_bear_cree_sur_impulsion_baissiere() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        // bar0 neutre, bar1 haussière (close>open), bar2 baissière impulsive.
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        det.update(&bar(1, 99.0, 103.0, 99.0, 102.0), 5.0, false, false, &mut brk); // close>open
        // bar2 baissière : open=102 > close=88, prev close=102 > prev open=99, ROC grand.
        let ev = det.update(&bar(2, 102.0, 103.0, 86.0, 88.0), 5.0, false, false, &mut brk);
        let z = ev.new_bear.expect("OB bear créé");
        assert_eq!(z.top, 103.0, "top = high[1] de la bougie haussière");
        assert_eq!(z.bot, 99.0, "bot = low[1]");
    }

    /// FIFO 40 par sens.
    #[test]
    fn ob_fifo_limite_a_40_par_sens() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        // Génère 45 impulsions haussières consécutives (pattern 2-barres répété).
        for k in 0..45usize {
            let base = k * 2;
            det.update(&bar(base, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
            // bar baissière puis impulsion : ROC grand.
            det.update(&bar(base + 1, 100.0, 101.0, 99.0, 99.0), 5.0, false, false, &mut brk);
            det.update(
                &bar(base + 2, 99.0, 120.0, 99.0, 115.0),
                5.0,
                false,
                false,
                &mut brk,
            );
        }
        assert!(
            det.bull_zones().len() <= MAX_OB,
            "bull OB plafonné à 40 (got {})",
            det.bull_zones().len()
        );
    }

    /// OB bear invalidé (high >= bot après création) avec close > top ⇒ Breaker bull.
    #[test]
    fn ob_bear_invalide_cree_breaker_bull() {
        let mut det = ObDetector::new();
        let mut brk = BreakerDetector::new();
        det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 5.0, false, false, &mut brk);
        det.update(&bar(1, 99.0, 103.0, 99.0, 102.0), 5.0, false, false, &mut brk);
        det.update(&bar(2, 102.0, 103.0, 86.0, 88.0), 5.0, false, false, &mut brk);
        // OB bear : top=103 bot=99 impulse_bar=2.
        // bar3 : high=104 >= bot=99 (cur_idx=3>2) ET close=104 > top=103 ⇒ Breaker bull.
        det.update(&bar(3, 95.0, 104.0, 95.0, 104.0), 5.0, false, false, &mut brk);
        assert_eq!(det.bear_zones().len(), 0, "OB bear invalidé");
        assert_eq!(brk.bull_zones().len(), 1, "Breaker bull créé");
    }
}
