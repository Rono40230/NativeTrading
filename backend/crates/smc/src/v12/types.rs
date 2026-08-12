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

/// BOS (Break of Structure) détecté à la bar courante (BRUT, pré-MSS).
///
/// `bullish`/`bearish` reflètent `bosHaussier`/`bosBaissier` du Pine (ligne 437-438),
/// SANS le masque `not mssHaussier`. Le masque est appliqué dans `SmcOutput::bos`
/// (cf. MODULE 3, concern 2.0 #2) : un BOS qui est aussi un MSS n'est pas exposé
/// comme BOS afin d'éviter le double-compte downstream.
#[derive(Debug, Clone, Default)]
pub struct BosEvent {
    pub bullish: bool,
    pub bearish: bool,
    pub level: Option<f64>,
    pub bar_index: Option<usize>,
}

/// MSS / CHOCH (MODULE 3 Pine, lignes 452-527).
///
/// - MSS = premier BOS contre la tendance dominante (alerte précoce, non confirmé).
/// - CHOCH = MSS pending + nouveau swing confirmé dans le nouveau sens (HL bull / LH bear).
#[derive(Debug, Clone, Default)]
pub struct MssEvent {
    pub mss_haussier: bool,
    pub mss_baissier: bool,
    pub choch_haussier: bool,
    pub choch_baissier: bool,
    pub mss_level: Option<f64>,
    pub mss_bar: Option<usize>,
    pub mss_dir: Option<MssDir>,
    pub choch_level: Option<f64>,
    pub choch_bar: Option<usize>,
    pub choch_dir: Option<MssDir>,
    /// `_mssHPending` (Pine) — MSS haussier déclenché, en attente d'un HL de confirmation.
    pub mss_h_pending: bool,
    /// `_mssBPending` (Pine) — MSS baissier déclenché, en attente d'un LH de confirmation.
    pub mss_b_pending: bool,
}

/// Sens d'un MSS/CHOCH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MssDir {
    Haussier,
    Baissier,
}

/// Liquidités (MODULE 4 Pine, lignes 163-728).
///
/// Combine les niveaux précédents Day/Week (PDH/PDL/PWH/PWL) avec leur sweep brut,
/// et les égalités EQH/EQL (pool `LiqLevel`).
#[derive(Debug, Clone, Default)]
pub struct LiquiditeEvent {
    // --- Niveaux précédents (bruts, Pine request.security high[1]/low[1]) ---
    pub pdh: Option<f64>,
    pub pdl: Option<f64>,
    pub pwh: Option<f64>,
    pub pwl: Option<f64>,
    // --- Niveaux actifs (invalidés après sweep) ---
    pub pdh_active: Option<f64>,
    pub pdl_active: Option<f64>,
    pub pwh_active: Option<f64>,
    pub pwl_active: Option<f64>,
    // --- Sweeps bruts (high > niveau ET close revers) ---
    pub sweep_pdh: bool,
    pub sweep_pdl: bool,
    pub sweep_pwh: bool,
    pub sweep_pwl: bool,
    // --- EQH/EQL ---
    pub is_eqh: bool,
    pub is_eql: bool,
    pub dernier_eqh_level: Option<f64>,
    pub dernier_eql_level: Option<f64>,
    /// Nombre de niveaux EQH/EQL actifs (non sweepés) dans le pool.
    pub nb_liq_levels: usize,
    /// Nombre de niveaux sweepés dans le pool (contexte historique).
    pub nb_liq_swept: usize,
}

/// Sweep (MODULE 5 Pine, lignes 730-816) — machine 5 phases.
///
/// Phases : armé → expire → confirmé → consommé → fraîcheur.
#[derive(Debug, Clone, Default)]
pub struct SweepEvent {
    /// Sweep haussier confirmé (close > sweep_h_level après armement sur EQL).
    pub sweep_haussier: bool,
    /// Sweep baissier confirmé (close < sweep_b_level après armement sur EQH).
    pub sweep_baissier: bool,
    pub sweep_h_level: Option<f64>,
    pub sweep_h_bar: Option<usize>,
    pub sweep_b_level: Option<f64>,
    pub sweep_b_bar: Option<usize>,
    /// Armement en cours (sweepH_bar/sweepB_bar non-na, pas encore confirmé ni expiré).
    pub sweep_h_armed: bool,
    pub sweep_b_armed: bool,
    /// Dernier sweep haussier confirmé (level/bar) — pour scoring.
    pub dernier_sweep_h_level: Option<f64>,
    pub dernier_sweep_h_bar: Option<usize>,
    pub dernier_sweep_b_level: Option<f64>,
    pub dernier_sweep_b_bar: Option<usize>,
    /// Fraîcheur (≤ SWEEP_FRESH_BARS) — Phase 5.1 Pine.
    pub sweep_bull_frais: bool,
    pub sweep_bear_frais: bool,
    /// Fenêtre de fraîcheur courante (barres).
    pub sweep_fresh_bars: i64,
}

/// Sortie complète du moteur pour une bar.
#[derive(Debug, Clone, Default)]
pub struct SmcOutput {
    pub atr14: f64,
    pub pivot: PivotEvent,
    pub structure: StructureEvent,
    /// BOS **masqué** : `bosHaussier and not mssHaussier` (Pine lignes 524-527, 540).
    /// Un BOS qui est aussi un MSS n'apparaît pas ici.
    pub bos: BosEvent,
    pub mss: MssEvent,
    pub liquidite: LiquiditeEvent,
    pub sweep: SweepEvent,
    /// Dernier swing high (sh1).
    pub sh1: Option<f64>,
    /// Dernier swing low (sl1).
    pub sl1: Option<f64>,
    /// Tendance **pré-reset MSS** (fidélité Pine : `tendanceHaussiere` est calculé
    /// ligne 381 avant la réinitialisation MSS ligne 504).
    pub tendance_haussiere: bool,
    pub tendance_baissiere: bool,
}
