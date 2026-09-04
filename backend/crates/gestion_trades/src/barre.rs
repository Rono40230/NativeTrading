//! Barre d'entrée du moteur de gestion — commune à toutes les stratégies.

/// Barre OHLCV (Pine `open`/`high`/`low`/`close`/`volume`). Le lifecycle
/// n'utilise que `timestamp`/`high`/`low`/`close` — le reste est porté pour
/// la commodité des appelants.
#[derive(Debug, Clone, Copy)]
pub struct BarInput {
    /// Unix secondes (ouverture de la bar).
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl BarInput {
    /// Construit une bar à partir de ses prix bruts (timestamp=0, volume=0).
    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            timestamp: 0,
            open,
            high,
            low,
            close,
            volume: 0.0,
        }
    }
}
