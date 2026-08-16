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
            }
            Some(prev_dk) if prev_dk != dk => {
                // Changement de jour : la période précédente devient pdh/pdl.
                self.pdh = Some(self.cur_day_high);
                self.pdl = Some(self.cur_day_low);
                self.pdh_active = self.pdh;
                self.pdl_active = self.pdl;
                self.cur_day_key = Some(dk);
                self.cur_day_high = bar.high;
                self.cur_day_low = bar.low;
            }
            _ => {
                // Même jour : étend le high/low du jour courant.
                if bar.high > self.cur_day_high {
                    self.cur_day_high = bar.high;
                }
                if bar.low < self.cur_day_low {
                    self.cur_day_low = bar.low;
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
            }
            Some(prev_wk) if prev_wk != wk => {
                self.pwh = Some(self.cur_week_high);
                self.pwl = Some(self.cur_week_low);
                self.pwh_active = self.pwh;
                self.pwl_active = self.pwl;
                self.cur_week_key = Some(wk);
                self.cur_week_high = bar.high;
                self.cur_week_low = bar.low;
            }
            _ => {
                if bar.high > self.cur_week_high {
                    self.cur_week_high = bar.high;
                }
                if bar.low < self.cur_week_low {
                    self.cur_week_low = bar.low;
                }
            }
        }

        // ── 2. Sweep brut PDH/PDL/PWH/PWL (Pine lignes 244-247) ──
        let sweep_pdh = self.pdh.is_some_and(|lvl| bar.high > lvl && bar.close < lvl);
        let sweep_pdl = self.pdl.is_some_and(|lvl| bar.low < lvl && bar.close > lvl);
        let sweep_pwh = self.pwh.is_some_and(|lvl| bar.high > lvl && bar.close < lvl);
        let sweep_pwl = self.pwl.is_some_and(|lvl| bar.low < lvl && bar.close > lvl);

        // Invalidation des niveaux actifs (Pine lignes 249-256).
        if sweep_pdh {
            self.pdh_active = None;
        }
        if sweep_pdl {
            self.pdl_active = None;
        }
        if sweep_pwh {
            self.pwh_active = None;
        }
        if sweep_pwl {
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
        let found = self.pool.iter_mut().find(|l| {
            l.is_high == is_high && !l.swept && (l.price - p1).abs() <= tol_cluster
        });
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
        let found = self.pool.iter_mut().find(|l| {
            l.is_high == is_high && !l.swept && (l.price - p1).abs() <= tol_cluster
        });
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
    /// `dernierEQL_level := na` (Pine ligne 770) — consommé par un sweep haussier.
    pub fn clear_dernier_eql(&mut self) {
        self.dernier_eql_level = None;
    }
    /// `dernierEQH_level := na` (Pine ligne 785) — consommé par un sweep baissier.
    pub fn clear_dernier_eqh(&mut self) {
        self.dernier_eqh_level = None;
    }

    /// Marque le 1ᵉʳ niveau correspondant comme sweepé (Pine lignes 773-778 / 786-791).
    /// Tolérance `tolEq` (plus stricte que tolCluster).
    pub fn mark_swept(&mut self, is_high: bool, level: f64, tol_eq: f64) {
        let found = self.pool.iter_mut().find(|l| {
            l.is_high == is_high && !l.swept && (l.price - level).abs() <= tol_eq
        });
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
mod tests {
    use super::*;

    fn bar(ts: i64, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: ts,
            open: close,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    fn no_pivot() -> PivotEvent {
        PivotEvent::default()
    }

    // ===================== PDH / PDL =====================

    #[test]
    fn pdh_pdl_apparaissent_au_changement_de_jour() {
        let mut det = LiquiditesDetector::new();
        // Jour 1 (ts 0.., multiple bars M15).
        for i in 0..10 {
            let ts = i * 900; // 900s = 15 min
            det.update(&bar(ts, 100.0 + i as f64, 90.0, 95.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        }
        assert!(det.pdh.is_none(), "aucun pdh tant qu'un jour complet n'est pas passé");
        // Jour 2 (ts >= 86400).
        det.update(&bar(86400, 50.0, 40.0, 45.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        // pdh = high max du jour 1 = 109.0 (100..109).
        assert_eq!(det.pdh, Some(109.0));
        assert_eq!(det.pdl, Some(90.0));
        assert_eq!(det.pdh_active, Some(109.0));
        assert_eq!(det.pdl_active, Some(90.0));
    }

    #[test]
    fn sweep_pdh_invalide_pdh_active() {
        let mut det = LiquiditesDetector::new();
        // Jour 1 : high monte à 120.
        for i in 0..10 {
            let ts = i * 900;
            det.update(&bar(ts, 100.0 + i as f64 * 2.0, 90.0, 95.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        }
        // Jour 2 : pdh = 118.0.
        det.update(&bar(86400, 50.0, 40.0, 45.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        assert_eq!(det.pdh, Some(118.0));
        // Sweep : high > pdh (118) ET close < pdh.
        let ev = det.update(&bar(86400 + 900, 120.0, 110.0, 115.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        assert!(ev.sweep_pdh, "high=120 > pdh=118 ET close=115 < 118 ⇒ sweep");
        assert!(det.pdh_active.is_none(), "pdh_active invalidé après sweep");
        // pdh brut reste disponible (Pine n'efface pas pdh, seulement pdhActive).
        assert_eq!(det.pdh, Some(118.0));
    }

    #[test]
    fn pas_de_sweep_si_close_ne_reviend_pas() {
        let mut det = LiquiditesDetector::new();
        for i in 0..10 {
            let ts = i * 900;
            det.update(&bar(ts, 100.0 + i as f64, 90.0, 95.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        }
        det.update(&bar(86400, 50.0, 40.0, 45.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        assert_eq!(det.pdh, Some(109.0));
        // close >= pdh (pas de retour) ⇒ pas un sweep.
        let ev = det.update(&bar(86400 + 900, 120.0, 110.0, 110.0), &PivotDetector::new(3), &no_pivot(), 2.0);
        assert!(!ev.sweep_pdh);
    }

    // ===================== PWH / PWL =====================

    #[test]
    fn pwh_pwl_changement_de_semaine() {
        // Déterministe : départ lundi 2024-01-01 (unix 1704067200), semaine ISO W01.
        // Mon..Sun (d=0..6) ⇒ toujours W01 ; Mon d=7 (2024-01-08) ⇒ W02.
        let base = 1_704_067_200_i64;
        let mut det = LiquiditesDetector::new();
        // Une bar par jour, high croissant 100..160 sur la semaine W01.
        for d in 0..7usize {
            let ts = base + (d as i64) * 86_400;
            det.update(
                &bar(ts, 100.0 + d as f64 * 10.0, 90.0, 95.0),
                &PivotDetector::new(3),
                &no_pivot(),
                2.0,
            );
        }
        // Toujours dans W01 ⇒ aucun pwh encore.
        assert!(det.pwh.is_none(), "aucun pwh tant qu'une semaine ISO complète n'est pas passée");

        // d=7 ⇒ lundi W02 : pwh = high max de W01 = 160.0.
        let ts_w2 = base + 7 * 86_400;
        det.update(
            &bar(ts_w2, 50.0, 40.0, 45.0),
            &PivotDetector::new(3),
            &no_pivot(),
            2.0,
        );
        assert_eq!(det.pwh, Some(160.0), "pwh = high max de la semaine précédente");
        assert_eq!(det.pwl, Some(90.0));
        assert_eq!(det.pwh_active, det.pwh, "pwh_active rafraîchi au changement de semaine");
    }

    // ===================== EQH / EQL =====================

    fn build_pivots_eqh() -> PivotDetector {
        // 2 pivots high égaux (sh1=sh2=110), sl=3. Deux pics à index 3 et 9.
        let mut piv = PivotDetector::new(3);
        for i in 0..13usize {
            let h = if i == 3 || i == 9 { 110.0 } else { 100.0 };
            let b = BarInput {
                timestamp: i as i64,
                open: 100.0,
                high: h,
                low: 90.0,
                close: 100.0,
                volume: 0.0,
            };
            piv.update(&b);
        }
        piv
    }

    #[test]
    fn is_eqh_detecte_deux_highs_egaux_et_cree_niveau() {
        let piv = build_pivots_eqh();
        assert_eq!(piv.sh1(), Some(110.0));
        assert_eq!(piv.sh2(), Some(110.0));
        let mut det = LiquiditesDetector::new();
        // ATR14 artificiel = 10 ⇒ tolEq = 2.0 (|110-110|=0 <= 2 ⇒ EQH).
        let ev = det.update(
            &BarInput {
                timestamp: 12,
                open: 100.0,
                high: 100.0,
                low: 90.0,
                close: 100.0,
                volume: 0.0,
            },
            &piv,
            &PivotEvent {
                is_pivot_high: true,
                pivot_high_price: Some(110.0),
                pivot_bar_index: Some(9),
                ..Default::default()
            },
            10.0,
        );
        assert!(ev.is_eqh);
        assert_eq!(ev.dernier_eqh_level, Some(110.0));
        assert_eq!(det.pool().len(), 1, "un niveau EQH créé");
        let lvl = det.pool()[0];
        assert!(lvl.is_high);
        assert_eq!(lvl.touches, 2);
        assert!(!lvl.swept);
    }

    #[test]
    fn mark_swept_niveau_eqh_via_pool() {
        let piv = build_pivots_eqh();
        let mut det = LiquiditesDetector::new();
        // Crée le niveau EQH (2 touches) via le flux MODULE 4.
        det.update(
            &bar(12, 100.0, 90.0, 100.0),
            &piv,
            &PivotEvent {
                is_pivot_high: true,
                pivot_high_price: Some(110.0),
                pivot_bar_index: Some(9),
                ..Default::default()
            },
            10.0,
        );
        assert_eq!(det.pool()[0].touches, 2);
        // mark_swept grise le niveau EQH correspondant (consommation par un sweep baissier).
        det.mark_swept(true, 110.0, 2.0);
        assert!(det.pool()[0].swept, "mark_swept grise le niveau correspondant");
    }

    #[test]
    fn mark_swept_niveau_eql() {
        let mut det = LiquiditesDetector::new();
        // Crée un niveau EQL directement dans le pool.
        det.pool.push(LiqLevel {
            price: 90.0,
            t_first: 0,
            touches: 2,
            swept: false,
            is_high: false,
        });
        det.dernier_eql_level = Some(90.0);
        det.mark_swept(false, 90.0, 2.0);
        assert!(det.pool()[0].swept);
    }

    #[test]
    fn pool_fifo_limite_a_20_niveaux() {
        let mut det = LiquiditesDetector::new();
        for k in 0..25 {
            det.pool.push(LiqLevel {
                price: 100.0 + k as f64,
                t_first: k,
                touches: 2,
                swept: false,
                is_high: k % 2 == 0,
            });
        }
        // Simule le comportement FIFO de liq_update.
        if det.pool.len() >= MAX_LIQ {
            det.pool.remove(0);
        }
        assert_eq!(det.pool.len(), 25 - 1);
    }
}
