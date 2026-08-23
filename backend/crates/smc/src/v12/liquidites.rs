//! MODULE 4 — Liquidités : PDH/PDL/PWH/PWL + EQH/EQL.
//!
//! Reproduit MODULE 4 Pine (lignes 163-728) :
//!
//! **PDH/PDL/PWH/PWL** (lignes 163-256) : en Pine, les niveaux proviennent de
//! `request.security("D"/"W", high[1]/low[1])`. En Rust hors-plateforme, on
//! reconstitue ces niveaux en trackant les bornes day/week : à chaque changement
//! de jour (semaine), le high/low de la période **précédente** devient pdh/pdl
//! (pwh/pwl), et les niveaux *actifs* sont rafraîchis. Invalidation après sweep
//! (`pdhActive := na`).
//!
//! **EQH/EQL** (lignes 610-694) : égalités de highs/lows avec tolérances ATR
//! (`tolEq = 0.20 × ATR14` pour la détection, `tolCluster = 0.50 × ATR14` pour le
//! regroupement). Pool FIFO de `LiqLevel` (max 20) avec compteur de touches
//! (2 à la création, incrémenté par 3ᵉ touche isolée), prix moyen cumulé, drapeau
//! `swept`.

use super::pivots::PivotDetector;
use super::types::{BarInput, LiquiditeEvent, PivotEvent};

/// Nombre max de niveaux EQH/EQL dans le pool (Pine `_maxLiq = 20`).
const MAX_LIQ: usize = 20;

/// UDT Pine `LiqLevel` : un niveau de liquidité EQH/EQL.
#[derive(Debug, Clone, Copy)]
pub struct LiqLevel {
    /// Niveau moyen (moyenne cumulée des pivots touchés).
    pub price: f64,
    /// `bar_index` du 1ᵉʳ pivot (bord gauche de la ligne).
    pub t_first: usize,
    /// Nombre de pivots touchés (2 = en formation, 3+ = confirmé).
    pub touches: u32,
    /// `true` si le prix a traversé le niveau puis est revenu (sweep).
    pub swept: bool,
    /// `true` = EQH, `false` = EQL.
    pub is_high: bool,
}

/// Détecteur de liquidités (PDH/PDL/PWH/PWL + EQH/EQL).
#[derive(Clone)]
pub struct LiquiditesDetector {
    // --- Tracking day (UTC) ---
    cur_day_key: Option<i64>,
    cur_day_high: f64,
    cur_day_low: f64,
    /// `pdh` = previous day high (Pine request.security "D" high[1]).
    pdh: Option<f64>,
    /// Timestamp de la bougie où le pdh s'est formé (bord gauche ligne, Pine
    /// `_prevDayHighTime`, fallback début du jour précédent).
    pub pdh_ts: Option<i64>,
    pub pdl_ts: Option<i64>,
    pub pwh_ts: Option<i64>,
    pub pwl_ts: Option<i64>,
    cur_day_high_ts: i64,
    cur_day_low_ts: i64,
    cur_week_high_ts: i64,
    cur_week_low_ts: i64,
    /// `pdl` = previous day low.
    pdl: Option<f64>,
    pdh_active: Option<f64>,
    pdl_active: Option<f64>,

    // --- Tracking week (ISO) ---
    cur_week_key: Option<(i32, u32)>,
    cur_week_high: f64,
    cur_week_low: f64,
    /// `pwh` = previous week high.
    pwh: Option<f64>,
    /// `pwl` = previous week low.
    pwl: Option<f64>,
    pwh_active: Option<f64>,
    pwl_active: Option<f64>,

    // --- EQH/EQL ---
    /// `liqPool` (Pine) — pool FIFO de niveaux EQH/EQL.
    pool: Vec<LiqLevel>,
    /// `dernierEQH_level` (Pine) — niveau EQH le plus récent.
    dernier_eqh_level: Option<f64>,
    /// `dernierEQL_level` (Pine) — niveau EQL le plus récent.
    dernier_eql_level: Option<f64>,

    last_event: LiquiditeEvent,
}

impl LiquiditesDetector {
    pub fn new() -> Self {
        Self {
            cur_day_key: None,
            cur_day_high: 0.0,
            cur_day_low: 0.0,
            pdh: None,
            pdh_ts: None,
            pdl_ts: None,
            pwh_ts: None,
            pwl_ts: None,
            cur_day_high_ts: 0,
            cur_day_low_ts: 0,
            cur_week_high_ts: 0,
            cur_week_low_ts: 0,
            pdl: None,
            pdh_active: None,
            pdl_active: None,
            cur_week_key: None,
            cur_week_high: 0.0,
            cur_week_low: 0.0,
            pwh: None,
            pwl: None,
            pwh_active: None,
            pwl_active: None,
            pool: Vec::new(),
            dernier_eqh_level: None,
            dernier_eql_level: None,
            last_event: LiquiditeEvent::default(),
        }
    }

    /// Traite une bar. `atr14` doit être la valeur ATR14 courante.
    pub fn update(
        &mut self,
        bar: &BarInput,
        pivots: &PivotDetector,
        pivot_event: &PivotEvent,
        atr14: f64,
    ) -> LiquiditeEvent {
        // ── 1. Tracking daily / weekly boundaries (remplace request.security) ──
        let dk = day_key(bar.timestamp);
        match self.cur_day_key {
            None => {
                // Première bar : initialise le jour courant (pas de pdh encore).
                self.cur_day_key = Some(dk);
                self.cur_day_high = bar.high;
                self.cur_day_low = bar.low;
                self.cur_day_high_ts = bar.timestamp;
                self.cur_day_low_ts = bar.timestamp;
            }
            Some(prev_dk) if prev_dk != dk => {
                // Changement de jour : la période précédente devient pdh/pdl.
                self.pdh = Some(self.cur_day_high);
                self.pdl = Some(self.cur_day_low);
                self.pdh_ts = Some(self.cur_day_high_ts);
                self.pdl_ts = Some(self.cur_day_low_ts);
                self.pdh_active = self.pdh;
                self.pdl_active = self.pdl;
                self.cur_day_key = Some(dk);
                self.cur_day_high = bar.high;
                self.cur_day_low = bar.low;
                self.cur_day_high_ts = bar.timestamp;
                self.cur_day_low_ts = bar.timestamp;
            }
            _ => {
                // Même jour : étend le high/low du jour courant.
                if bar.high > self.cur_day_high {
                    self.cur_day_high = bar.high;
                    self.cur_day_high_ts = bar.timestamp;
                }
                if bar.low < self.cur_day_low {
                    self.cur_day_low = bar.low;
                    self.cur_day_low_ts = bar.timestamp;
                }
            }
        }

        // Semaine (ISO) — Pine `weekofyear != weekofyear[1]`.
        let wk = week_key(bar.timestamp);
        match self.cur_week_key {
            None => {
                self.cur_week_key = Some(wk);
                self.cur_week_high = bar.high;
                self.cur_week_low = bar.low;
                self.cur_week_high_ts = bar.timestamp;
                self.cur_week_low_ts = bar.timestamp;
            }
            Some(prev_wk) if prev_wk != wk => {
                self.pwh = Some(self.cur_week_high);
                self.pwl = Some(self.cur_week_low);
                self.pwh_ts = Some(self.cur_week_high_ts);
                self.pwl_ts = Some(self.cur_week_low_ts);
                self.pwh_active = self.pwh;
                self.pwl_active = self.pwl;
                self.cur_week_key = Some(wk);
                self.cur_week_high = bar.high;
                self.cur_week_low = bar.low;
                self.cur_week_high_ts = bar.timestamp;
                self.cur_week_low_ts = bar.timestamp;
            }
            _ => {
                if bar.high > self.cur_week_high {
                    self.cur_week_high = bar.high;
                    self.cur_week_high_ts = bar.timestamp;
                }
                if bar.low < self.cur_week_low {
                    self.cur_week_low = bar.low;
                    self.cur_week_low_ts = bar.timestamp;
                }
            }
        }

        // ── 2. Consommation PDH/PDL/PWH/PWL (« décisions trading » 23/08) ──
        // Un niveau ATTEINT sur bar confirmée (sweep OU cassure) est consommé.
        // Avant : sweep uniquement (la cassure laissait le niveau actif).
        // Les flags d'événement sweep_p* (informatifs, tests/API) gardent la
        // définition sweep historique : mèche au-delà + close retour.
        let sweep_pdh = self
            .pdh
            .is_some_and(|lvl| bar.high > lvl && bar.close < lvl);
        let sweep_pdl = self.pdl.is_some_and(|lvl| bar.low < lvl && bar.close > lvl);
        let sweep_pwh = self
            .pwh
            .is_some_and(|lvl| bar.high > lvl && bar.close < lvl);
        let sweep_pwl = self.pwl.is_some_and(|lvl| bar.low < lvl && bar.close > lvl);
        if self.pdh_active.is_some_and(|lvl| bar.high >= lvl) {
            self.pdh_active = None;
        }
        if self.pdl_active.is_some_and(|lvl| bar.low <= lvl) {
            self.pdl_active = None;
        }
        if self.pwh_active.is_some_and(|lvl| bar.high >= lvl) {
            self.pwh_active = None;
        }
        if self.pwl_active.is_some_and(|lvl| bar.low <= lvl) {
            self.pwl_active = None;
        }

        // ── 3. EQH/EQL (Pine lignes 633-694) ──
        let tol_eq = i_tol_eq() * atr14;
        let tol_cluster = i_tol_cluster() * atr14;

        let (sh1, sh2, sl1, sl2) = (pivots.sh1(), pivots.sh2(), pivots.sl1(), pivots.sl2());
        let bsh2 = pivots.last_pivot_high_bar_prev();
        let bsl2 = pivots.last_pivot_low_bar_prev();

        // isEQH = ph présent ET sh2 présent ET |sh1-sh2| <= tolEq.
        let is_eqh = pivot_event.is_pivot_high
            && sh2.is_some()
            && sh1.zip(sh2).is_some_and(|(a, b)| (a - b).abs() <= tol_eq);
        let is_eql = pivot_event.is_pivot_low
            && sl2.is_some()
            && sl1.zip(sl2).is_some_and(|(a, b)| (a - b).abs() <= tol_eq);

        // Création / mise à jour du pool (f_liqUpdate).
        if is_eqh && bsh2.is_some() {
            if let (Some(p1), Some(p2), Some(bp2)) = (sh1, sh2, bsh2) {
                self.dernier_eqh_level = Some((p1 + p2) / 2.0);
                self.liq_update(true, p1, p2, bp2, tol_cluster);
            }
        }
        if is_eql && bsl2.is_some() {
            if let (Some(p1), Some(p2), Some(bp2)) = (sl1, sl2, bsl2) {
                self.dernier_eql_level = Some((p1 + p2) / 2.0);
                self.liq_update(false, p1, p2, bp2, tol_cluster);
            }
        }

        // 3ᵉ touche isolée (Pine lignes 679-694) : un pivot confirmé (ph/pl) qui n'est
        // PAS égal à sh2/sl2 mais tombe dans la tolérance cluster d'un niveau existant.
        if !is_eqh && pivot_event.is_pivot_high && sh2.is_some() {
            if let Some(p1) = sh1 {
                self.liq_touch_isolated(true, p1, tol_cluster);
            }
        }
        if !is_eql && pivot_event.is_pivot_low && sl2.is_some() {
            if let Some(p1) = sl1 {
                self.liq_touch_isolated(false, p1, tol_cluster);
            }
        }

        let nb_liq_levels = self.pool.iter().filter(|l| !l.swept).count();
        let nb_liq_swept = self.pool.iter().filter(|l| l.swept).count();

        let ev = LiquiditeEvent {
            pdh: self.pdh,
            pdh_ts: self.pdh_ts,
            pdl_ts: self.pdl_ts,
            pwh_ts: self.pwh_ts,
            pwl_ts: self.pwl_ts,
            pdl: self.pdl,
            pwh: self.pwh,
            pwl: self.pwl,
            pdh_active: self.pdh_active,
            pdl_active: self.pdl_active,
            pwh_active: self.pwh_active,
            pwl_active: self.pwl_active,
            sweep_pdh,
            sweep_pdl,
            sweep_pwh,
            sweep_pwl,
            is_eqh,
            is_eql,
            dernier_eqh_level: self.dernier_eqh_level,
            dernier_eql_level: self.dernier_eql_level,
            nb_liq_levels,
            nb_liq_swept,
        };
        self.last_event = ev.clone();
        ev
    }

    /// `f_liqUpdate` Pine (lignes 647-668) : crée ou incrémente un niveau.
    fn liq_update(&mut self, is_high: bool, p1: f64, p2: f64, bp2: usize, tol_cluster: f64) {
        let new_price = (p1 + p2) / 2.0;
        // Cherche un niveau existant du même type, non sweepé, dans tolCluster du pivot.
        let found = self
            .pool
            .iter_mut()
            .find(|l| l.is_high == is_high && !l.swept && (l.price - p1).abs() <= tol_cluster);
        if let Some(level) = found {
            level.touches += 1;
            // Moyenne cumulée : price = (price × (touches-1) + p1) / touches.
            level.price = (level.price * (level.touches as f64 - 1.0) + p1) / level.touches as f64;
        } else {
            // Nouveau niveau (2 pivots à la création).
            if self.pool.len() >= MAX_LIQ {
                self.pool.remove(0); // FIFO shift.
            }
            self.pool.push(LiqLevel {
                price: new_price,
                t_first: bp2,
                touches: 2,
                swept: false,
                is_high,
            });
        }
    }

    /// 3ᵉ touche isolée (Pine lignes 679-694) : incrémente le 1ᵉʳ niveau match.
    fn liq_touch_isolated(&mut self, is_high: bool, p1: f64, tol_cluster: f64) {
        let found = self
            .pool
            .iter_mut()
            .find(|l| l.is_high == is_high && !l.swept && (l.price - p1).abs() <= tol_cluster);
        if let Some(level) = found {
            level.touches += 1;
            level.price = (level.price * (level.touches as f64 - 1.0) + p1) / level.touches as f64;
        }
    }

    // --- Accès sweep (MODULE 5) ---
    pub fn dernier_eqh_level(&self) -> Option<f64> {
        self.dernier_eqh_level
    }
    pub fn dernier_eql_level(&self) -> Option<f64> {
        self.dernier_eql_level
    }
    /// Test uniquement : injecte un dernierEQH.
    #[cfg(test)]
    pub fn set_dernier_eqh_pour_test(&mut self, lvl: f64) {
        self.dernier_eqh_level = Some(lvl);
    }

    /// `dernierEQL_level := na` (Pine ligne 770) — consommé par un sweep haussier.
    pub fn clear_dernier_eql(&mut self) {
        self.dernier_eql_level = None;
    }
    /// `dernierEQH_level := na` (Pine ligne 785) — consommé par un sweep baissier.
    pub fn clear_dernier_eqh(&mut self) {
        self.dernier_eqh_level = None;
    }

    /// « Décisions trading » 23/08 : consommation à l'ATTEINTE des dernierEQH/EQL
    /// (couvre la cassure — le sweep confirmé est déjà traité par le détecteur)
    /// et purge du pool (tout niveau touché est marqué sweepé = invisible).
    /// À appeler APRÈS `SweepDetector::update` (l'armement du sweep lit les niveaux).
    pub fn consommer_niveaux_atteints(&mut self, bar: &BarInput) {
        if self.dernier_eqh_level.is_some_and(|lvl| bar.high >= lvl) {
            self.dernier_eqh_level = None;
        }
        if self.dernier_eql_level.is_some_and(|lvl| bar.low <= lvl) {
            self.dernier_eql_level = None;
        }
        for l in self.pool.iter_mut() {
            if !l.swept {
                let atteint = if l.is_high {
                    bar.high >= l.price
                } else {
                    bar.low <= l.price
                };
                if atteint {
                    l.swept = true;
                }
            }
        }
    }

    /// Marque le 1ᵉʳ niveau correspondant comme sweepé (Pine lignes 773-778 / 786-791).
    /// Tolérance `tolEq` (plus stricte que tolCluster).
    pub fn mark_swept(&mut self, is_high: bool, level: f64, tol_eq: f64) {
        let found = self
            .pool
            .iter_mut()
            .find(|l| l.is_high == is_high && !l.swept && (l.price - level).abs() <= tol_eq);
        if let Some(l) = found {
            l.swept = true;
        }
    }

    pub fn pool(&self) -> &[LiqLevel] {
        &self.pool
    }
    pub fn last_event(&self) -> LiquiditeEvent {
        self.last_event.clone()
    }

    /// Accès mutateur de test (uniquement pour injecter un état EQL/EQH dans les
    /// tests du MODULE 5 sans rejouer tout le flux MODULE 4).
    #[cfg(test)]
    pub fn set_dernier_eql_for_test(&mut self, lvl: f64) {
        self.dernier_eql_level = Some(lvl);
    }
    #[cfg(test)]
    pub fn set_dernier_eqh_for_test(&mut self, lvl: f64) {
        self.dernier_eqh_level = Some(lvl);
    }
    #[cfg(test)]
    pub fn push_liq_level_for_test(&mut self, l: LiqLevel) {
        self.pool.push(l);
    }
}

impl Default for LiquiditesDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// `i_tolEq = 0.20` (Pine ligne 613).
fn i_tol_eq() -> f64 {
    0.20
}
/// `i_tolCluster = 0.50` (Pine ligne 614).
fn i_tol_cluster() -> f64 {
    0.50
}

/// Clé de jour **Paris** (Europe/Paris, DST auto via `common::time`).
/// Deux barres ont la même clé ssi elles appartiennent au même jour calendaire
/// Paris (00:00→23:59 CET/CEST) — utilisé pour détecter le changement de jour
/// PDH/PDL (équivalent `dayofmonth != dayofmonth[1]` en heure de Paris).
fn day_key(ts: i64) -> i64 {
    common::time::day_key_paris(ts)
}

/// Clé de semaine ISO (Pine `weekofyear`) : `(year, week)`.
fn week_key(ts: i64) -> (i32, u32) {
    use chrono::Datelike;
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0);
    match dt {
        Some(dt) => {
            let iso = dt.naive_utc().iso_week();
            (iso.year(), iso.week())
        }
        None => (0, 0),
    }
}

#[cfg(test)]
#[path = "liquidites_tests.rs"]
mod liquidites_tests;
