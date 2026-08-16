//! MODULE 8b — Breaker Blocks.
//!
//! Reproduit MODULE 8b Pine (lignes 1078-1397).
//!
//! Un Breaker naît de l'invalidation d'un Order Block (MODULE 7) :
//! - **Bullish Breaker** (`bbBull`) = Bear OB invalidé par `close > top` → la zone
//!   devient un support. Zone = `top/bot` exacts de l'ancien OB bear.
//! - **Bearish Breaker** (`bbBear`) = Bull OB invalidé par `close < bot` → la zone
//!   devient une résistance. Zone = `top/bot` exacts de l'ancien OB bull.
//!
//! FIFO 5 par sens (`i_maxBB = 5`), éviction à la création.
//! Lifecycle (Pine `f_bbLifecycle`, lignes 1351-1394) :
//! - Bullish Breaker supprimé si `close < bot` (prix passe sous le support).
//! - Bearish Breaker supprimé si `close > top` (prix passe au-dessus de la résistance).
//!
//! Les Breakers sont créés par le `ObDetector` (lors de son lifecycle d'invalidation)
//! via `push_bull` / `push_bear`. Le lifecycle de suppression est assuré par `update`.

use super::fvg::remove_descending;
use super::types::{BarInput, BreakerEvent, BreakerZone};

/// `i_maxBB` (Pine ligne 1082) — Breaker Blocks max par sens.
pub const MAX_BB: usize = 5;

/// Détecteur de Breaker Blocks (bull + bear).
#[derive(Clone)]
pub struct BreakerDetector {
    /// `bbBull*` (Pine) — Bullish Breakers (support, issus d'OB bear invalidés).
    bull: Vec<BreakerZone>,
    /// `bbBear*` (Pine) — Bearish Breakers (résistance, issus d'OB bull invalidés).
    bear: Vec<BreakerZone>,
    /// Breakers créés depuis le dernier `update` (par `push_*`) — flushed dans l'event.
    pending_created: Vec<BreakerZone>,
    last_event: BreakerEvent,
}

impl BreakerDetector {
    pub fn new() -> Self {
        Self {
            bull: Vec::with_capacity(MAX_BB + 1),
            bear: Vec::with_capacity(MAX_BB + 1),
            pending_created: Vec::new(),
            last_event: BreakerEvent::default(),
        }
    }

    /// Crée un Bullish Breaker (Bear OB invalidé par `close > top`).
    /// FIFO 5 : éviction du plus ancien si capacité atteinte.
    /// `top`/`bot` = bornes exactes de l'ancien OB bear (Pine lignes 1287-1288).
    pub fn push_bull(&mut self, top: f64, bot: f64, bar: usize) {
        if self.bull.len() >= MAX_BB {
            self.bull.remove(0);
        }
        let z = BreakerZone {
            top,
            bot,
            bar,
            bull: true,
        };
        self.bull.push(z);
        self.pending_created.push(z);
    }

    /// Crée un Bearish Breaker (Bull OB invalidé par `close < bot`).
    /// `top`/`bot` = bornes exactes de l'ancien OB bull (Pine lignes 1210-1211).
    pub fn push_bear(&mut self, top: f64, bot: f64, bar: usize) {
        if self.bear.len() >= MAX_BB {
            self.bear.remove(0);
        }
        let z = BreakerZone {
            top,
            bot,
            bar,
            bull: false,
        };
        self.bear.push(z);
        self.pending_created.push(z);
    }

    /// Lifecycle (Pine `f_bbLifecycle`) : supprime les breakers complètement traversés.
    /// Retourne l'événement listant les breakers créés depuis le précédent `update`.
    pub fn update(&mut self, bar: &BarInput) -> BreakerEvent {
        // Bullish Breaker : supprimé si close < bot (Pine ligne 1357).
        let mut del_bull = Vec::new();
        for i in 0..self.bull.len() {
            if bar.close < self.bull[i].bot {
                del_bull.push(i);
            }
        }
        remove_descending(&mut self.bull, &del_bull);

        // Bearish Breaker : supprimé si close > top (Pine ligne 1379).
        let mut del_bear = Vec::new();
        for i in 0..self.bear.len() {
            if bar.close > self.bear[i].top {
                del_bear.push(i);
            }
        }
        remove_descending(&mut self.bear, &del_bear);

        let created = std::mem::take(&mut self.pending_created);
        let ev = BreakerEvent { created };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> BreakerEvent {
        self.last_event.clone()
    }
    pub fn bull_zones(&self) -> &[BreakerZone] {
        &self.bull
    }
    pub fn bear_zones(&self) -> &[BreakerZone] {
        &self.bear
    }
}

impl Default for BreakerDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(c: f64) -> BarInput {
        BarInput {
            timestamp: 0,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 0.0,
        }
    }

    #[test]
    fn push_puis_lifecycle_supprime_bull_breaker_si_close_sous_bot() {
        let mut det = BreakerDetector::new();
        det.push_bull(110.0, 100.0, 5); // support bull bot=100
        assert_eq!(det.bull_zones().len(), 1);
        let ev = det.update(&bar(105.0));
        assert_eq!(det.bull_zones().len(), 1, "close=105 > bot=100 ⇒ conservé");
        assert_eq!(ev.created.len(), 1, "breaker créé flushé dans l'event");
        // close sous bot ⇒ supprimé.
        det.update(&bar(99.0));
        assert_eq!(det.bull_zones().len(), 0, "close<bot ⇒ supprimé");
    }

    #[test]
    fn bear_breaker_supprime_si_close_au_dessus_top() {
        let mut det = BreakerDetector::new();
        det.push_bear(110.0, 100.0, 5); // résistance bear top=110
        det.update(&bar(105.0));
        assert_eq!(det.bear_zones().len(), 1, "close=105 < top=110 ⇒ conservé");
        det.update(&bar(111.0));
        assert_eq!(det.bear_zones().len(), 0, "close>top ⇒ supprimé");
    }

    #[test]
    fn fifo_limite_a_5_par_sens() {
        let mut det = BreakerDetector::new();
        for _ in 0..7 {
            det.push_bull(110.0, 100.0, 1);
        }
        assert_eq!(det.bull_zones().len(), MAX_BB, "plafonné à 5 bull");
        det.update(&bar(105.0));
        assert_eq!(det.bull_zones().len(), MAX_BB, "aucun traversé ⇒ inchangé");
    }
}
