//! Structure HH/HL/LH/LL + compteurs bull/bear + tendance.
//!
//! Reproduit MODULE 1 Pine (lignes 368-382). Squelette Task 1 — implémentation réelle en Task 4.

use super::types::{BarInput, PivotEvent, StructureEvent};

/// Détecteur de structure. Stub Task 1.
pub struct StructureDetector {
    last_event: StructureEvent,
}

impl StructureDetector {
    pub fn new() -> Self {
        Self {
            last_event: StructureEvent::default(),
        }
    }

    pub fn update(&mut self, _bar: &BarInput, _pivot: &PivotEvent) {}

    pub fn last_event(&self) -> StructureEvent {
        self.last_event.clone()
    }

    pub fn tendance_haussiere(&self) -> bool {
        false
    }
    pub fn tendance_baissiere(&self) -> bool {
        false
    }
}

impl Default for StructureDetector {
    fn default() -> Self {
        Self::new()
    }
}
