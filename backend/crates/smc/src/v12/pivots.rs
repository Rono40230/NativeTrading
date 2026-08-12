//! Détection pivots high/low + maintenance sh1/sl1/sh2/sl2.
//!
//! Reproduit MODULE 1 Pine (lignes 314-366) : `ta.pivothigh/low(high/low, swingLength, swingLength)`.
//! Squelette Task 1 — implémentation réelle en Task 3.

use super::types::{BarInput, PivotEvent};

/// Détecteur de pivots (swings). Stub Task 1.
pub struct PivotDetector {
    #[allow(dead_code)]
    swing_length: usize,
    last_event: PivotEvent,
}

impl PivotDetector {
    pub fn new(swing_length: usize) -> Self {
        Self {
            swing_length,
            last_event: PivotEvent::default(),
        }
    }

    pub fn update(&mut self, _bar: &BarInput) {}

    pub fn last_event(&self) -> PivotEvent {
        self.last_event.clone()
    }

    pub fn sh1(&self) -> Option<f64> {
        None
    }
    pub fn sl1(&self) -> Option<f64> {
        None
    }
    pub fn sh2(&self) -> Option<f64> {
        None
    }
    pub fn sl2(&self) -> Option<f64> {
        None
    }

    /// Index de la bar du pivot high courant (Pine `bsh1`). Utilisé par le BOS anti-doublon.
    pub fn last_pivot_high_bar(&self) -> Option<usize> {
        None
    }
    /// Index de la bar du pivot low courant (Pine `bsl1`).
    pub fn last_pivot_low_bar(&self) -> Option<usize> {
        None
    }
}
