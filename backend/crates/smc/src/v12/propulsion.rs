//! MODULE 8c — Propulsion Blocks.
//!
//! Reproduit MODULE 8c Pine (lignes 1398-1518).
//!
//! Un Propulsion Block naît du chevauchement d'un nouveau FVG et d'un OB actif,
//! **même sens** : confluence haute probabilité.
//!
//! - Détection bull (`f_fvgBullOB`) : si `isFVGBull` et qu'un OB bull chevauche,
//!   `ovBot = max(fBot, oBot)`, `ovTop = min(fTop, oTop)`, si `ovTop > ovBot` ⇒ zone.
//! - Détection bear (`f_fvgBearOB`) : symétrique avec FVG bear ∩ OB bear.
//! - FIFO 3 par sens (`i_maxProp = 3`).
//! - Lifecycle : Propulsion bull supprimé si `close < bot` ; bear si `close > top`.
//!
//! L'ordre Pine : détection (`f_fvgBullOB`/`f_fvgBearOB`, lignes 1441/1469) PUIS
//! lifecycle (`f_propBullLifecycle`/`f_propBearLifecycle`, lignes 1494/1517). La
//! détection lit l'état des OB **post-lifecycle** (les OB invalidés ce tour sont
//! déjà supprimés).

use super::fvg::remove_descending;
use super::order_blocks::ObDetector;
use super::types::{BarInput, FvgEvent, PropulsionEvent, PropulsionZone};

/// `i_maxProp` (Pine ligne 1402) — Propulsion Blocks max par sens.
pub const MAX_PROP: usize = 3;

/// Détecteur de Propulsion Blocks (bull + bear).
#[derive(Clone)]
pub struct PropulsionDetector {
    bull: Vec<PropulsionZone>,
    bear: Vec<PropulsionZone>,
    bar_count: usize,
    last_event: PropulsionEvent,
}

impl PropulsionDetector {
    pub fn new() -> Self {
        Self {
            bull: Vec::with_capacity(MAX_PROP + 1),
            bear: Vec::with_capacity(MAX_PROP + 1),
            bar_count: 0,
            last_event: PropulsionEvent::default(),
        }
    }

    /// Traite une bar. `fvg` = détection FVG courante (bornes + flags) ;
    /// `obs` = Order Blocks actifs (post-lifecycle), lus pour le chevauchement.
    pub fn update(&mut self, bar: &BarInput, fvg: &FvgEvent, obs: &ObDetector) -> PropulsionEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;

        // --- Détection bull (f_fvgBullOB, Pine lignes 1416-1438) ---
        let mut new_bull = Vec::new();
        if fvg.is_fvg_bull {
            let f_top = fvg.bull_top; // low
            let f_bot = fvg.bull_bot; // high[2]
            for ob in obs.bull_zones() {
                let ov_bot = f_bot.max(ob.bot);
                let ov_top = f_top.min(ob.top);
                if ov_top > ov_bot {
                    if self.bull.len() >= MAX_PROP {
                        self.bull.remove(0);
                    }
                    let z = PropulsionZone {
                        top: ov_top,
                        bot: ov_bot,
                        bar: cur_idx,
                        bull: true,
                    };
                    self.bull.push(z);
                    new_bull.push(z);
                }
            }
        }

        // --- Détection bear (f_fvgBearOB, Pine lignes 1444-1466) ---
        let mut new_bear = Vec::new();
        if fvg.is_fvg_bear {
            let f_top = fvg.bear_top; // low[2]
            let f_bot = fvg.bear_bot; // high
            for ob in obs.bear_zones() {
                let ov_bot = f_bot.max(ob.bot);
                let ov_top = f_top.min(ob.top);
                if ov_top > ov_bot {
                    if self.bear.len() >= MAX_PROP {
                        self.bear.remove(0);
                    }
                    let z = PropulsionZone {
                        top: ov_top,
                        bot: ov_bot,
                        bar: cur_idx,
                        bull: false,
                    };
                    self.bear.push(z);
                    new_bear.push(z);
                }
            }
        }

        // --- Lifecycle bull (f_propBullLifecycle, Pine lignes 1472-1491) ---
        let mut del_bull = Vec::new();
        for i in 0..self.bull.len() {
            if bar.close < self.bull[i].bot {
                del_bull.push(i);
            }
        }
        remove_descending(&mut self.bull, &del_bull);

        // --- Lifecycle bear (f_propBearLifecycle, Pine lignes 1496-1515) ---
        let mut del_bear = Vec::new();
        for i in 0..self.bear.len() {
            if bar.close > self.bear[i].top {
                del_bear.push(i);
            }
        }
        remove_descending(&mut self.bear, &del_bear);

        let ev = PropulsionEvent { new_bull, new_bear };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> PropulsionEvent {
        self.last_event.clone()
    }
    pub fn bull_zones(&self) -> &[PropulsionZone] {
        &self.bull
    }
    pub fn bear_zones(&self) -> &[PropulsionZone] {
        &self.bear
    }
}

impl Default for PropulsionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v12::breaker::BreakerDetector;
    use crate::v12::order_blocks::ObDetector;

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

    /// Scénario : un OB bull est créé, puis un FVG bull le chevauche ⇒ Propulsion bull.
    #[test]
    fn propulsion_bull_cree_si_fvg_chevauche_ob() {
        let mut obs = ObDetector::new();
        let mut brk = BreakerDetector::new();
        // Crée un OB bull : bar1 baissière (top=102 bot=98), bar2 impulsion.
        obs.update(
            &bar(0, 100.0, 101.0, 99.0, 100.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        obs.update(
            &bar(1, 100.0, 102.0, 98.0, 98.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        let _ = obs.update(
            &bar(2, 99.0, 112.0, 99.0, 110.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        assert!(!obs.bull_zones().is_empty());

        // Prépare le détecteur FVG simulé : FVG bull avec top=108, bot=100
        // (chevauche l'OB top=102 bot=98 ⇒ overlap [100,102]).
        let fvg = FvgEvent {
            is_fvg_bull: true,
            is_fvg_bear: false,
            bull_top: 108.0,
            bull_bot: 100.0,
            bear_top: 0.0,
            bear_bot: 0.0,
            new_bull: None,
            new_bear: None,
        };
        let mut det = PropulsionDetector::new();
        // cur bar : close=109 > overlap bot=100 ⇒ non supprimé.
        let ev = det.update(&bar(3, 108.0, 110.0, 107.0, 109.0), &fvg, &obs);
        assert_eq!(ev.new_bull.len(), 1, "FVG∩OB bull chevauchent");
        let z = &ev.new_bull[0];
        assert_eq!(z.top, 102.0, "ovTop = min(108,102)");
        assert_eq!(z.bot, 100.0, "ovBot = max(100,98)");
        assert_eq!(det.bull_zones().len(), 1);
    }

    /// Pas de propulsion si pas de chevauchement.
    #[test]
    fn pas_de_propulsion_sans_chevauchement() {
        let mut obs = ObDetector::new();
        let mut brk = BreakerDetector::new();
        obs.update(
            &bar(0, 100.0, 101.0, 99.0, 100.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        obs.update(
            &bar(1, 100.0, 102.0, 98.0, 98.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        let _ = obs.update(
            &bar(2, 99.0, 112.0, 99.0, 110.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        // FVG bull loin de l'OB : top=150 bot=140 (OB est à 98-102).
        let fvg = FvgEvent {
            is_fvg_bull: true,
            is_fvg_bear: false,
            bull_top: 150.0,
            bull_bot: 140.0,
            bear_top: 0.0,
            bear_bot: 0.0,
            new_bull: None,
            new_bear: None,
        };
        let mut det = PropulsionDetector::new();
        let ev = det.update(&bar(3, 145.0, 151.0, 144.0, 149.0), &fvg, &obs);
        assert!(ev.new_bull.is_empty(), "pas de chevauchement");
    }

    /// Lifecycle : propulsion bull supprimé si close < bot.
    #[test]
    fn propulsion_bull_supprime_si_close_sous_bot() {
        let mut obs = ObDetector::new();
        let mut brk = BreakerDetector::new();
        obs.update(
            &bar(0, 100.0, 101.0, 99.0, 100.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        obs.update(
            &bar(1, 100.0, 102.0, 98.0, 98.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        let _ = obs.update(
            &bar(2, 99.0, 112.0, 99.0, 110.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        let fvg = FvgEvent {
            is_fvg_bull: true,
            is_fvg_bear: false,
            bull_top: 108.0,
            bull_bot: 100.0,
            bear_top: 0.0,
            bear_bot: 0.0,
            new_bull: None,
            new_bear: None,
        };
        let mut det = PropulsionDetector::new();
        det.update(&bar(3, 108.0, 110.0, 107.0, 109.0), &fvg, &obs);
        assert_eq!(det.bull_zones().len(), 1);
        // Sans nouveau FVG, on relance avec is_fvg_bull=false et close sous le bot.
        let no_fvg = FvgEvent::default();
        det.update(&bar(4, 102.0, 103.0, 99.0, 99.0), &no_fvg, &obs);
        assert_eq!(det.bull_zones().len(), 0, "close<bot ⇒ propulsion supprimé");
    }

    /// FIFO 3 par sens.
    #[test]
    fn propulsion_fifo_limite_a_3() {
        let mut obs = ObDetector::new();
        let mut brk = BreakerDetector::new();
        // Un seul OB bull couvrant [98,102].
        obs.update(
            &bar(0, 100.0, 101.0, 99.0, 100.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        obs.update(
            &bar(1, 100.0, 102.0, 98.0, 98.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        let _ = obs.update(
            &bar(2, 99.0, 112.0, 99.0, 110.0),
            5.0,
            false,
            false,
            &mut brk,
            &super::super::types::SweepEvent::default(),
        );
        let fvg = FvgEvent {
            is_fvg_bull: true,
            is_fvg_bear: false,
            bull_top: 108.0,
            bull_bot: 100.0,
            bear_top: 0.0,
            bear_bot: 0.0,
            new_bull: None,
            new_bear: None,
        };
        let mut det = PropulsionDetector::new();
        // Chaque bar crée 1 propulsion (1 OB chevauchant), en maintenant close > bot.
        for k in 0..6usize {
            det.update(&bar(3 + k, 108.0, 110.0, 107.0, 109.0), &fvg, &obs);
        }
        assert_eq!(det.bull_zones().len(), MAX_PROP, "plafonné à 3 par FIFO");
    }
}
