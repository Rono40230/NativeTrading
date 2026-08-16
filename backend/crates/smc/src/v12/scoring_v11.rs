//! MODULE 11 — Scoring institutionnel v11 (Pine lignes 2156-2286, 2987-3016).
//!
//! Reproduit fidèlement :
//! - **`f_score(isBull)`** (16 composantes) — score brut par bar, borné par la garde
//!   anti-bruit (BOS seul → plafond 8) et le gate asset reconnu.
//! - **`f_force(sc)`** — mapping score → force /10 sur 4 bandes calibrées.
//! - **`f_accumScores`** — enrichit le score de chaque OB non signalé avec freshness
//!   (état 0→+3, 2→−2) + proximité (dist/ATR), en mode « sticky » max pour les OB
//!   non profonds (le score ne redescend pas), descente autorisée pour les profonds.
//! - **`f_znQualBull/Bear`** — filtres qualité zone (FVG sur l'OB + DoL au-delà).
//!
//! Le scoring lit l'état de TOUS les détecteurs via `SmcOutput` + la bar courante.

use super::calibration::AssetCalibration;
use super::types::{BarInput, ObState, ObZone, SmcOutput};

// ── Inputs Pine codés en dur (lignes 166-169) ────────────────────────────────
/// `i_prevLiqScore` — gate du bonus prevLiq (activé par défaut).
const PREV_LIQ_SCORE: bool = true;
/// `i_prevLiqAtrProx` — fenêtre de proximité en × ATR14.
const PREV_LIQ_ATR_PROX: f64 = 0.35;
/// `i_prevLiqPtsProx` — points de bonus proximité prevLiq.
const PREV_LIQ_PTS_PROX: i32 = 2;
/// `i_prevLiqPtsSweep` — points de bonus sweep prevLiq.
const PREV_LIQ_PTS_SWEEP: i32 = 4;

/// Détecteur de scoring v11 : maintient les scores + flags « signaled » des OB.
///
/// Les OB sont identifiés par leur `impulse_bar` (stable, contrairement à l'index
/// dans le `Vec<ObZone>` qui décale à chaque FIFO `remove(0)`).
#[derive(Clone)]
pub struct ScoringV11 {
    ob_bull_score: std::collections::HashMap<usize, i32>,
    ob_bear_score: std::collections::HashMap<usize, i32>,
    ob_bull_signaled: std::collections::HashSet<usize>,
    ob_bear_signaled: std::collections::HashSet<usize>,
    /// Filtre qualité P1.2 (FVG sur l'OB). Neutralisé pour DAX TF≥M15.
    zn_fvg_req: bool,
    /// Filtre qualité P2.3 (DoL au-delà de l'OB). Neutralisé pour DAX TF≥M15.
    zn_dol_req: bool,
}

impl ScoringV11 {
    /// Construit le scorer. `zn_dax_htf` = `_isDAX and tf_mins >= 15` (Pine 1074).
    pub fn new(cal: &AssetCalibration, tf_mins: u32) -> Self {
        let zn_dax_htf = cal.is_dax && tf_mins >= 15;
        Self {
            ob_bull_score: Default::default(),
            ob_bear_score: Default::default(),
            ob_bull_signaled: Default::default(),
            ob_bear_signaled: Default::default(),
            // i_znFvgReq / i_znDolReq = not _znDaxHTF (Pine 1075-1076).
            zn_fvg_req: !zn_dax_htf,
            zn_dol_req: !zn_dax_htf,
        }
    }

    /// Score brut live `f_score(isBull)` (Pine lignes 2184-2250).
    pub fn live_score(is_bull: bool, out: &SmcOutput, bar: &BarInput, cal: &AssetCalibration) -> i32 {
        let atr = out.atr14;
        let mut sc: i32 = 0;

        // 1. BOS directionnel — poids dynamique selon force du corps (P1.1 + P5.2).
        //    Anti-double-compte : on ne compte le BOS que si aucun MSS directionnel.
        let bos_dir = if is_bull { out.bos.bullish } else { out.bos.bearish };
        let mss_dir = if is_bull { out.mss.mss_haussier } else { out.mss.mss_baissier };
        if bos_dir && !mss_dir {
            let body = (bar.close - bar.open).abs();
            let w = if atr > 0.0 {
                if body >= 1.5 * atr {
                    if cal.is_btc { 5 } else { 6 }
                } else if body >= 0.5 * atr {
                    if cal.is_btc { 3 } else { 4 }
                } else if cal.is_btc {
                    1
                } else {
                    2
                }
            } else if cal.is_btc {
                3
            } else {
                4
            };
            sc += w;
        }

        // 2. FVG.
        if is_bull && out.fvg.is_fvg_bull || !is_bull && out.fvg.is_fvg_bear {
            sc += cal.w_fvg;
        }
        // 3. Sweep frais.
        if is_bull && out.sweep.sweep_bull_frais || !is_bull && out.sweep.sweep_bear_frais {
            sc += cal.w_sweep;
        }
        // 4. MSS directionnel.
        if mss_dir {
            sc += 3;
        }
        // 5. CHOCH confirmé.
        if is_bull && out.mss.choch_haussier || !is_bull && out.mss.choch_baissier {
            sc += 4;
        }
        // 6. ATR impulsion — range1 = high - low (bar courante, Pine ligne 1525).
        if (bar.high - bar.low) > cal.atr_score * atr {
            sc += cal.w_atr;
        }
        // 7. Confluence H4 (+4) — `not na(h4BullTop)` = au moins 1 OB bull H4.
        if out.mtf.confluence_h4
            && (is_bull && !out.mtf.h4.bull_obs.is_empty()
                || !is_bull && !out.mtf.h4.bear_obs.is_empty())
        {
            sc += 4;
        }
        // 8. Confluence H1 (+1).
        if out.mtf.confluence_h1
            && (is_bull && !out.mtf.h1.bull_obs.is_empty()
                || !is_bull && !out.mtf.h1.bear_obs.is_empty())
        {
            sc += 1;
        }
        // 9. Confluence W1 (+5).
        if out.mtf.confluence_w1
            && (is_bull && !out.mtf.w1.bull_obs.is_empty()
                || !is_bull && !out.mtf.w1.bear_obs.is_empty())
        {
            sc += 5;
        }
        // 10. Confluence MN (+6).
        if out.mtf.confluence_mn
            && (is_bull && !out.mtf.mn.bull_obs.is_empty()
                || !is_bull && !out.mtf.mn.bear_obs.is_empty())
        {
            sc += 6;
        }
        // 11. Imbalance (inner bar).
        if is_bull && out.imbalance.ib_bull || !is_bull && out.imbalance.ib_bear {
            sc += 3;
        }
        // 12. OTE 61.8–78.6 %.
        if is_bull && out.ote.in_ote_bull || !is_bull && out.ote.in_ote_bear {
            sc += cal.w_ote;
        }
        // 13. Kill Zone.
        if out.kill_zone.in_kz {
            sc += cal.w_kz;
        }
        // 14/15. Proximité / sweep prevLiq (PDL/PWL pour bull, PDH/PWH pour bear).
        if PREV_LIQ_SCORE {
            let (near, swept) = prev_liq_bull_bear(is_bull, out, bar, atr);
            if near {
                sc += PREV_LIQ_PTS_PROX;
            }
            if swept {
                sc += PREV_LIQ_PTS_SWEEP;
            }
        }
        // 16. Premium/Discount.
        if is_bull && out.premium_discount.in_discount || !is_bull && out.premium_discount.in_premium
        {
            sc += 1;
        }

        // ── Garde anti-bruit P1.2 : BOS seul (sans sweep/FVG/OTE/HTF≥H4) → plafond 8 ──
        let hs_bos = bos_dir;
        let hs_sweep = is_bull && out.sweep.sweep_bull_frais || !is_bull && out.sweep.sweep_bear_frais;
        let hs_fvg = is_bull && out.fvg.is_fvg_bull || !is_bull && out.fvg.is_fvg_bear;
        let hs_ote = is_bull && out.ote.in_ote_bull || !is_bull && out.ote.in_ote_bear;
        let hs_htf = out.mtf.confluence_h4 || out.mtf.confluence_w1 || out.mtf.confluence_mn;
        if hs_bos && !(hs_sweep || hs_fvg || hs_ote || hs_htf) {
            sc = sc.min(8);
        }

        // Phase 4.1 : asset non reconnu → aucun scoring.
        if !cal.asset_reconnu {
            sc = 0;
        }
        sc
    }

    /// `f_force(sc)` (Pine lignes 1000-1010) — score brut → force /10 sur 4 bandes.
    pub fn force(sc: i32, cal: &AssetCalibration) -> i32 {
        let seuil_moyen = cal.seuil_moyen as f64;
        let seuil_fort = cal.seuil_fort as f64;
        let seuil_instit = cal.seuil_instit as f64;
        let score_max = cal.score_max as f64;
        let s = sc as f64;
        let f = if s < seuil_moyen {
            1.0 + 3.0 * s / seuil_moyen.max(1.0)
        } else if s < seuil_fort {
            5.0 + (s - seuil_moyen) / (seuil_fort - seuil_moyen).max(1.0)
        } else if s < seuil_instit {
            7.0 + (s - seuil_fort) / (seuil_instit - seuil_fort).max(1.0)
        } else {
            9.0 + (s - seuil_instit) / (score_max - seuil_instit).max(1.0)
        };
        let r = f.round();
        // math.min(10, math.max(1, round)).
        (r as i32).clamp(1, 10)
    }

    /// `f_accumScores` (Pine lignes 2257-2285) — enrichit chaque OB non signalé.
    ///
    /// Doit être appelée à chaque bar APRES tous les détecteurs. Calcule le score
    /// live bull/bear puis, pour chaque OB vivant non signalé, applique freshness
    /// et proximité en mode sticky `max` — sauf OB profonds état 2 qui peuvent
    /// descendre via `max(0, cand)`. Prune les clés des OB disparus FIFO/invalidation.
    pub fn update(
        &mut self,
        out: &SmcOutput,
        bar: &BarInput,
        cal: &AssetCalibration,
        ob_bull: &[ObZone],
        ob_bear: &[ObZone],
    ) {
        let atr = out.atr14;
        let live_bull = Self::live_score(true, out, bar, cal);
        let live_bear = Self::live_score(false, out, bar, cal);

        // Bull.
        let mut alive_bull: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for z in ob_bull {
            alive_bull.insert(z.impulse_bar);
            if self.ob_bull_signaled.contains(&z.impulse_bar) {
                continue;
            }
            let mid = (z.top + z.bot) * 0.5;
            let dist = if atr > 0.0 { (bar.close - mid).abs() / atr } else { 0.0 };
            let st = z.state;
            let fresh = match st {
                ObState::Vierge => 3,
                ObState::Profond => -2,
                ObState::Partiel => 0,
            };
            let prox = if dist > 10.0 {
                -999
            } else if dist < 1.0 {
                2
            } else if dist > 5.0 {
                -1
            } else {
                0
            };
            let cand = live_bull + fresh + prox;
            let entry = self.ob_bull_score.entry(z.impulse_bar).or_insert(0);
            *entry = if st == ObState::Profond {
                cand.max(0)
            } else {
                (*entry).max(cand)
            };
        }
        // Bear.
        let mut alive_bear: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for z in ob_bear {
            alive_bear.insert(z.impulse_bar);
            if self.ob_bear_signaled.contains(&z.impulse_bar) {
                continue;
            }
            let mid = (z.top + z.bot) * 0.5;
            let dist = if atr > 0.0 { (bar.close - mid).abs() / atr } else { 0.0 };
            let st = z.state;
            let fresh = match st {
                ObState::Vierge => 3,
                ObState::Profond => -2,
                ObState::Partiel => 0,
            };
            let prox = if dist > 10.0 {
                -999
            } else if dist < 1.0 {
                2
            } else if dist > 5.0 {
                -1
            } else {
                0
            };
            let cand = live_bear + fresh + prox;
            let entry = self.ob_bear_score.entry(z.impulse_bar).or_insert(0);
            *entry = if st == ObState::Profond {
                cand.max(0)
            } else {
                (*entry).max(cand)
            };
        }

        // Prune : retire les OB disparus (FIFO ou invalidation).
        self.ob_bull_score.retain(|k, _| alive_bull.contains(k));
        self.ob_bear_score.retain(|k, _| alive_bear.contains(k));
        self.ob_bull_signaled.retain(|k| alive_bull.contains(k));
        self.ob_bear_signaled.retain(|k| alive_bear.contains(k));
    }

    /// Score enrichi d'un OB (0 si inconnu). `is_bull` = sens de l'OB.
    pub fn ob_score(&self, is_bull: bool, impulse_bar: usize) -> i32 {
        if is_bull {
            self.ob_bull_score.get(&impulse_bar).copied().unwrap_or(0)
        } else {
            self.ob_bear_score.get(&impulse_bar).copied().unwrap_or(0)
        }
    }

    /// Marque un OB comme ayant généré un signal (anti-retrade, Pine `obBullSignaled[i]:=true`).
    pub fn mark_signaled(&mut self, is_bull: bool, impulse_bar: usize) {
        if is_bull {
            self.ob_bull_signaled.insert(impulse_bar);
        } else {
            self.ob_bear_signaled.insert(impulse_bar);
        }
    }

    pub fn is_signaled(&self, is_bull: bool, impulse_bar: usize) -> bool {
        if is_bull {
            self.ob_bull_signaled.contains(&impulse_bar)
        } else {
            self.ob_bear_signaled.contains(&impulse_bar)
        }
    }

    /// `f_znQualBull` (Pine lignes 2998-3006) — FVG sur l'OB + DoL au-delà du top.
    /// `fvg_zones` = zones FVG bull vivantes, `out` fournit EQH/PDH/PWH actives.
    pub fn zn_qual_bull(&self, ob: &ObZone, out: &SmcOutput, fvg_zones: &[super::types::FvgZone]) -> bool {
        let fvg_ok = if self.zn_fvg_req {
            zn_has_fvg(fvg_zones, ob.top, ob.bot)
        } else {
            true
        };
        let dol_ok = if self.zn_dol_req {
            let t2 = ob.top;
            out.liquidite.dernier_eqh_level.is_some_and(|l| l > t2)
                || out.liquidite.pdh_active.is_some_and(|l| l > t2)
                || out.liquidite.pwh_active.is_some_and(|l| l > t2)
                // _ahHighDrawn (Asian High) omis — voir concern dans le rapport.
        } else {
            true
        };
        fvg_ok && dol_ok
    }

    /// `f_znQualBear` (Pine lignes 3008-3016) — FVG sur l'OB + DoL sous le bas.
    pub fn zn_qual_bear(&self, ob: &ObZone, out: &SmcOutput, fvg_zones: &[super::types::FvgZone]) -> bool {
        let fvg_ok = if self.zn_fvg_req {
            zn_has_fvg(fvg_zones, ob.top, ob.bot)
        } else {
            true
        };
        let dol_ok = if self.zn_dol_req {
            let b2 = ob.bot;
            out.liquidite.dernier_eql_level.is_some_and(|l| l < b2)
                || out.liquidite.pdl_active.is_some_and(|l| l < b2)
                || out.liquidite.pwl_active.is_some_and(|l| l < b2)
        } else {
            true
        };
        fvg_ok && dol_ok
    }
}

/// `f_znHasFVG` (Pine lignes 2990-2996) — chevauchement FVG avec un OB spécifique.
/// `fvg.top > ob.bot && fvg.bot < ob.top`.
fn zn_has_fvg(fvg_zones: &[super::types::FvgZone], ob_top: f64, ob_bot: f64) -> bool {
    fvg_zones
        .iter()
        .any(|f| f.top > ob_bot && f.bot < ob_top)
}

/// Calcule les flags prevLiq (near / swept) pour un sens donné (Pine lignes 2174-2182).
///
/// - Bull : PDL/PWL actifs.  near = |close - n| <= 0.35×ATR.
///   swept = low < n && close > n (revers haussier sous la liquidité).
/// - Bear : PDH/PWH actifs.  near = |close - n| <= 0.35×ATR.
///   swept = high > n && close < n.
fn prev_liq_bull_bear(is_bull: bool, out: &SmcOutput, bar: &BarInput, atr: f64) -> (bool, bool) {
    let prox = PREV_LIQ_ATR_PROX * atr;
    let (n1, n2) = if is_bull {
        (out.liquidite.pdl_active, out.liquidite.pwl_active)
    } else {
        (out.liquidite.pdh_active, out.liquidite.pwh_active)
    };
    let near = |n: Option<f64>| n.is_some_and(|v| (bar.close - v).abs() <= prox);
    let near_any = near(n1) || near(n2);
    let swept = if is_bull {
        let s = |n: Option<f64>| n.is_some_and(|v| bar.low < v && bar.close > v);
        s(n1) || s(n2)
    } else {
        let s = |n: Option<f64>| n.is_some_and(|v| bar.high > v && bar.close < v);
        s(n1) || s(n2)
    };
    (near_any, swept)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v12::calibration::AssetCalibration;

    fn cal_xau() -> AssetCalibration {
        AssetCalibration::detect("XAUUSD", "M15")
    }

    #[test]
    fn force_xau_4_bandes() {
        let c = cal_xau();
        // XAU : SEUIL_MOYEN=7, FORT=10, INSTIT=12, scoreMax=13.
        // sc=0 → 1+0 = 1.
        assert_eq!(ScoringV11::force(0, &c), 1);
        // sc=7 (==moyen) → 2e bande : 5 + (7-7)/3 = 5.
        assert_eq!(ScoringV11::force(7, &c), 5);
        // sc=10 (==fort) → 3e bande : 7 + (10-10)/2 = 7.
        assert_eq!(ScoringV11::force(10, &c), 7);
        // sc=12 (==instit) → 4e bande : 9 + (12-12)/1 = 9.
        assert_eq!(ScoringV11::force(12, &c), 9);
        // sc=13 → 9 + 1/1 = 10 (plafond).
        assert_eq!(ScoringV11::force(13, &c), 10);
        // sc=100 → clamp 10.
        assert_eq!(ScoringV11::force(100, &c), 10);
    }

    #[test]
    fn force_btc_plafond_moyen_only() {
        // BTC : MOYEN=8, FORT=INSTIT=99 (Moyen-only) → la note plafonne ~5-6.
        let c = AssetCalibration::detect("BTCUSD", "M15");
        assert_eq!(ScoringV11::force(8, &c), 5, "sc=moyen → 5");
        // sc=15 (scoreMax) → 2e bande (15<99) : 5 + (15-8)/91 ≈ 5.077 → 5.
        assert_eq!(ScoringV11::force(15, &c), 5);
    }

    #[test]
    fn live_score_asset_non_reconnu_zero() {
        let c = AssetCalibration::detect("EURUSD", "M15");
        let out = SmcOutput::default();
        let bar = BarInput::new(100.0, 110.0, 99.0, 105.0);
        assert_eq!(ScoringV11::live_score(true, &out, &bar, &c), 0);
        assert!(!c.asset_reconnu);
    }

    #[test]
    fn live_score_bos_seul_plafonne_a_8() {
        // Un BOS haussier sans aucune confluence → plafond 8 (garde anti-bruit).
        let c = cal_xau();
        let mut out = SmcOutput::default();
        out.atr14 = 2.0;
        out.bos.bullish = true;
        // Corps = 3 (>= 1.5×ATR=3) → poids 6.
        let bar = BarInput::new(100.0, 106.0, 99.0, 103.0);
        let sc = ScoringV11::live_score(true, &out, &bar, &c);
        assert_eq!(sc, 8, "BOS seul (poids 6) → plafond 8, pas 6");
    }

    #[test]
    fn live_score_bos_plus_fvg_depasse_plafond() {
        // BOS (6) + FVG (wFVG=5 pour XAU) → 11 (>8, garde ne s'applique pas).
        let c = cal_xau();
        let mut out = SmcOutput::default();
        out.atr14 = 2.0;
        out.bos.bullish = true;
        out.fvg.is_fvg_bull = true;
        let bar = BarInput::new(100.0, 106.0, 99.0, 103.0);
        let sc = ScoringV11::live_score(true, &out, &bar, &c);
        assert!(sc >= 11, "BOS+FVG doit dépasser 8, got {sc}");
    }

    #[test]
    fn accum_scores_sticky_max() {
        // Un OB vierge : score sticky = max(précédent, live+fresh+prox).
        let c = cal_xau();
        let mut s = ScoringV11::new(&c, 15);
        let mut out = SmcOutput::default();
        out.atr14 = 2.0;
        out.bos.bullish = true; // live élève
        let bar = BarInput::new(100.0, 106.0, 99.0, 103.0);
        let ob = ObZone {
            top: 102.0,
            bot: 98.0,
            state: ObState::Vierge,
            impulse_bar: 5,
            ob_bar: 4,
            timestamp: 0,
            is_ib: false,
        };
        s.update(&out, &bar, &c, &[ob], &[]);
        let sc1 = s.ob_score(true, 5);
        assert!(sc1 >= 8, "premier update: score >= 8 (plafond BOS), got {sc1}");

        // Bar suivante sans BOS → live faible, mais sticky max conserve sc1.
        let mut out2 = SmcOutput::default();
        out2.atr14 = 2.0;
        let bar2 = BarInput::new(103.0, 104.0, 102.0, 103.0);
        s.update(&out2, &bar2, &c, &[ob], &[]);
        let sc2 = s.ob_score(true, 5);
        assert_eq!(sc2, sc1, "sticky : le score ne redescend pas pour un OB vierge");
    }

    #[test]
    fn zn_qual_neutralise_pour_dax_m15() {
        // DAX M15 → _znDaxHTF true → filtres neutralisés (toujours qualifiés).
        let c = AssetCalibration::detect("DAX", "M15");
        let s = ScoringV11::new(&c, 15);
        let ob = ObZone {
            top: 102.0,
            bot: 98.0,
            state: ObState::Vierge,
            impulse_bar: 1,
            ob_bar: 0,
            timestamp: 0,
            is_ib: false,
        };
        let out = SmcOutput::default();
        assert!(s.zn_qual_bull(&ob, &out, &[]), "DAX M15 : zone toujours qualifiée");
    }
}
