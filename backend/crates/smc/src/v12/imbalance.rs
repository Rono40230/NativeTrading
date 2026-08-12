//! MODULE 13b — Imbalance.
//!
//! Reproduit MODULE 13b Pine (lignes 2578-2702), le flag `ibBull`/`ibBear` étant
//! calculé lignes 432-433.
//!
//! - `ibBull = (close - open) > i_seuilIB × atr14`  (corps haussier disproportionné).
//! - `ibBear = (open - close) > i_seuilIB × atr14`.
//! - Création : box = corps de la bougie. Bull `top = close, bot = open` ;
//!   bear `top = open, bot = close`. `state = 0`.
//! - FIFO 10 par sens (`i_maxIB = 10`).
//! - Lifecycle bull : supprimé si `close <= bot` ; partial si `low <= mid` (state 0→1).
//! - Lifecycle bear : supprimé si `close >= top` ; partial si `high >= mid` (state 0→1).
//!
//! **Note fidélité** : en Pine, la création est aussi gateée par `i_showIB` (toggle
//! d'affichage, défaut OFF). Le moteur Rust est headless (pas d'affichage) : tous
//! les toggles `i_show*` sont considérés activés — la détection tourne toujours
//! (cohérent avec FVG/OB qui ne gate que sur `i_moteur*`).
//!
//! Le flag `ibBull`/`ibBear` est exposé via `last_ib_bull()`/`last_ib_bear()` pour
//! le MODULE 7 (OB stocke `ibBull[1]`/`ibBear[1]`). Comme l'Imbalance s'exécute en
//! dernier dans le moteur, sa valeur "courante" vu par l'OB (qui tourne avant) est
//! celle de la bar précédente = `[1]` en Pine.

use super::fvg::remove_descending;
use super::types::{BarInput, ImbalanceEvent, ImbalanceState, ImbalanceZone};

/// `i_maxIB` (Pine ligne 425) — Imbalance actifs max par sens.
pub const MAX_IB: usize = 10;

/// Détecteur d'Imbalance (bull + bear).
pub struct ImbalanceDetector {
    bull: Vec<ImbalanceZone>,
    bear: Vec<ImbalanceZone>,
    /// `ibBull` de la bar courante (Pine ligne 432) — exposé pour MODULE 7.
    last_ib_bull: bool,
    last_ib_bear: bool,
    /// `_ibLastBull` (Pine ligne 2583) — garde anti-double (tjs vrai en streaming).
    ib_last_bull: i64,
    ib_last_bear: i64,
    bar_count: usize,
    last_event: ImbalanceEvent,
}

impl ImbalanceDetector {
    pub fn new() -> Self {
        Self {
            bull: Vec::with_capacity(MAX_IB + 1),
            bear: Vec::with_capacity(MAX_IB + 1),
            last_ib_bull: false,
            last_ib_bear: false,
            ib_last_bull: -1,
            ib_last_bear: -1,
            bar_count: 0,
            last_event: ImbalanceEvent::default(),
        }
    }

    /// Traite une bar. `seuil_ib` = `_autoSeuilIB` (× ATR14, par asset).
    pub fn update(&mut self, bar: &BarInput, atr14: f64, seuil_ib: f64) -> ImbalanceEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;

        // --- Flags ibBull/ibBear (Pine lignes 432-433) ---
        let threshold = seuil_ib * atr14;
        let ib_bull = (bar.close - bar.open) > threshold;
        let ib_bear = (bar.open - bar.close) > threshold;

        // --- Création bull (Pine lignes 2591-2606) ---
        let new_bull = if ib_bull && cur_idx as i64 != self.ib_last_bull {
            self.ib_last_bull = cur_idx as i64;
            if self.bull.len() >= MAX_IB {
                self.bull.remove(0);
            }
            let zone = ImbalanceZone {
                top: bar.close, // corps bougie
                bot: bar.open,
                state: ImbalanceState::Fresh,
                bar: cur_idx,
                bull: true,
            };
            self.bull.push(zone);
            Some(zone)
        } else {
            None
        };

        // --- Création bear (Pine lignes 2608-2623) ---
        let new_bear = if ib_bear && cur_idx as i64 != self.ib_last_bear {
            self.ib_last_bear = cur_idx as i64;
            if self.bear.len() >= MAX_IB {
                self.bear.remove(0);
            }
            let zone = ImbalanceZone {
                top: bar.open, // bear : top=open, bot=close
                bot: bar.close,
                state: ImbalanceState::Fresh,
                bar: cur_idx,
                bull: false,
            };
            self.bear.push(zone);
            Some(zone)
        } else {
            None
        };

        // --- Lifecycle bull (f_ibBullLifecycle, Pine lignes 2625-2652) ---
        let mut del_bull = Vec::new();
        for i in 0..self.bull.len() {
            let top = self.bull[i].top;
            let bot = self.bull[i].bot;
            if bar.close <= bot {
                del_bull.push(i);
            } else {
                let mid = (top + bot) * 0.5;
                if bar.low <= mid && self.bull[i].state == ImbalanceState::Fresh {
                    self.bull[i].state = ImbalanceState::Partial;
                }
            }
        }
        remove_descending(&mut self.bull, &del_bull);

        // --- Lifecycle bear (f_ibBearLifecycle, Pine lignes 2657-2684) ---
        let mut del_bear = Vec::new();
        for i in 0..self.bear.len() {
            let top = self.bear[i].top;
            let bot = self.bear[i].bot;
            if bar.close >= top {
                del_bear.push(i);
            } else {
                let mid = (top + bot) * 0.5;
                if bar.high >= mid && self.bear[i].state == ImbalanceState::Fresh {
                    self.bear[i].state = ImbalanceState::Partial;
                }
            }
        }
        remove_descending(&mut self.bear, &del_bear);

        // Mémorise les flags pour le MODULE 7 (seront lus comme [1] à la bar suivante).
        self.last_ib_bull = ib_bull;
        self.last_ib_bear = ib_bear;

        let ev = ImbalanceEvent {
            ib_bull,
            ib_bear,
            new_bull,
            new_bear,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> ImbalanceEvent {
        self.last_event.clone()
    }
    /// `ibBull[1]` (Pine) — flag d'imbalance de la bar précédente (pour MODULE 7).
    pub fn last_ib_bull(&self) -> bool {
        self.last_ib_bull
    }
    /// `ibBear[1]` (Pine).
    pub fn last_ib_bear(&self) -> bool {
        self.last_ib_bear
    }
    pub fn bull_zones(&self) -> &[ImbalanceZone] {
        &self.bull
    }
    pub fn bear_zones(&self) -> &[ImbalanceZone] {
        &self.bear
    }
}

impl Default for ImbalanceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Imbalance bull : corps (close-open) > seuilIB×ATR. seuilIB=1.5, ATR=10 ⇒ seuil=15.
    #[test]
    fn imbalance_bull_cree_sur_grand_corps_haussier() {
        let mut det = ImbalanceDetector::new();
        // close-open = 110-100 = 10 < 15 ⇒ pas d'imbalance.
        let ev0 = det.update(&bar(0, 100.0, 111.0, 99.0, 110.0), 10.0, 1.5);
        assert!(!ev0.ib_bull);
        // close-open = 120-100 = 20 > 15 ⇒ imbalance bull.
        let ev1 = det.update(&bar(1, 100.0, 121.0, 99.0, 120.0), 10.0, 1.5);
        assert!(ev1.ib_bull);
        let z = ev1.new_bull.unwrap();
        assert_eq!(z.top, 120.0, "top = close");
        assert_eq!(z.bot, 100.0, "bot = open");
        assert_eq!(det.bull_zones().len(), 1);
    }

    /// Imbalance bear : open-close > seuil.
    #[test]
    fn imbalance_bear_cree_sur_grand_corps_baissier() {
        let mut det = ImbalanceDetector::new();
        let ev = det.update(&bar(0, 120.0, 121.0, 99.0, 100.0), 10.0, 1.5);
        assert!(ev.ib_bear, "open-close=20 > 15");
        let z = ev.new_bear.unwrap();
        assert_eq!(z.top, 120.0, "top = open");
        assert_eq!(z.bot, 100.0, "bot = close");
    }

    /// Suppression si close <= bot (bull).
    #[test]
    fn imbalance_bull_supprime_si_close_sous_bot() {
        let mut det = ImbalanceDetector::new();
        det.update(&bar(0, 100.0, 121.0, 99.0, 120.0), 10.0, 1.5); // IB bull bot=100
        assert_eq!(det.bull_zones().len(), 1);
        det.update(&bar(1, 101.0, 102.0, 99.0, 99.0), 10.0, 1.5); // close=99 <= bot=100
        assert_eq!(det.bull_zones().len(), 0, "close<=bot ⇒ supprimé");
    }

    /// Partial si low <= mid (state 0→1), sans suppression.
    #[test]
    fn imbalance_bull_partiel_si_low_sous_mid() {
        let mut det = ImbalanceDetector::new();
        // IB bull top=120 bot=100 ⇒ mid=110.
        det.update(&bar(0, 100.0, 121.0, 99.0, 120.0), 10.0, 1.5);
        // bar1 : low=105 <= mid=110 ET close=115 > bot=100 ⇒ partial.
        det.update(&bar(1, 114.0, 122.0, 105.0, 115.0), 10.0, 1.5);
        assert_eq!(det.bull_zones().len(), 1);
        assert_eq!(det.bull_zones()[0].state, ImbalanceState::Partial);
    }

    /// last_ib_bull expose le flag de la bar précédente (pour MODULE 7).
    #[test]
    fn last_ib_bull_reflete_la_bar_precedente() {
        let mut det = ImbalanceDetector::new();
        assert!(!det.last_ib_bull(), "initialement faux");
        det.update(&bar(0, 100.0, 121.0, 99.0, 120.0), 10.0, 1.5); // ib_bull=true
        assert!(det.last_ib_bull());
        det.update(&bar(1, 100.0, 101.0, 99.0, 100.0), 10.0, 1.5); // ib_bull=false
        assert!(!det.last_ib_bull(), "mis à jour chaque bar");
    }

    /// FIFO 10 par sens.
    #[test]
    fn imbalance_fifo_limite_a_10() {
        let mut det = ImbalanceDetector::new();
        for i in 0..15usize {
            det.update(&bar(i, 100.0, 130.0, 99.0, 125.0), 10.0, 1.5); // IB bull chaque bar
        }
        assert_eq!(det.bull_zones().len(), MAX_IB, "plafonné à 10");
    }
}
