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

use super::asian_hl::SessHlLevels;
use super::bpr::{bonus_bpr, BprZone};
use super::calibration::AssetCalibration;
use super::types::{BarInput, ObState, ObZone, SmcOutput};

// ── Inputs Pine codés en dur (lignes 166-169) ────────────────────────────────
/// `i_prevLiqScore` — gate du bonus prevLiq (activé par défaut).
const PREV_LIQ_SCORE: bool = true;
/// `i_prevLiqAtrProx` — fenêtre de proximité en × ATR14.
const PREV_LIQ_ATR_PROX: f64 = 0.35;
/// `i_prevLiqPtsProx` — points de bonus proximité prevLiq.
const PREV_LIQ_PTS_PROX: i32 = 2;
/// `i_sessHlPtsProx` (Module F, Pine) — bonus proximité H/L de session.
const SESS_HL_PTS_PROX: i32 = 2;
/// `i_megaVolPts` (Module H, Pine) — bonus volume[1] ≥ 2× SMA20[1].
const MEGA_VOL_PTS: i32 = 2;
/// `i_megaVolMult` (Module H, Pine).
pub const MEGA_VOL_MULT: f64 = 2.0;
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
    /// Flags actifs au moment du NOUVEAU MAX du score de chaque zone
    /// (diag MQL5 `diagFlags`) — clé = impulse_bar.
    ob_bull_diag: std::collections::HashMap<usize, String>,
    ob_bear_diag: std::collections::HashMap<usize, String>,
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
            ob_bull_diag: Default::default(),
            ob_bear_diag: Default::default(),
            ob_bull_signaled: Default::default(),
            ob_bear_signaled: Default::default(),
            // i_znFvgReq / i_znDolReq = not _znDaxHTF (Pine 1075-1076).
            zn_fvg_req: !zn_dax_htf,
            zn_dol_req: !zn_dax_htf,
        }
    }

    /// Score brut live `f_score(isBull)` (Pine lignes 2184-2250).
    pub fn live_score(
        is_bull: bool,
        out: &SmcOutput,
        bar: &BarInput,
        cal: &AssetCalibration,
    ) -> i32 {
        Self::live_score_detaille(is_bull, out, bar, cal, None, None).0
    }

    /// `live_score` + liste des composantes actives (diag MQL5 `diagFlags` :
    /// flags au moment du NOUVEAU MAX du score d'une zone — comparaison
    /// directe avec le MQL5/TV pour traquer les écarts de scoring).
    pub fn live_score_detaille(
        is_bull: bool,
        out: &SmcOutput,
        bar: &BarInput,
        cal: &AssetCalibration,
        sess_hl: Option<&SessHlLevels>,
        mega_vol: Option<bool>,
    ) -> (i32, Vec<&'static str>) {
        let atr = out.atr14;
        let mut sc: i32 = 0;
        let mut flags: Vec<&'static str> = Vec::new();

        // 1. BOS directionnel — poids dynamique selon force du corps (P1.1 + P5.2).
        //    Anti-double-compte : on ne compte le BOS que si aucun MSS directionnel.
        let bos_dir = if is_bull {
            out.bos.bullish
        } else {
            out.bos.bearish
        };
        let mss_dir = if is_bull {
            out.mss.mss_haussier
        } else {
            out.mss.mss_baissier
        };
        if bos_dir && !mss_dir {
            let body = (bar.close - bar.open).abs();
            let w = if atr > 0.0 {
                if body >= 1.5 * atr {
                    if cal.is_btc {
                        5
                    } else {
                        6
                    }
                } else if body >= 0.5 * atr {
                    if cal.is_btc {
                        3
                    } else {
                        4
                    }
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
            flags.push("BOS");
        }

        // 2. FVG.
        if is_bull && out.fvg.is_fvg_bull || !is_bull && out.fvg.is_fvg_bear {
            sc += cal.w_fvg;
            flags.push("FVG");
        }
        // 3. Sweep frais.
        if is_bull && out.sweep.sweep_bull_frais || !is_bull && out.sweep.sweep_bear_frais {
            sc += cal.w_sweep;
            flags.push("Sweep");
        }
        // 4. MSS directionnel.
        if mss_dir {
            sc += 3;
            flags.push("MSS");
        }
        // 5. CHOCH confirmé.
        if is_bull && out.mss.choch_haussier || !is_bull && out.mss.choch_baissier {
            sc += 4;
            flags.push("CHoCH");
        }
        // 6. ATR impulsion — range1 = high - low (bar courante, Pine ligne 1525).
        if (bar.high - bar.low) > cal.atr_score * atr {
            sc += cal.w_atr;
            flags.push("ATR");
        }
        // 7. Confluence H4 (+4) — `not na(h4BullTop)` = au moins 1 OB bull H4.
        if out.mtf.confluence_h4
            && (is_bull && !out.mtf.h4.bull_obs.is_empty()
                || !is_bull && !out.mtf.h4.bear_obs.is_empty())
        {
            sc += 4;
            flags.push("H4");
        }
        // 8. Confluence H1 (+1).
        if out.mtf.confluence_h1
            && (is_bull && !out.mtf.h1.bull_obs.is_empty()
                || !is_bull && !out.mtf.h1.bear_obs.is_empty())
        {
            sc += 1;
            flags.push("H1");
        }
        // 9. Confluence W1 (+5).
        if out.mtf.confluence_w1
            && (is_bull && !out.mtf.w1.bull_obs.is_empty()
                || !is_bull && !out.mtf.w1.bear_obs.is_empty())
        {
            sc += 5;
            flags.push("W1");
        }
        // 10. Confluence MN (+6).
        if out.mtf.confluence_mn
            && (is_bull && !out.mtf.mn.bull_obs.is_empty()
                || !is_bull && !out.mtf.mn.bear_obs.is_empty())
        {
            sc += 6;
            flags.push("MN");
        }
        // 11. Imbalance (inner bar).
        if is_bull && out.imbalance.ib_bull || !is_bull && out.imbalance.ib_bear {
            sc += 3;
            flags.push("IB");
        }
        // 12. OTE 61.8–78.6 %.
        if is_bull && out.ote.in_ote_bull || !is_bull && out.ote.in_ote_bear {
            sc += cal.w_ote;
            flags.push("OTE");
        }
        // 13. Kill Zone.
        if out.kill_zone.in_kz {
            sc += cal.w_kz;
            flags.push("KZ");
        }
        // 14/15. Proximité / sweep prevLiq (PDL/PWL pour bull, PDH/PWH pour bear).
        if PREV_LIQ_SCORE {
            let (near, swept) = prev_liq_bull_bear(is_bull, out, bar, atr);
            if near {
                sc += PREV_LIQ_PTS_PROX;
                flags.push("nearLiq");
            }
            if swept {
                sc += PREV_LIQ_PTS_SWEEP;
                flags.push("swpLiq");
            }
        }
        // 15b. Module F — proximité H/L de session (Asie + Londres, état N-1) :
        //      bull près d'un LOW = SSL à cueillir ; bear près d'un HIGH = BSL.
        if let Some(sess) = sess_hl {
            if sess_hl_near(is_bull, sess, bar.close, atr) {
                sc += SESS_HL_PTS_PROX;
                flags.push("sessHL");
            }
        }
        // 15c. Module H — mega-order : volume[1] ≥ 2× SMA20[1] (sémantique
        //      _volScore BSZones ; volMa[1] Pine).
        if mega_vol == Some(true) {
            sc += MEGA_VOL_PTS;
            flags.push("megaVol");
        }
        // 16. Premium/Discount.
        if is_bull && out.premium_discount.in_discount
            || !is_bull && out.premium_discount.in_premium
        {
            sc += 1;
            flags.push("Disc");
        }

        // ── Garde anti-bruit P1.2 : BOS seul (sans sweep/FVG/OTE/HTF≥H4) → plafond 8 ──
        let hs_bos = bos_dir;
        let hs_sweep =
            is_bull && out.sweep.sweep_bull_frais || !is_bull && out.sweep.sweep_bear_frais;
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
        (sc, flags)
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

    /// Un-signal du PREMIER OB signalé dans l'ordre du carnet (Pine BE-force,
    /// lignes 3936-3941 + 3987-3988 : `_obIdx` = premier `signaled` trouvé en
    /// scannant les arrays — PAS forcément l'OB du trade). Sémantique exacte :
    /// l'OB du trade reste généralement verrouillé, ce qui limite les re-trades.
    pub fn unmark_premier_signale(&mut self, is_bull: bool, zones: &[ObZone]) {
        for z in zones {
            let present = if is_bull {
                self.ob_bull_signaled.remove(&z.impulse_bar)
            } else {
                self.ob_bear_signaled.remove(&z.impulse_bar)
            };
            if present {
                return; // premier trouvé, comme le break du Pine
            }
        }
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
        bpr_zones: &[BprZone],
        sess_hl: Option<&SessHlLevels>,
        mega_vol: Option<bool>,
    ) {
        let atr = out.atr14;
        let (live_bull, flags_bull) =
            Self::live_score_detaille(true, out, bar, cal, sess_hl, mega_vol);
        let (live_bear, flags_bear) =
            Self::live_score_detaille(false, out, bar, cal, sess_hl, mega_vol);

        // Bull.
        let mut alive_bull: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for z in ob_bull {
            alive_bull.insert(z.impulse_bar);
            if self.ob_bull_signaled.contains(&z.impulse_bar) {
                continue;
            }
            let mid = (z.top + z.bot) * 0.5;
            let dist = if atr > 0.0 {
                (bar.close - mid).abs() / atr
            } else {
                0.0
            };
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
            // Bonus BPR Module 6b (Pine 2504) : chevauchement BPR actif de
            // même sens (+4 frais · +3 partiel · +1 profond).
            let bpr_b = bonus_bpr(bpr_zones, true, z.top, z.bot);
            let cand = live_bull + fresh + prox + bpr_b;
            let entry = self.ob_bull_score.entry(z.impulse_bar).or_insert(0);
            let nouveau_max = cand > *entry;
            *entry = if st == ObState::Profond {
                cand.max(0)
            } else {
                (*entry).max(cand)
            };
            if nouveau_max {
                self.ob_bull_diag.insert(
                    z.impulse_bar,
                    format!("{} (+{}f{:?}p{})", flags_bull.join("+"), fresh, st, {
                        let d = if dist > 10.0 {
                            -999
                        } else if dist < 1.0 {
                            2
                        } else if dist > 5.0 {
                            -1
                        } else {
                            0
                        };
                        d
                    }),
                );
            }
        }
        // Bear.
        let mut alive_bear: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for z in ob_bear {
            alive_bear.insert(z.impulse_bar);
            if self.ob_bear_signaled.contains(&z.impulse_bar) {
                continue;
            }
            let mid = (z.top + z.bot) * 0.5;
            let dist = if atr > 0.0 {
                (bar.close - mid).abs() / atr
            } else {
                0.0
            };
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
            // Bonus BPR Module 6b (Pine 2520).
            let bpr_r = bonus_bpr(bpr_zones, false, z.top, z.bot);
            let cand = live_bear + fresh + prox + bpr_r;
            let entry = self.ob_bear_score.entry(z.impulse_bar).or_insert(0);
            let nouveau_max = cand > *entry;
            *entry = if st == ObState::Profond {
                cand.max(0)
            } else {
                (*entry).max(cand)
            };
            if nouveau_max {
                self.ob_bear_diag.insert(
                    z.impulse_bar,
                    format!("{} (+{}f{:?}p{})", flags_bear.join("+"), fresh, st, {
                        let d = if dist > 10.0 {
                            -999
                        } else if dist < 1.0 {
                            2
                        } else if dist > 5.0 {
                            -1
                        } else {
                            0
                        };
                        d
                    }),
                );
            }
        }

        // Prune : retire les OB disparus (FIFO ou invalidation).
        self.ob_bull_score.retain(|k, _| alive_bull.contains(k));
        self.ob_bear_score.retain(|k, _| alive_bear.contains(k));
        self.ob_bull_signaled.retain(|k| alive_bull.contains(k));
        self.ob_bear_signaled.retain(|k| alive_bear.contains(k));
        self.ob_bull_diag.retain(|k, _| alive_bull.contains(k));
        self.ob_bear_diag.retain(|k, _| alive_bear.contains(k));
    }

    /// Score enrichi d'un OB (0 si inconnu). `is_bull` = sens de l'OB.
    pub fn ob_score(&self, is_bull: bool, impulse_bar: usize) -> i32 {
        if is_bull {
            self.ob_bull_score.get(&impulse_bar).copied().unwrap_or(0)
        } else {
            self.ob_bear_score.get(&impulse_bar).copied().unwrap_or(0)
        }
    }

    /// Flags au moment du nouveau max du score (diag MQL5).
    pub fn ob_diag(&self, is_bull: bool, impulse_bar: usize) -> Option<&str> {
        if is_bull {
            self.ob_bull_diag.get(&impulse_bar).map(|s| s.as_str())
        } else {
            self.ob_bear_diag.get(&impulse_bar).map(|s| s.as_str())
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
    pub fn zn_qual_bull(
        &self,
        ob: &ObZone,
        out: &SmcOutput,
        fvg_zones: &[super::types::FvgZone],
    ) -> bool {
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
                || out.asian_hl.high.is_some_and(|l| l > t2) // _ahHighDrawn (Pine 3052)
        } else {
            true
        };
        fvg_ok && dol_ok
    }

    /// `f_znQualBear` (Pine lignes 3008-3016) — FVG sur l'OB + DoL sous le bas.
    pub fn zn_qual_bear(
        &self,
        ob: &ObZone,
        out: &SmcOutput,
        fvg_zones: &[super::types::FvgZone],
    ) -> bool {
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
                || out.asian_hl.low.is_some_and(|l| l < b2) // _ahLowDrawn (Pine 3061)
        } else {
            true
        };
        fvg_ok && dol_ok
    }
}

/// `f_znHasFVG` (Pine lignes 2990-2996) — chevauchement FVG avec un OB spécifique.
/// `fvg.top > ob.bot && fvg.bot < ob.top`.
fn zn_has_fvg(fvg_zones: &[super::types::FvgZone], ob_top: f64, ob_bot: f64) -> bool {
    fvg_zones.iter().any(|f| f.top > ob_bot && f.bot < ob_top)
}

/// Module F — proximité H/L de session (Pine `nearSessLow/nearSessHigh`) :
/// `|close - drawn| <= 0.35×ATR` sur les drawn Asie + Londres (état N-1),
/// bull → Lows, bear → Highs. `None` au niveau engine = module coupé.
pub(crate) fn sess_hl_near(is_bull: bool, s: &SessHlLevels, close: f64, atr: f64) -> bool {
    let prox = PREV_LIQ_ATR_PROX * atr;
    let near = |n: Option<f64>| n.is_some_and(|v| (close - v).abs() <= prox);
    if is_bull {
        near(s.ah_low) || near(s.ld_low)
    } else {
        near(s.ah_high) || near(s.ld_high)
    }
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
#[path = "scoring_v11_tests.rs"]
mod tests;
