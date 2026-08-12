//! Types partagés du moteur SMC v12.
//!
//! Reproduit les structures de données implicites du Pine `smc_indicateur_v12.pine`.

/// Une bar OHLCV clôturée (équivalent Pine `barstate.isconfirmed`).
///
/// En replay/backtest, on ne traite QUE des bars clôturées : pas de logique intrabar,
/// pas de repaint.
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
        Self { timestamp: 0, open, high, low, close, volume: 0.0 }
    }
}

/// Événement pivot détecté à la bar courante (high et/ou low).
#[derive(Debug, Clone, Default)]
pub struct PivotEvent {
    pub is_pivot_high: bool,
    pub is_pivot_low: bool,
    pub pivot_high_price: Option<f64>,
    pub pivot_low_price: Option<f64>,
    /// Index de la bar pivot (n-sl-1), PAS la bar courante.
    pub pivot_bar_index: Option<usize>,
}

/// Structure : HH/HL/LH/LL + compteurs de tendance.
#[derive(Debug, Clone, Default)]
pub struct StructureEvent {
    pub is_hh: bool,
    pub is_hl: bool,
    pub is_lh: bool,
    pub is_ll: bool,
    pub bull_count: u32,
    pub bear_count: u32,
    pub tendance_haussiere: bool,
    pub tendance_baissiere: bool,
}

/// BOS (Break of Structure) détecté à la bar courante.
#[derive(Debug, Clone, Default)]
pub struct BosEvent {
    pub bullish: bool,
    pub bearish: bool,
    pub level: Option<f64>,
    pub bar_index: Option<usize>,
}

/// Sortie complète du moteur pour une bar.
#[derive(Debug, Clone, Default)]
pub struct SmcOutput {
    pub atr14: f64,
    pub pivot: PivotEvent,
    pub structure: StructureEvent,
    pub bos: BosEvent,
    /// Dernier swing high (sh1).
    pub sh1: Option<f64>,
    /// Dernier swing low (sl1).
    pub sl1: Option<f64>,
    pub tendance_haussiere: bool,
    pub tendance_baissiere: bool,
}
