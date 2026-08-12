//! BOS (Break of Structure) haussier/baissier + anti-doublon.
//!
//! Reproduit MODULE 2 Pine (lignes 437-450). Squelette Task 1 — implémentation réelle en Task 5.

use super::pivots::PivotDetector;
use super::structure::StructureDetector;
use super::types::{BarInput, BosEvent};

/// Détecteur de BOS. Stub Task 1.
pub struct BosDetector {
    last_event: BosEvent,
}

impl BosDetector {
    pub fn new() -> Self {
        Self {
            last_event: BosEvent::default(),
        }
    }

    pub fn update(
        &mut self,
        _bar: &BarInput,
        _pivots: &PivotDetector,
        _structure: &StructureDetector,
    ) {
    }

    pub fn last_event(&self) -> BosEvent {
        self.last_event.clone()
    }
}

impl Default for BosDetector {
    fn default() -> Self {
        Self::new()
    }
}
