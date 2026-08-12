//! ATR14 (Wilder) — équivalent `ta.atr(14)` du Pine (ligne 421).
//!
//! Squelette Task 1 — implémentation réelle en Task 2.

use super::types::BarInput;

/// ATR14 (lissage Wilder). Stub Task 1.
pub struct Atr14 {
    value: f64,
}

impl Atr14 {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }

    pub fn update(&mut self, _bar: &BarInput) {}

    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for Atr14 {
    fn default() -> Self {
        Self::new()
    }
}
