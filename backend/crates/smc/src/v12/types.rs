//! Types partagés du moteur SMC v12.
//!
//! Reproduit les structures de données implicites du Pine `smc_indicateur_v12.pine`.

pub use gestion_trades::barre::BarInput;

/// Une bar OHLCV clôturée (équivalent Pine `barstate.isconfirmed`).
///
/// En replay/backtest, on ne traite QUE des bars clôturées : pas de logique intrabar,
/// pas de repaint.


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
    // --- Timestamps d'origine (bord gauche des lignes, Pine _prevDayHighTime) ---
    pub pdh_ts: Option<i64>,
    pub pdl_ts: Option<i64>,
    pub pwh_ts: Option<i64>,
    pub pwl_ts: Option<i64>,
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

// ============================================================================
// MODULE 6 — FVG (Fair Value Gap)
// ============================================================================

/// État d'un FVG (Pine `fvgBullState`/`fvgBearState`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FvgState {
    /// `0` — frais, non encore touché.
    #[default]
    Fresh,
    /// `1` — partiellement touché (`low < topB` pour un FVG bull).
    Partial,
}

/// Une zone FVG active (Pine : 5 arrays parallèles `fvg{Bull,Bear}{Top,Bot,State,Bar}`).
#[derive(Debug, Clone, Copy)]
pub struct FvgZone {
    /// `topB` (bull) / `topBr` (bear) — bord supérieur du gap.
    pub top: f64,
    /// `botB` (bull) / `botBr` (bear) — bord inférieur du gap.
    pub bot: f64,
    pub state: FvgState,
    /// `fvgBullBar` (Pine) = `bar_index[2]` à la création.
    pub bar: usize,
}

/// Événement FVG pour une bar (MODULE 6 Pine, lignes 818-973).
#[derive(Debug, Clone)]
pub struct FvgEvent {
    /// `isFVGBull` (Pine ligne 833) — détection courante.
    pub is_fvg_bull: bool,
    /// `isFVGBear` (Pine ligne 834).
    pub is_fvg_bear: bool,
    /// Bornes du FVG bull courant (`low` / `high[2]`) — pour Propulsion.
    pub bull_top: f64,
    pub bull_bot: f64,
    /// Bornes du FVG bear courant (`low[2]` / `high`).
    pub bear_top: f64,
    pub bear_bot: f64,
    /// Nouveau FVG bull créé cette bar (None si rien).
    pub new_bull: Option<FvgZone>,
    /// Nouveau FVG bear créé cette bar.
    pub new_bear: Option<FvgZone>,
}

impl Default for FvgEvent {
    fn default() -> Self {
        Self {
            is_fvg_bull: false,
            is_fvg_bear: false,
            bull_top: 0.0,
            bull_bot: 0.0,
            bear_top: 0.0,
            bear_bot: 0.0,
            new_bull: None,
            new_bear: None,
        }
    }
}

// ============================================================================
// MODULE 7 — ORDER BLOCKS
// ============================================================================

/// État d'un OB (Pine `obBullState`/`obBearState`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObState {
    /// `0` — vierge.
    #[default]
    Vierge,
    /// `1` — partiel (`close > mid` bull / `close < mid` bear).
    Partiel,
    /// `2` — profond (`close <= mid` bull / `close >= mid` bear).
    Profond,
}

/// Un Order Block actif (Pine : 11 arrays parallèles `ob{Bull,Bear}*`).
#[derive(Debug, Clone, Copy)]
pub struct ObZone {
    /// `topB` = `high[1]` (bougie précédant l'impulsion).
    pub top: f64,
    /// `botB` = `low[1]`.
    pub bot: f64,
    pub state: ObState,
    /// `obBullBar` (Pine) = `bar_index` de l'impulsion — garde anti-suppression.
    pub impulse_bar: usize,
    /// `bar_index[1]` — bougie OB réelle.
    pub ob_bar: usize,
    /// `int(time[1])` (Pine).
    pub timestamp: i64,
    /// `ibBull[1]` / `ibBear[1]` (Pine) — la bougie OB était-elle une imbalance ?
    pub is_ib: bool,
}

/// Événement OB pour une bar (MODULE 7 Pine, lignes 1016-1337).
#[derive(Debug, Clone, Default)]
pub struct ObEvent {
    /// Nouvel OB bull créé cette bar.
    pub new_bull: Option<ObZone>,
    /// Nouvel OB bear créé cette bar.
    pub new_bear: Option<ObZone>,
    /// OBs bulls invalidés (supprimés) cette bar — peuvent avoir spawné un Breaker.
    pub invalidated_bull: Vec<ObZone>,
    /// OBs bears invalidés cette bar.
    pub invalidated_bear: Vec<ObZone>,
}

// ============================================================================
// MODULE 8b — BREAKER BLOCKS
// ============================================================================

/// Un Breaker Block (Pine : `bb{Bull,Bear}{Top,Bot}`).
///
/// - **Bullish Breaker** (`bbBull`) = Bear OB invalidé par `close > top` → support.
/// - **Bearish Breaker** (`bbBear`) = Bull OB invalidé par `close < bot` → résistance.
#[derive(Debug, Clone, Copy)]
pub struct BreakerZone {
    pub top: f64,
    pub bot: f64,
    /// `bar_index` de création (Pine `box.new(bar_index, ...)`).
    pub bar: usize,
    /// `true` = Bullish Breaker (`bbBull`), `false` = Bearish Breaker (`bbBear`).
    pub bull: bool,
}

/// Événement Breaker pour une bar (MODULE 8b Pine, lignes 1078-1397).
#[derive(Debug, Clone, Default)]
pub struct BreakerEvent {
    /// Nouveaux breakers créés cette bar (par invalidation OB).
    pub created: Vec<BreakerZone>,
}

// ============================================================================
// MODULE 8c — PROPULSION BLOCKS
// ============================================================================

/// Un Propulsion Block = chevauchement FVG ∩ OB (Pine `prop{Bull,Bear}{Top,Bot}`).
///
/// `top = min(fTop, oTop)`, `bot = max(fBot, oBot)` avec `top > bot`.
#[derive(Debug, Clone, Copy)]
pub struct PropulsionZone {
    pub top: f64,
    pub bot: f64,
    /// `bar_index` de création.
    pub bar: usize,
    /// `true` = Propulsion bull, `false` = Propulsion bear.
    pub bull: bool,
}

/// Événement Propulsion pour une bar (MODULE 8c Pine, lignes 1398-1518).
#[derive(Debug, Clone, Default)]
pub struct PropulsionEvent {
    pub new_bull: Vec<PropulsionZone>,
    pub new_bear: Vec<PropulsionZone>,
}

// ============================================================================
// MODULE 13b — IMBALANCE
// ============================================================================

/// État d'une Imbalance (Pine `ib{Bull,Bear}State`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImbalanceState {
    /// `0` — frais.
    #[default]
    Fresh,
    /// `1` — partiellement mitigé.
    Partial,
}

/// Une zone d'Imbalance active (Pine : `ib{Bull,Bear}{Top,Bot,State}`).
///
/// Box = corps de la bougie : bull `top=close, bot=open` ; bear `top=open, bot=close`.
#[derive(Debug, Clone, Copy)]
pub struct ImbalanceZone {
    pub top: f64,
    pub bot: f64,
    pub state: ImbalanceState,
    /// `bar_index` de création.
    pub bar: usize,
    /// `true` = imbalance bull, `false` = bear.
    pub bull: bool,
}

/// Événement Imbalance pour une bar (MODULE 13b Pine, lignes 2578-2702).
#[derive(Debug, Clone, Default)]
pub struct ImbalanceEvent {
    /// `ibBull` (Pine ligne 432) — flag de la bar courante.
    pub ib_bull: bool,
    /// `ibBear` (Pine ligne 433).
    pub ib_bear: bool,
    pub new_bull: Option<ImbalanceZone>,
    pub new_bear: Option<ImbalanceZone>,
}

// ============================================================================
// MODULE 4b — PREMIUM / DISCOUNT (Equilibrium ICT)
// ============================================================================

/// Événement Premium/Discount (MODULE 4b Pine, lignes 1654-1691).
///
/// Plage de référence = dernier dealing range BOS (sh1/sl1 capturés au BOS).
#[derive(Debug, Clone, Default)]
pub struct PdEvent {
    /// `inPremium` — close > equilibrium + tolérance.
    pub in_premium: bool,
    /// `inDiscount` — close < equilibrium - tolérance.
    pub in_discount: bool,
    /// `pdEquilibrium = (_pdRangeH + _pdRangeL) / 2`.
    pub equilibrium: Option<f64>,
    /// `_pdRangeH` = sh1 au dernier BOS.
    pub pd_range_h: Option<f64>,
    /// `_pdRangeL` = sl1 au dernier BOS.
    pub pd_range_l: Option<f64>,
}

// ============================================================================
// MODULE 13c — FIBONACCI OTE (Optimal Trade Entry)
// ============================================================================

/// Événement OTE (MODULE 13c Pine, lignes 2022-2110).
///
/// Zone OTE = Fibonacci 61.8 % - 78.6 % du dernier dealing range BOS.
#[derive(Debug, Clone, Default)]
pub struct OteEvent {
    /// `inOTE_bull` — close ∈ [_oteBotBull, _oteTopBull].
    pub in_ote_bull: bool,
    /// `inOTE_bear` — close ∈ [_oteBotBear, _oteTopBear].
    pub in_ote_bear: bool,
    pub bull_top: Option<f64>,
    pub bull_bot: Option<f64>,
    pub bear_top: Option<f64>,
    pub bear_bot: Option<f64>,
    /// `OTE_EXPIRY_BARS = max(1, round(10800/_tfSec))` (12 en M15).
    pub expiry_bars: i64,
    /// Box d'affichage bull (Pine `_oteBullBox`, lignes 2126-2148) :
    /// `(top, bot, ts_bos)`. Créée au BOS avec les bornes du moment, remplacée
    /// à chaque BOS ; **persiste après expiration de la plage Fib** (Pine :
    /// suppression `close < _oteBotBull` lit les bornes COURANTES → impossible
    /// une fois la plage expirée `na`).
    pub bull_box: Option<(f64, f64, i64)>,
    /// Box d'affichage bear (Pine `_oteBearBox`) : `(top, bot, ts_bos)`.
    pub bear_box: Option<(f64, f64, i64)>,
}

// ============================================================================
// KILL ZONES (Pine lignes 124-161)
// ============================================================================

/// Kill Zone active à la bar courante (UTC).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KillZone {
    #[default]
    None,
    Asian,
    London,
    NyAm,
    NyPm,
}

/// Événement Kill Zone (time-based, UTC minutes depuis minuit).
#[derive(Debug, Clone, Default)]
pub struct KillZoneEvent {
    /// `inKZ` — OU des 4 plages UTC.
    pub in_kz: bool,
    pub zone: KillZone,
    /// `_gKzMins = (timestamp % 86400) / 60` — minutes UTC depuis minuit.
    pub mins: i64,
}

// ============================================================================
// MODULE 10b — NDOG / NWOG (New Day / New Week Opening Gaps)
// ============================================================================

/// Un gap d'ouverture NDOG (jour) ou NWOG (semaine) — MODULE 10b Pine (1533-1652).
#[derive(Debug, Clone, Copy)]
pub struct GapZone {
    /// `_gTop = max(open, close[1])`.
    pub top: f64,
    /// `_gBot = min(open, close[1])`.
    pub bot: f64,
    /// Mitigé quand `low <= bot and high >= top` (prix remplit le gap).
    pub mitigated: bool,
    /// `bar_index` de création.
    pub bar: usize,
    /// `false` = NDOG (jour), `true` = NWOG (semaine).
    pub is_week: bool,
}

/// Événement NDOG/NWOG pour une bar.
#[derive(Debug, Clone, Default)]
pub struct NdogEvent {
    /// Nouveau NDOG créé cette bar (gating TF M1–M15).
    pub new_ndog: Option<GapZone>,
    /// Nouveau NWOG créé cette bar (gating TF H1–H4).
    pub new_nwog: Option<GapZone>,
}

// ============================================================================
// MODULE 12 — MULTI-TIMEFRAME (MTF)
// ============================================================================

/// Un Order Block HTF (Pine `_b1T/_b1B` etc.) — borne + timestamp de création.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HtfObZone {
    pub top: f64,
    pub bot: f64,
    /// `_b1time`/`_r1time` (Pine) — `time` de la bougie OB HTF.
    pub timestamp: i64,
}

/// État MTF d'un TF supérieur (trend + 3 derniers OB bull + 3 derniers OB bear).
#[derive(Debug, Clone, Default)]
pub struct HtfState {
    /// `_trend` Pine : 1 = bull, -1 = bear, 0 = neutre.
    pub trend: i32,
    /// 3 derniers OB bull (`_b1.._b3`), index 0 = plus récent. Peut contenir < 3.
    pub bull_obs: Vec<HtfObZone>,
    /// 3 derniers OB bear (`_r1.._r3`), index 0 = plus récent.
    pub bear_obs: Vec<HtfObZone>,
}

/// Événement MTF pour une bar (MODULE 12 Pine, lignes 1693-1878).
///
/// ⚠️ **REPAINT assumé** : les OB/confluences HTF reflètent la bougie HTF en cours
/// (équivalent Pine `request.security(..., lookahead_off)` live).
#[derive(Debug, Clone, Default)]
pub struct MtfEvent {
    /// `confluenceH1` — close ∈ au moins une des 6 zones OB H1.
    pub confluence_h1: bool,
    pub confluence_h4: bool,
    pub confluence_w1: bool,
    pub confluence_mn: bool,
    /// R5 (étude étape 3) : containment DIRECTIONNEL — close ∈ une zone OB du
    /// sens indiqué. Le flag historique reste direction-agnostique (parité
    /// Pine) ; l'étude comparatif passe_finale mesure la correction.
    pub confluence_h1_bull: bool,
    pub confluence_h1_bear: bool,
    pub confluence_h4_bull: bool,
    pub confluence_h4_bear: bool,
    pub confluence_w1_bull: bool,
    pub confluence_w1_bear: bool,
    pub confluence_mn_bull: bool,
    pub confluence_mn_bear: bool,
    pub h1: HtfState,
    pub h4: HtfState,
    pub w1: HtfState,
    pub mn: HtfState,
}

// ============================================================================
// ZONE-CŒUR (Pine lignes 2112-2154)
// ============================================================================

/// Une zone-cœur = intersection OB ∩ OTE ∩ 1er FVG chevauchant (Pine `f_coeurBull/Bear`).
#[derive(Debug, Clone, Copy)]
pub struct ZoneCoeurZone {
    pub top: f64,
    pub bot: f64,
    /// `ob_bar` de l'OB à l'origine de la zone-cœur.
    pub ob_bar: usize,
    /// `true` = zone-cœur bull (Discount), `false` = bear (Premium).
    pub bull: bool,
}

/// Événement Zone-cœur pour une bar.
#[derive(Debug, Clone, Default)]
pub struct ZoneCoeurEvent {
    pub bull: Vec<ZoneCoeurZone>,
    pub bear: Vec<ZoneCoeurZone>,
    /// Boxes **live** (Pine `f_zoneCoeurLifecycle`) : créées au premier setup
    /// valide (bornes figées), supprimées dès invalidation ou disparition de
    /// l'OB parent — c'est la sortie d'affichage (Pine supprime la box).
    pub live_bull: Vec<ZoneCoeurZone>,
    pub live_bear: Vec<ZoneCoeurZone>,
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
    /// BOS **brut** (`bosHaussier`/`bosBaissier` Pine, jamais masqués) — utilisé
    /// par le BE-force du lifecycle (Pine `_beForce = not _t1Hit and bosBaissier`).
    pub bos_raw: BosEvent,
    pub mss: MssEvent,
    pub liquidite: LiquiditeEvent,
    pub sweep: SweepEvent,
    /// MODULE 6 — FVG.
    pub fvg: FvgEvent,
    /// MODULE 7 — Order Blocks.
    pub order_blocks: ObEvent,
    /// MODULE 8b — Breaker Blocks.
    pub breaker: BreakerEvent,
    /// MODULE 8c — Propulsion Blocks.
    pub propulsion: PropulsionEvent,
    /// MODULE 13b — Imbalance.
    pub imbalance: ImbalanceEvent,
    /// MODULE 4b — Premium/Discount (Equilibrium ICT).
    pub premium_discount: PdEvent,
    /// MODULE 13c — Fibonacci OTE.
    pub ote: OteEvent,
    /// Kill Zones (UTC).
    pub kill_zone: KillZoneEvent,
    /// MODULE 10b — NDOG/NWOG.
    pub ndog: NdogEvent,
    /// MODULE 12 — Multi-Timeframe (repaint assumé).
    pub mtf: MtfEvent,
    /// Zone-cœur (intersection OB ∩ OTE ∩ FVG).
    pub zone_coeur: ZoneCoeurEvent,
    /// MODULE 14 — Asian High/Low drawn (DoL znQual + cible TP3).
    pub asian_hl: super::asian_hl::AsianHlEvent,
    /// MODULE 14b — London High/Low drawn (Module F).
    pub london_hl: super::asian_hl::AsianHlEvent,
    /// Dernier swing high (sh1).
    pub sh1: Option<f64>,
    /// Dernier swing low (sl1).
    pub sl1: Option<f64>,
    /// Tendance **pré-reset MSS** (fidélité Pine : `tendanceHaussiere` est calculé
    /// ligne 381 avant la réinitialisation MSS ligne 504).
    pub tendance_haussiere: bool,
    pub tendance_baissiere: bool,
}
