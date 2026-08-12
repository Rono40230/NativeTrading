//! Structure HH/HL/LH/LL + compteurs bull/bear + tendance.
//!
//! Reproduit MODULE 1 Pine (lignes 368-382) :
//!   isHH = ph > sh2 ; isLH = ph < sh2 ; isHL = pl > sl2 ; isLL = pl < sl2
//!   HH|HL → bullCount++ ET bearCount = max(0, bearCount-1)
//!   LH|LL → bearCount++ ET bullCount = max(0, bullCount-1)
//!   tendanceHaussiere = bullCount >= 2 ; tendanceBaissiere = bearCount >= 2
//!
//! Le détecteur maintient son propre `prev_ph`/`prev_pl` (équivalent au sh2/sl2
//! au moment de la classification). Comme le PivotDetector émet les mêmes événements
//! dans le même ordre, les deux restent cohérents.

use super::types::{BarInput, PivotEvent, StructureEvent};

/// Détecteur de structure (HH/HL/LH/LL) + tendance dérivée.
pub struct StructureDetector {
    bull_count: u32,
    bear_count: u32,
    /// Pivot high précédent (sh2 au moment de la classif).
    prev_ph: Option<f64>,
    /// Pivot low précédent (sl2 au moment de la classif).
    prev_pl: Option<f64>,
    last_event: StructureEvent,
}

impl StructureDetector {
    pub fn new() -> Self {
        Self {
            bull_count: 0,
            bear_count: 0,
            prev_ph: None,
            prev_pl: None,
            last_event: StructureEvent::default(),
        }
    }

    pub fn update(&mut self, _bar: &BarInput, pivot: &PivotEvent) {
        let mut ev = StructureEvent::default();

        if pivot.is_pivot_high {
            if let Some(p) = pivot.pivot_high_price {
                if let Some(prev) = self.prev_ph {
                    if p > prev {
                        ev.is_hh = true;
                    } else if p < prev {
                        ev.is_lh = true;
                    }
                    // p == prev ⇒ ni HH ni LH (égalité parfaite, rare).
                }
                self.prev_ph = Some(p);
            }
        }
        if pivot.is_pivot_low {
            if let Some(p) = pivot.pivot_low_price {
                if let Some(prev) = self.prev_pl {
                    if p > prev {
                        ev.is_hl = true;
                    } else if p < prev {
                        ev.is_ll = true;
                    }
                }
                self.prev_pl = Some(p);
            }
        }

        // Compteurs tendance (Pine lignes 375-380) : incrément + décrément saturé du opposé.
        if ev.is_hh || ev.is_hl {
            self.bull_count = self.bull_count.saturating_add(1);
            self.bear_count = self.bear_count.saturating_sub(1);
        }
        if ev.is_lh || ev.is_ll {
            self.bear_count = self.bear_count.saturating_add(1);
            self.bull_count = self.bull_count.saturating_sub(1);
        }

        ev.bull_count = self.bull_count;
        ev.bear_count = self.bear_count;
        ev.tendance_haussiere = self.bull_count >= 2;
        ev.tendance_baissiere = self.bear_count >= 2;
        self.last_event = ev;
    }

    pub fn last_event(&self) -> StructureEvent {
        self.last_event.clone()
    }
    pub fn tendance_haussiere(&self) -> bool {
        self.bull_count >= 2
    }
    pub fn tendance_baissiere(&self) -> bool {
        self.bear_count >= 2
    }
}

impl Default for StructureDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_bar() -> BarInput {
        BarInput::new(100.0, 101.0, 99.0, 100.0)
    }

    fn pivot_high(p: f64) -> PivotEvent {
        PivotEvent {
            is_pivot_high: true,
            is_pivot_low: false,
            pivot_high_price: Some(p),
            pivot_low_price: None,
            pivot_bar_index: Some(0),
        }
    }

    fn pivot_low(p: f64) -> PivotEvent {
        PivotEvent {
            is_pivot_high: false,
            is_pivot_low: true,
            pivot_high_price: None,
            pivot_low_price: Some(p),
            pivot_bar_index: Some(0),
        }
    }

    #[test]
    fn premier_pivot_high_ne_classifie_rien() {
        let mut s = StructureDetector::new();
        s.update(&dummy_bar(), &pivot_high(100.0));
        let ev = s.last_event();
        assert!(!ev.is_hh && !ev.is_lh);
        assert_eq!(ev.bull_count, 0);
        assert!(!s.tendance_haussiere());
    }

    #[test]
    fn deux_hh_consecutifs_declenchent_tendance_haussiere() {
        let mut s = StructureDetector::new();
        s.update(&dummy_bar(), &pivot_high(100.0)); // init, pas de classif
        s.update(&dummy_bar(), &pivot_high(110.0)); // HH
        assert!(s.last_event().is_hh);
        assert!(!s.tendance_haussiere(), "1 HH ⇒ pas encore tendance (bull=1)");
        s.update(&dummy_bar(), &pivot_high(120.0)); // HH
        assert!(s.last_event().is_hh);
        assert!(s.tendance_haussiere(), "2 HH ⇒ tendance haussière");
        assert!(!s.tendance_baissiere());
    }

    #[test]
    fn deux_lh_consecutifs_declenchent_tendance_baissiere() {
        let mut s = StructureDetector::new();
        s.update(&dummy_bar(), &pivot_high(120.0)); // init
        s.update(&dummy_bar(), &pivot_high(110.0)); // LH
        s.update(&dummy_bar(), &pivot_high(100.0)); // LH
        assert!(s.last_event().is_lh);
        assert!(s.tendance_baissiere());
        assert!(!s.tendance_haussiere());
    }

    #[test]
    fn compteur_croise_decrement_sature_l_oppose() {
        // Hausses puis baisses : la tendance doit basculer (Pine lignes 375-380).
        let mut s = StructureDetector::new();
        s.update(&dummy_bar(), &pivot_high(100.0)); // init
        s.update(&dummy_bar(), &pivot_high(110.0)); // HH → bull=1
        s.update(&dummy_bar(), &pivot_high(120.0)); // HH → bull=2 (tendance haussière)
        assert!(s.tendance_haussiere());
        s.update(&dummy_bar(), &pivot_high(115.0)); // LH → bear=1, bull=1
        s.update(&dummy_bar(), &pivot_high(105.0)); // LH → bear=2, bull=0
        assert!(s.tendance_baissiere(), "2 LH consécutifs ⇒ bascule baissière");
        assert!(!s.tendance_haussiere(), "bull remis à 0 par décrément saturé");
    }

    #[test]
    fn hl_et_ll_via_pivots_low() {
        let mut s = StructureDetector::new();
        s.update(&dummy_bar(), &pivot_low(100.0)); // init
        s.update(&dummy_bar(), &pivot_low(110.0)); // HL
        assert!(s.last_event().is_hl);
        s.update(&dummy_bar(), &pivot_low(95.0)); // LL
        assert!(s.last_event().is_ll);
    }
}
