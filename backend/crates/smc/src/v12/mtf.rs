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
                Some(dt) => (dt.naive_utc().iso_week().year() as i64) * 100
                    + dt.naive_utc().iso_week().week() as i64,
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

    /// Série HTF complète = bars clôturées + bar en cours (pour le replay repaint).
    fn series(&self, out: &mut Vec<BarInput>) {
        out.clear();
        out.extend_from_slice(&self.closed);
        if let Some(c) = self.cur_bar {
            out.push(c);
        }
    }
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
            let is_ph = (1..=sw_len).all(|j| {
                ph > bars[pidx - j].high && ph > bars[pidx + j].high
            });
            if is_ph {
                sh = Some(ph);
                bsh = Some(pidx);
            }
            let pl = bars[pidx].low;
            let is_pl = (1..=sw_len).all(|j| {
                pl < bars[pidx - j].low && pl < bars[pidx + j].low
            });
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
mod tests {
    use super::*;

    fn ltf_bar(ts: i64, open: f64, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: ts,
            open,
            high,
            low,
            close,
            volume: 1.0,
        }
    }

    /// Construit une série H1 haussière : pivot high net confirmé, puis plus tard un BOS
    /// au-dessus → doit produire un OB bull H1 (la dernière bougie baissière avant le BOS).
    #[test]
    fn h1_aggregation_et_ob_bull_apres_bos() {
        let mut det = MtfDetector::new();
        // sw_len=3 ⇒ pivot confirmé à i = pivot_idx + 3. Le BOS doit survenir à une bar
        // POSTÉRIEURE au pivot ET dont le high ne casse pas la confirmation du pivot.
        //   i0..i2 : doji 100 (high 102)
        //   i3     : pic high=110 (pivot candidat)
        //   i4..i6 : doji 100 (high 102 < 110 ⇒ pivot i3 confirmé à i6 ⇒ sh1=110)
        //   i7     : bear candle (open 105, close 100, low 99) ⇒ OB candidat (l_b_t=105)
        //   i8     : doji 100 (close==open ⇒ conserve le candidat 105)
        //   i9     : BOS (close 111 > 110, prev_close 100 <= 110) ⇒ OB bull = [105,99]
        let bars = [
            (0 * 3600, 100.0, 102.0, 98.0, 100.0),
            (1 * 3600, 100.0, 102.0, 98.0, 100.0),
            (2 * 3600, 100.0, 102.0, 98.0, 100.0),
            (3 * 3600, 100.0, 110.0, 99.0, 100.0), // pic
            (4 * 3600, 100.0, 102.0, 98.0, 100.0),
            (5 * 3600, 100.0, 102.0, 98.0, 100.0),
            (6 * 3600, 100.0, 102.0, 98.0, 100.0), // pivot i3 confirmé ⇒ sh1=110
            (7 * 3600, 105.0, 106.0, 99.0, 100.0), // bear candle ⇒ OB candidat [105,99]
            (8 * 3600, 100.0, 102.0, 98.0, 100.0), // doji ⇒ conserve candidat
            (9 * 3600, 100.0, 112.0, 99.0, 111.0), // BOS : close 111 > 110
        ];
        for (ts, o, h, l, c) in bars {
            det.update(&ltf_bar(ts, o, h, l, c));
        }
        let st = &det.last_event.h1;
        assert!(!st.bull_obs.is_empty(), "BOS up ⇒ au moins un OB bull H1");
        let ob = &st.bull_obs[0];
        assert!((ob.top - 105.0).abs() < 1e-9, "OB top = open du bear candle (105)");
        assert!((ob.bot - 99.0).abs() < 1e-9, "OB bot = low du bear candle (99)");
        assert_eq!(st.trend, 1, "BOS up ⇒ trend=1");
    }

    #[test]
    fn aggregation_h1_regroupe_4_bars_m15() {
        let mut det = MtfDetector::new();
        // 4 bars M15 dans la même heure (ts 0,900,1800,2700).
        det.update(&ltf_bar(0, 100.0, 102.0, 98.0, 101.0));
        det.update(&ltf_bar(900, 101.0, 105.0, 100.0, 103.0));
        det.update(&ltf_bar(1800, 103.0, 108.0, 102.0, 107.0));
        det.update(&ltf_bar(2700, 107.0, 110.0, 106.0, 109.0));
        // La bar H1 courante (pas encore clôturée) agrège les 4.
        let mut series = Vec::new();
        det.h1.series(&mut series);
        assert_eq!(series.len(), 1, "4 bars M15 ⇒ 1 bar H1 en cours");
        let h1bar = series[0];
        assert!((h1bar.open - 100.0).abs() < 1e-9, "open = 1ʳᵉ bar");
        assert!((h1bar.high - 110.0).abs() < 1e-9, "high = max");
        assert!((h1bar.low - 98.0).abs() < 1e-9, "low = min");
        assert!((h1bar.close - 109.0).abs() < 1e-9, "close = dernière");
    }

    #[test]
    fn confluence_fausse_sans_ob() {
        let mut det = MtfDetector::new();
        // Aucun pivot/BOS ⇒ aucun OB ⇒ pas de confluence.
        for i in 0..10 {
            det.update(&ltf_bar(i * 3600, 100.0, 101.0, 99.0, 100.0));
        }
        let ev = det.last_event();
        assert!(!ev.confluence_h1);
    }

    #[test]
    fn fifo_htf_cappe_a_max() {
        // S'assure qu'au-delà de MAX_HTF_BARS le tampon ne croît pas indéfiniment.
        let mut agg = HtfAggregator::new(Period::Seconds(3600));
        for i in 0..(MAX_HTF_BARS + 50) as i64 {
            agg.add(&ltf_bar(i * 3600, 100.0, 101.0, 99.0, 100.0));
        }
        // Toutes les périodes sont distinctes ⇒ chaque bar clôt la précédente (sauf la dernière en cours).
        assert!(agg.closed.len() <= MAX_HTF_BARS);
    }

    #[test]
    fn pas_de_panic_sur_serie_courte() {
        let st = replay_htf(&[], HTF_SWING);
        assert_eq!(st.trend, 0);
        assert!(st.bull_obs.is_empty() && st.bear_obs.is_empty());
    }
}
