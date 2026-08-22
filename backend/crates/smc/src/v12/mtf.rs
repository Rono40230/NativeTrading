//! MODULE 12 — Multi-Timeframe (MTF).
//!
//! Reproduit MODULE 12 Pine (lignes 1693-1878) — c'est le module le plus complexe.
//!
//! Le Pine appelle `request.security("60"/"240"/"W"/"M", f_htf(3), lookahead_off)` pour
//! rejouer pivots/BOS/OB sur 4 TF supérieurs. En Rust, `request.security` n'existe pas :
//! on **agrège** les bars du TF courant en bars H1/H4/W1/MN, puis on rejoue la logique
//! `f_htf` sur la série agrégée.
//!
//! ## Approche
//! 1. Pour chaque TF (H1/H4/W1/MN), on maintient un `HtfAggregator` : bars clôturés
//!    + bar HTF en cours (période courante).
//! 2. À chaque bar LTF, on met à jour la bar HTF en cours (ou on en ouvre une nouvelle
//!    si la période a changé), puis on **rejoue** `f_htf` sur (bars clôturés + bar en cours).
//! 3. On extrait trend + 3 derniers OB bull + 3 derniers OB bear par TF.
//! 4. Confluence : `close ∈ au moins une des 6 zones` (3 bull + 3 bear).
//!
//! ## REPAINT assumé (Phase 3.4 Pine)
//! On évalue la bougie HTF **en cours** (comme `lookahead_off` live) : tant que la
//! bougie HTF n'est pas clôturée, les OB/confluences peuvent changer en temps réel.
//! C'est un choix délibéré du Pine (confluence réactive utile au scalping intrabar).
//!
//! ## `f_htf` (Pine lignes 1709-1832) — logique par bar HTF
//! - Pivots `ta.pivothigh/low(swLen, swLen)` → `_sh/_bsh`, `_sl/_bsl`.
//! - BOS : `_bosUp = close > _sh and close[1] <= _sh and anti-doublon(bsh)`.
//! - Dernière bougie opposée avant BOS = OB candidat :
//!   bear candle (close<open) : `_lBT=open, _lBB=low` ;
//!   bull candle (close>open) : `_lBuT=high, _lBuB=open`.
//! - Au BOS up : décale `_b1<-_prevLBT` (3 max), clear candidat.
//! - Au BOS down : décale `_r1<-_prevLBuT` (3 max).
//! - Mitigation : `close < _bXB` (bull) / `close > _rXT` (bear) ⇒ OB invalidé.
//! - `_trend` : +1 (bosUp), -1 (bosDown).

use super::types::{BarInput, HtfObZone, HtfState, MtfEvent};

/// `i_htfSwing` (Pine ligne 1695) = 3.
pub const HTF_SWING: usize = 3;
/// Durées en secondes des TF agrégés.
pub const H1_SEC: i64 = 3_600;
pub const H4_SEC: i64 = 14_400;
/// Taille max du tampon de bars HTF clôturées (borne mémoire/temps — vibe 600).
pub const MAX_HTF_BARS: usize = 600;

/// Période d'agrégation HTF.
#[derive(Debug, Clone, Copy)]
enum Period {
    /// Période fixe en secondes (H1, H4).
    Seconds(i64),
    /// Semaine ISO (W1).
    Week,
    /// Mois calendaire (MN).
    Month,
}

/// Clé de période : valeur identique ⟹ même période. (Valeur de comparaison seulement.)
fn period_key(p: Period, ts: i64) -> i64 {
    use chrono::Datelike;
    match p {
        Period::Seconds(s) => ts.div_euclid(s),
        Period::Week => {
            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0);
            match dt {
                Some(dt) => {
                    (dt.naive_utc().iso_week().year() as i64) * 100
                        + dt.naive_utc().iso_week().week() as i64
                }
                None => ts.div_euclid(604_800),
            }
        }
        Period::Month => {
            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0);
            match dt {
                Some(dt) => (dt.year() as i64) * 100 + dt.month() as i64,
                None => ts.div_euclid(2_592_000),
            }
        }
    }
}

/// Agrégateur de bars LTF → bars HTF.
#[derive(Clone)]
struct HtfAggregator {
    period: Period,
    /// Bars HTF clôturées (les plus anciennes d'abord).
    closed: Vec<BarInput>,
    /// Bar HTF en cours (période non clôturée) — REPAINT.
    cur_key: Option<i64>,
    cur_bar: Option<BarInput>,
}

impl HtfAggregator {
    fn new(period: Period) -> Self {
        Self {
            period,
            closed: Vec::new(),
            cur_key: None,
            cur_bar: None,
        }
    }

    /// Ingère une bar LTF. Agrège dans la bar HTF courante, ou clôture et en ouvre une nouvelle.
    fn add(&mut self, bar: &BarInput) {
        let key = period_key(self.period, bar.timestamp);
        let same = matches!(self.cur_key, Some(k) if k == key);
        if same {
            // Fusion dans la bar HTF courante (open conserve, close = dernière).
            if let Some(c) = self.cur_bar.as_mut() {
                if bar.high > c.high {
                    c.high = bar.high;
                }
                if bar.low < c.low {
                    c.low = bar.low;
                }
                c.close = bar.close;
                c.volume += bar.volume;
            }
        } else {
            // Nouvelle période : clôturer la bar courante.
            if let Some(c) = self.cur_bar.take() {
                self.closed.push(c);
                if self.closed.len() > MAX_HTF_BARS {
                    self.closed.remove(0); // FIFO — borne la mémoire.
                }
            }
            self.cur_key = Some(key);
            self.cur_bar = Some(*bar);
        }
    }

    /// Amorce l'agrégateur avec des bars HTF **clôturées** de la DB, antérieures
    /// à la fenêtre de replay LTF (t0 = timestamp de la 1re bar LTF).
    ///
    /// Sans cela, `f_htf` ne verrait que la fenêtre LTF (ex. 5 000 M15 ≈ 52 j
    /// → ~2 bars MN, ~8 W1) alors que le Pine/TV calcule sur des ANNÉES : les
    /// OB W1 (+5) et MN (+6) du scoring seraient structurellement invisibles.
    /// Sémantique : bars de période strictement antérieure à t0 → `closed` ;
    /// bar CONTENANT t0 → bar "en cours" (complétée aux bornes exactes par les
    /// bars LTF suivantes — high/low monotones ⇒ état clôturé exact).
    fn primer(&mut self, bars: &[BarInput], t0: i64) {
        let key0 = period_key(self.period, t0);
        for b in bars {
            let k = period_key(self.period, b.timestamp);
            if k < key0 {
                self.closed.push(*b);
                if self.closed.len() > MAX_HTF_BARS {
                    self.closed.remove(0);
                }
            } else if k == key0 {
                // Dernière écriture gagne (bars triées croissantes).
                self.cur_key = Some(k);
                self.cur_bar = Some(*b);
            }
            // k > key0 : postérieure à la fenêtre — ignorée (le replay LTF la reconstruit).
        }
    }

    /// Série HTF complète = bars clôturées + bar en cours (pour le replay repaint).
    fn series(&self, out: &mut Vec<BarInput>) {
        out.clear();
        out.extend_from_slice(&self.closed);
        if let Some(c) = self.cur_bar {
            out.push(c);
        }
    }
}

/// Historique HTF (H1/H4/W1/MN) pour l'amorçage du détecteur MTF.
#[derive(Debug, Clone, Default)]
pub struct AmorceMtf {
    pub h1: Vec<BarInput>,
    pub h4: Vec<BarInput>,
    pub w1: Vec<BarInput>,
    pub mn: Vec<BarInput>,
}

/// Agrège des bars journalières (D1) en bars mensuelles (MN) — la DB ne
/// collecte pas MN directement ; l'amorçage MTF en a besoin (confluence +6).
pub fn agreger_mensuel(d1: &[BarInput]) -> Vec<BarInput> {
    use chrono::Datelike;
    let mut out: Vec<BarInput> = Vec::new();
    for b in d1 {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(b.timestamp, 0);
        let Some(dt) = dt else { continue };
        let key = (dt.year() as i64) * 100 + dt.month() as i64;
        match out.last_mut() {
            Some(c) if period_key(Period::Month, c.timestamp) == key => {
                if b.high > c.high {
                    c.high = b.high;
                }
                if b.low < c.low {
                    c.low = b.low;
                }
                c.close = b.close;
                c.volume += b.volume;
            }
            _ => out.push(*b),
        }
    }
    out
}

/// Rejoue la logique `f_htf` (Pine lignes 1718-1831) sur une série HTF.
///
/// Retourne trend + 3 derniers OB bull + 3 derniers OB bear. Pivots en sémantique
/// stricte `>` (cohérent avec le `PivotDetector` du moteur — fix MQL5 2026-07-27).
fn replay_htf(bars: &[BarInput], sw_len: usize) -> HtfState {
    // Pivots persistants (Pine `_sh/_bsh/_sl/_bsl`).
    let mut sh: Option<f64> = None;
    let mut bsh: Option<usize> = None;
    let mut sl: Option<f64> = None;
    let mut bsl: Option<usize> = None;
    // Anti-doublon BOS (Pine `_lastSH_sig/_lastSL_sig`).
    let mut last_sh_sig: Option<usize> = None;
    let mut last_sl_sig: Option<usize> = None;
    // Candidats OB : dernière bougie opposée (Pine `_lBT/_lBB` = bear ; `_lBuT/_lBuB` = bull).
    let mut l_b_t: Option<f64> = None;
    let mut l_b_b: Option<f64> = None;
    let mut l_b_time: Option<i64> = None;
    let mut l_bu_t: Option<f64> = None;
    let mut l_bu_b: Option<f64> = None;
    let mut l_bu_time: Option<i64> = None;
    // 3 derniers OB bull (`_b1.._b3`) et bear (`_r1.._r3`).
    let mut b_top = [None; 3];
    let mut b_bot = [None; 3];
    let mut b_time = [None; 3];
    let mut r_top = [None; 3];
    let mut r_bot = [None; 3];
    let mut r_time = [None; 3];
    let mut trend: i32 = 0;
    let mut prev_close: Option<f64> = None;

    let n = bars.len();
    for i in 0..n {
        let bar = &bars[i];

        // --- Pivots (ta.pivothigh/low confirmé à la bar i pour le pivot à i-sw_len) ---
        if i >= 2 * sw_len {
            let pidx = i - sw_len;
            let ph = bars[pidx].high;
            let is_ph = (1..=sw_len).all(|j| ph > bars[pidx - j].high && ph > bars[pidx + j].high);
            if is_ph {
                sh = Some(ph);
                bsh = Some(pidx);
            }
            let pl = bars[pidx].low;
            let is_pl = (1..=sw_len).all(|j| pl < bars[pidx - j].low && pl < bars[pidx + j].low);
            if is_pl {
                sl = Some(pl);
                bsl = Some(pidx);
            }
        }

        // --- BOS (Pine lignes 1732-1733) ---
        let bos_up = match (sh, prev_close, bsh) {
            (Some(sh_v), Some(pc), Some(bsh_v)) => {
                let anti = match last_sh_sig {
                    None => true,
                    Some(s) => bsh_v != s,
                };
                anti && bar.close > sh_v && pc <= sh_v
            }
            _ => false,
        };
        let bos_down = match (sl, prev_close, bsl) {
            (Some(sl_v), Some(pc), Some(bsl_v)) => {
                let anti = match last_sl_sig {
                    None => true,
                    Some(s) => bsl_v != s,
                };
                anti && bar.close < sl_v && pc >= sl_v
            }
            _ => false,
        };
        if bos_up {
            last_sh_sig = bsh;
        }
        if bos_down {
            last_sl_sig = bsl;
        }

        // --- Sauvegarde des candidats bar[1] (Pine lignes 1744-1749) ---
        let prev_lbt = l_b_t;
        let prev_lbb = l_b_b;
        let prev_lbtime = l_b_time;
        let prev_lbut = l_bu_t;
        let prev_lbub = l_bu_b;
        let prev_lbutime = l_bu_time;

        // --- Mise à jour du candidat OB de la bougie courante (Pine lignes 1750-1763) ---
        // Remarque : deux `if` séparés (pas de else) ⇒ doji (close==open) garde l'ancien.
        if bar.close < bar.open {
            l_b_t = Some(bar.open);
            l_b_b = Some(bar.low);
            l_b_time = Some(bar.timestamp);
            l_bu_t = None;
            l_bu_b = None;
            l_bu_time = None;
        }
        if bar.close > bar.open {
            l_bu_t = Some(bar.high);
            l_bu_b = Some(bar.open);
            l_bu_time = Some(bar.timestamp);
            l_b_t = None;
            l_b_b = None;
            l_b_time = None;
        }

        // --- Décalage OB au BOS (Pine lignes 1783-1806) ---
        if bos_up && prev_lbt.is_some() {
            b_top[2] = b_top[1];
            b_bot[2] = b_bot[1];
            b_time[2] = b_time[1];
            b_top[1] = b_top[0];
            b_bot[1] = b_bot[0];
            b_time[1] = b_time[0];
            b_top[0] = prev_lbt;
            b_bot[0] = prev_lbb;
            b_time[0] = prev_lbtime;
            l_b_t = None;
            l_b_b = None;
        }
        if bos_down && prev_lbut.is_some() {
            r_top[2] = r_top[1];
            r_bot[2] = r_bot[1];
            r_time[2] = r_time[1];
            r_top[1] = r_top[0];
            r_bot[1] = r_bot[0];
            r_time[1] = r_time[0];
            r_top[0] = prev_lbut;
            r_bot[0] = prev_lbub;
            r_time[0] = prev_lbutime;
            l_bu_t = None;
            l_bu_b = None;
        }

        // --- Mitigation (Pine lignes 1808-1825) ---
        for k in 0..3 {
            if let (Some(_t), Some(bot)) = (b_top[k], b_bot[k]) {
                if bar.close < bot {
                    b_top[k] = None;
                    b_bot[k] = None;
                }
            }
        }
        for k in 0..3 {
            if let (Some(t), Some(_bot)) = (r_top[k], r_bot[k]) {
                if bar.close > t {
                    r_top[k] = None;
                    r_bot[k] = None;
                }
            }
        }

        // --- Trend (Pine lignes 1826-1830) ---
        if bos_up {
            trend = 1;
        }
        if bos_down {
            trend = -1;
        }

        prev_close = Some(bar.close);
    }

    // Construction de l'état final.
    let mut bull_obs = Vec::new();
    for k in 0..3 {
        if let (Some(top), Some(bot)) = (b_top[k], b_bot[k]) {
            if top > bot {
                bull_obs.push(HtfObZone {
                    top,
                    bot,
                    timestamp: b_time[k].unwrap_or(0),
                });
            }
        }
    }
    let mut bear_obs = Vec::new();
    for k in 0..3 {
        if let (Some(top), Some(bot)) = (r_top[k], r_bot[k]) {
            if top > bot {
                bear_obs.push(HtfObZone {
                    top,
                    bot,
                    timestamp: r_time[k].unwrap_or(0),
                });
            }
        }
    }
    HtfState {
        trend,
        bull_obs,
        bear_obs,
    }
}

/// `close ∈ au moins une des 6 zones OB` (3 bull + 3 bear) — Pine lignes 1851-1878.
fn confluence(close: f64, state: &HtfState) -> bool {
    for z in &state.bull_obs {
        if close >= z.bot && close <= z.top {
            return true;
        }
    }
    for z in &state.bear_obs {
        if close >= z.bot && close <= z.top {
            return true;
        }
    }
    false
}

/// Détecteur MTF — agrège H1/H4/W1/MN et calcule les confluences OB HTF.
#[derive(Clone)]
pub struct MtfDetector {
    h1: HtfAggregator,
    h4: HtfAggregator,
    w1: HtfAggregator,
    mn: HtfAggregator,
    sw_len: usize,
    last_event: MtfEvent,
}

impl MtfDetector {
    pub fn new() -> Self {
        Self {
            h1: HtfAggregator::new(Period::Seconds(H1_SEC)),
            h4: HtfAggregator::new(Period::Seconds(H4_SEC)),
            w1: HtfAggregator::new(Period::Week),
            mn: HtfAggregator::new(Period::Month),
            sw_len: HTF_SWING,
            last_event: MtfEvent::default(),
        }
    }

    /// Amorce les 4 agrégateurs avec l'historique HTF de la DB (avant replay LTF).
    /// `t0` = timestamp de la 1re bar LTF du replay. MN s'amorce depuis des
    /// bars mensuelles agrégées au préalable ([`agreger_mensuel`]).
    pub fn primer(
        &mut self,
        h1: &[BarInput],
        h4: &[BarInput],
        w1: &[BarInput],
        mn: &[BarInput],
        t0: i64,
    ) {
        self.h1.primer(h1, t0);
        self.h4.primer(h4, t0);
        self.w1.primer(w1, t0);
        self.mn.primer(mn, t0);
    }

    /// Variante struct ([`AmorceMtf`]).
    pub fn primer_amorce(&mut self, a: &AmorceMtf, t0: i64) {
        self.primer(&a.h1, &a.h4, &a.w1, &a.mn, t0);
    }

    /// Traite une bar LTF : agrège les 4 TF, rejoue `f_htf`, calcule les confluences.
    pub fn update(&mut self, bar: &BarInput) -> MtfEvent {
        // Tampon réutilisé (série HTF = closed + cur).
        let mut series: Vec<BarInput> = Vec::new();

        self.h1.add(bar);
        self.h1.series(&mut series);
        let h1_state = replay_htf(&series, self.sw_len);
        let confluence_h1 = confluence(bar.close, &h1_state);

        self.h4.add(bar);
        self.h4.series(&mut series);
        let h4_state = replay_htf(&series, self.sw_len);
        let confluence_h4 = confluence(bar.close, &h4_state);

        self.w1.add(bar);
        self.w1.series(&mut series);
        let w1_state = replay_htf(&series, self.sw_len);
        let confluence_w1 = confluence(bar.close, &w1_state);

        self.mn.add(bar);
        self.mn.series(&mut series);
        let mn_state = replay_htf(&series, self.sw_len);
        let confluence_mn = confluence(bar.close, &mn_state);

        let ev = MtfEvent {
            confluence_h1,
            confluence_h4,
            confluence_w1,
            confluence_mn,
            h1: h1_state,
            h4: h4_state,
            w1: w1_state,
            mn: mn_state,
        };
        self.last_event = ev.clone();
        ev
    }

    /// Série MN vue par le replay HTF (diagnostic amorçage).
    pub fn serie_mn(&self, out: &mut Vec<BarInput>) {
        self.mn.series(out);
    }

    pub fn last_event(&self) -> MtfEvent {
        self.last_event.clone()
    }
}

impl Default for MtfDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "mtf_tests.rs"]
mod mtf_tests;
