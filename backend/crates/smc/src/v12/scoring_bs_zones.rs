//! MODULE BSZONES — Détecteur de zones Buy/Sell 5 étoiles (Pine lignes 3051-3416).
//!
//! Moteur B coexistant avec le moteur OB v11 : crée ses PROPRES zones (Sweep→Disp→OB
//! canonique ICT 2022) et génère ses PROPRES trades. Indépendant des OB v12.
//!
//! Reproduit fidèlement :
//! - **Scoring socle /16** (displacement, sweep, mitigation, BOS, CHOCH, body/range, volume).
//! - **Bonus dynamique /11** (proximité, FVG overlap, PD, KZ, HTF, OTE, zone-cœur).
//! - **Conversion /10** `_toForce10` + **gate HTF** (zone ne naît que si baseScore≥6
//!   ET H1 ou H4 aligné) + **dead-zone** NY lunch (16h-18h UTC → KZ forcé 0).
//! - **Lifecycle** zones (mitigation 3 états, invalidation, recalcul score dynamique).
//!
//! Les fonctions de scoring pures et les helpers d'historique sont dans `bs_helpers`.

use super::bs_helpers::*;
use super::types::{BarInput, FvgZone, SmcOutput};

/// Une zone BSZones (Pine : arrays parallèles `bs{Bull,Bear}*`).
#[derive(Debug, Clone, Copy)]
pub struct BsZone {
    pub top: f64,
    pub bot: f64,
    /// 0=FRESH, 1=PARTIAL, 2=DEEP.
    pub state: u8,
    /// Socle figé /16 (Pine `bsBullBaseScore`).
    pub base_score: i32,
    /// Score total /10 (Pine `bsBullScore`), dynamique.
    pub score: i32,
    /// OTE figé à la création (Pine `bsBullInOTE`).
    pub in_ote: bool,
    /// Anti-retrade (Pine `bsBullSignaled`).
    pub signaled: bool,
    /// `bar_index` de création.
    pub bar: usize,
}

/// Détecteur BSZones (zones bull + bear + tracking _m15 BOS / dernier CHOCH).
#[derive(Clone)]
pub struct ScoringBsZones {
    bull: Vec<BsZone>,
    bear: Vec<BsZone>,
    /// `_m15_dernierBosBull_bar` (Pine 3032) — BOS swing-3 haussier (non masqué MSS).
    dern_bos_bull_bar: Option<usize>,
    dern_bos_bear_bar: Option<usize>,
    /// `dernierChochBull_bar` (Pine 490) — dernière confirmation CHOCH.
    dern_choch_bull_bar: Option<usize>,
    dern_choch_bear_bar: Option<usize>,
    /// FIFO max par sens (Pine `if array.size(bs*) > 20`).
    fifo_cap: usize,
    /// Compteur cumulé de naissances (diagnostic).
    births_bull: u32,
    births_bear: u32,
    /// Compteur cumulé du conjonction disp+sweep (pré-gate) — diagnostic.
    disp_sweep_bull: u32,
    disp_sweep_bear: u32,
    /// Max baseScore vu à la conjonction (diagnostic gate).
    max_base_bull: i32,
    max_base_bear: i32,
    /// Nombre de conjonctions avec baseScore≥6 (diagnostic gate).
    base_ok_bull: u32,
    base_ok_bear: u32,
}

impl ScoringBsZones {
    pub fn new() -> Self {
        Self {
            bull: Vec::with_capacity(24),
            bear: Vec::with_capacity(24),
            dern_bos_bull_bar: None,
            dern_bos_bear_bar: None,
            dern_choch_bull_bar: None,
            dern_choch_bear_bar: None,
            fifo_cap: 20,
            births_bull: 0,
            births_bear: 0,
            disp_sweep_bull: 0,
            disp_sweep_bear: 0,
            max_base_bull: 0,
            max_base_bear: 0,
            base_ok_bull: 0,
            base_ok_bear: 0,
        }
    }

    /// Nombre cumulé de zones nées (diagnostic fidélité).
    pub fn total_births(&self) -> (u32, u32) {
        (self.births_bull, self.births_bear)
    }

    /// Nombre cumulé de conjonctions disp+sweep (pré-gate baseScore/HTF) — diagnostic.
    pub fn total_disp_sweep(&self) -> (u32, u32) {
        (self.disp_sweep_bull, self.disp_sweep_bear)
    }

    /// (max_base_bull, max_base_bear, nb baseScore≥6 bull, nb bear) — diagnostic gate.
    pub fn gate_diag(&self) -> (i32, i32, u32, u32) {
        (
            self.max_base_bull,
            self.max_base_bear,
            self.base_ok_bull,
            self.base_ok_bear,
        )
    }

    pub fn bull_zones(&self) -> &[BsZone] {
        &self.bull
    }
    pub fn bear_zones(&self) -> &[BsZone] {
        &self.bear
    }
    pub fn bull_zones_mut(&mut self) -> &mut [BsZone] {
        &mut self.bull
    }
    pub fn bear_zones_mut(&mut self) -> &mut [BsZone] {
        &mut self.bear
    }

    /// Marque une zone comme ayant généré un trade (Pine `bsBullSignaled[i]:=true`).
    pub fn mark_signaled(&mut self, is_bull: bool, idx: usize) {
        let zones = if is_bull { &mut self.bull } else { &mut self.bear };
        if let Some(z) = zones.get_mut(idx) {
            z.signaled = true;
        }
    }

    /// Traite une bar : met à jour le tracking BOS/CHOCH, puis naissances + lifecycle.
    ///
    /// - `fvg_bull`/`fvg_bear` : zones FVG vivantes (pour le chevauchement zone-cœur/FVG).
    /// - `history` : historique rolling (dernier élément = bar courante).
    /// - `tf_sec` : timeframe en secondes (Pine `timeframe.in_seconds()`).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        out: &SmcOutput,
        bar: &BarInput,
        fvg_bull: &[FvgZone],
        fvg_bear: &[FvgZone],
        history: &[BarInput],
        bar_index: usize,
        tf_sec: i64,
    ) {
        let atr = out.atr14;

        // --- Tracking _m15 BOS (close-cross sh1, NON masqué MSS, Pine 3030-3037) ---
        let prev_close = bar_close_ago(history, 1);
        if let Some(sh1) = out.sh1 {
            if let Some(pc) = prev_close {
                if bar.close > sh1 && pc <= sh1 {
                    self.dern_bos_bull_bar = Some(bar_index);
                }
            }
        }
        if let Some(sl1) = out.sl1 {
            if let Some(pc) = prev_close {
                if bar.close < sl1 && pc >= sl1 {
                    self.dern_bos_bear_bar = Some(bar_index);
                }
            }
        }
        // Tracking dernier CHOCH (Pine 490-495) — pour `_chochScore`.
        if out.mss.choch_haussier {
            self.dern_choch_bull_bar = Some(bar_index);
        }
        if out.mss.choch_baissier {
            self.dern_choch_bear_bar = Some(bar_index);
        }

        // --- Contexte dynamique (Pine 3063-3067) ---
        let vol_sma20 = vol_sma_20(history);
        let ob_body_range1 = body_range_ago1(history);
        let mins = bar.timestamp.rem_euclid(86400) / 60;
        let in_dead_zone = (960..1080).contains(&mins);
        let fresh = sweep_fresh_bars(tf_sec);

        // --- Naissances (Pine 3224-3315) ---
        let disp_bull = bar.close > bar.open && (bar.close - bar.open) >= DISP_MULT * atr;
        let disp_bear = bar.open > bar.close && (bar.open - bar.close) >= DISP_MULT * atr;
        let sweep_recent_bull = sweep_recent(out.sweep.dernier_sweep_h_bar, bar_index, fresh);
        let sweep_recent_bear = sweep_recent(out.sweep.dernier_sweep_b_bar, bar_index, fresh);

        if disp_bull && sweep_recent_bull {
            self.disp_sweep_bull += 1;
            self.try_birth(true, out, bar, fvg_bull, history, bar_index, atr, vol_sma20, ob_body_range1);
        }
        if disp_bear && sweep_recent_bear {
            self.disp_sweep_bear += 1;
            self.try_birth(false, out, bar, fvg_bear, history, bar_index, atr, vol_sma20, ob_body_range1);
        }

        // --- Lifecycle zones (Pine 3318-3408) ---
        self.lifecycle(true, out, bar, fvg_bull, bar_index, atr, in_dead_zone);
        self.lifecycle(false, out, bar, fvg_bear, bar_index, atr, in_dead_zone);
    }

    #[allow(clippy::too_many_arguments)]
    fn try_birth(
        &mut self,
        is_bull: bool,
        out: &SmcOutput,
        bar: &BarInput,
        fvg: &[FvgZone],
        history: &[BarInput],
        bar_index: usize,
        atr: f64,
        vol_sma20: f64,
        ob_body_range1: f64,
    ) {
        // obT/obB = high[1]/low[1] de la bougie précédant le displacement.
        let (Some(ob_t), Some(ob_b)) = (bar_high_ago(history, 1), bar_low_ago(history, 1)) else {
            return;
        };

        // Corps sur 3 bougies (b1 courante + b2[1] + b3[2], même sens uniquement).
        let (b1, b2, b3) = if is_bull {
            let bb1 = bar.close - bar.open;
            let bb2 = body_delta(history, 1, bar, true);
            let bb3 = if bb2 > 0.0 { body_delta(history, 2, bar, true) } else { 0.0 };
            (bb1, bb2, bb3)
        } else {
            let bb1 = bar.open - bar.close;
            let bb2 = body_delta(history, 1, bar, false);
            let bb3 = if bb2 > 0.0 { body_delta(history, 2, bar, false) } else { 0.0 };
            (bb1, bb2, bb3)
        };
        let body3 = b1 + b2 + b3;

        let dern_sweep = if is_bull {
            out.sweep.dernier_sweep_h_bar
        } else {
            out.sweep.dernier_sweep_b_bar
        };
        let dern_bos = if is_bull {
            self.dern_bos_bull_bar
        } else {
            self.dern_bos_bear_bar
        };
        let dern_choch = if is_bull {
            self.dern_choch_bull_bar
        } else {
            self.dern_choch_bear_bar
        };
        let in_ote = if is_bull { out.ote.in_ote_bull } else { out.ote.in_ote_bear };

        let base_score = disp_score(b1, atr, body3)
            + sweep_score(dern_sweep, bar_index)
            + 3 // VIERGE à la création
            + bos_score(dern_bos, bar_index)
            + choch_score(dern_choch, bar_index)
            + body_range_score(ob_body_range1)
            + vol_score(bar_volume_ago(history, 1), vol_sma20);

        // Diagnostic gate (pré-baseScore≥6 / HTF).
        if is_bull {
            if base_score > self.max_base_bull {
                self.max_base_bull = base_score;
            }
            if base_score >= 6 {
                self.base_ok_bull += 1;
            }
        } else {
            if base_score > self.max_base_bear {
                self.max_base_bear = base_score;
            }
            if base_score >= 6 {
                self.base_ok_bear += 1;
            }
        }

        // GATE HTF (canon ICT) : baseScore≥6 ET (H1 ou H4 dans le sens).
        let h1 = out.mtf.h1.trend;
        let h4 = out.mtf.h4.trend;
        let d = if is_bull { 1 } else { -1 };
        if base_score >= 6 && (h1 == d || h4 == d) {
            let has_fvg = zn_has_fvg(fvg, ob_t, ob_b);
            // À la naissance, OTE/coeur utilisent l'OTE COURANT (Pine ligne 3246).
            let dyn_s = dyn_score(is_bull, bar.close, atr, ob_t, ob_b, has_fvg, in_ote, out, in_dead_zone_safe(bar));
            let sc = to_force10(base_score + dyn_s);
            let z = BsZone {
                top: ob_t,
                bot: ob_b,
                state: 0,
                base_score,
                score: sc,
                in_ote,
                signaled: false,
                bar: bar_index,
            };
            if is_bull {
                self.push_bull(z);
            } else {
                self.push_bear(z);
            }
        }
    }

    fn push_bull(&mut self, z: BsZone) {
        self.births_bull += 1;
        self.bull.push(z);
        if self.bull.len() > self.fifo_cap {
            self.bull.remove(0);
        }
    }
    fn push_bear(&mut self, z: BsZone) {
        self.births_bear += 1;
        self.bear.push(z);
        if self.bear.len() > self.fifo_cap {
            self.bear.remove(0);
        }
    }

    /// Lifecycle zones bull (is_bull=true) / bear (Pine 3318-3408).
    #[allow(clippy::too_many_arguments)]
    fn lifecycle(
        &mut self,
        is_bull: bool,
        out: &SmcOutput,
        bar: &BarInput,
        fvg: &[FvgZone],
        _bar_index: usize,
        atr: f64,
        in_dead_zone: bool,
    ) {
        let zones: &mut Vec<BsZone> = if is_bull { &mut self.bull } else { &mut self.bear };
        let mut i = zones.len() as isize - 1;
        while i >= 0 {
            let idx = i as usize;
            // Snapshot des champs figés pour éviter les aliasings.
            let (top, bot, st, in_ote, base) = {
                let z = &zones[idx];
                (z.top, z.bot, z.state, z.in_ote, z.base_score)
            };
            // Invalidation.
            let inval = if is_bull { bar.low < bot } else { bar.high > top };
            if inval {
                zones.remove(idx);
                i -= 1;
                continue;
            }
            // Transitions de mitigation.
            let mid = (top + bot) * 0.5;
            let mut new_st = st;
            if is_bull {
                if bar.low <= top {
                    if bar.close <= mid && st < 2 {
                        new_st = 2;
                    } else if bar.close > mid && st == 0 {
                        new_st = 1;
                    }
                }
            } else if bar.high >= bot {
                if bar.close >= mid && st < 2 {
                    new_st = 2;
                } else if bar.close < mid && st == 0 {
                    new_st = 1;
                }
            }
            let new_base = if new_st != st {
                base - mit_score(st) + mit_score(new_st)
            } else {
                base
            };
            // Recalcul dyn : OTE/coeur utilisent le OTE FIGÉ de la zone (Pine 3348).
            let has_fvg = zn_has_fvg(fvg, top, bot);
            let dyn_s = dyn_score(is_bull, bar.close, atr, top, bot, has_fvg, in_ote, out, in_dead_zone);
            let total = to_force10(new_base + dyn_s);
            {
                let z = &mut zones[idx];
                z.state = new_st;
                z.base_score = new_base;
                z.score = total;
            }
            i -= 1;
        }
    }
}

impl Default for ScoringBsZones {
    fn default() -> Self {
        Self::new()
    }
}
