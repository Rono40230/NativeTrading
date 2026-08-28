//! Génération de signaux — trades v11 (OB) + BSZones (Pine lignes 3419-3789).
//!
//! Reproduit fidèlement l'ordre Pine :
//!   `f_createBuySignals` → `f_createSellSignals` → `f_createBSBuySignals`
//!   → `f_createBSSellSignals`.
//!
//! Anti-doublon :
//! - **`f_tradeBloquant`** : bloque si un trade est REMPLI et non neutralisé
//!   (pas TP1/BE). Les trades en attente (non remplis) ne bloquent PAS.
//! - **`_tradePousseCetteBarre`** : 1 trade max par bar (tous systèmes confondus),
//!   reset à chaque bar confirmée.
//!
//! Modèle d'exécution figé "Retest (limite)" (gagnant A/B 15/15) : entrée TR forcée
//! au bord de la zone, fill réel au retest (géré par le lifecycle). SL selon
//! `_autoSlMode` clampé `[_slMin, _slMax]`. TP1=entry+R, TP2=entry+2R,
//! TP3=liquidité la plus proche PLAFONNÉE à 3R (décision DoL≤3R 28/08).

use super::calibration::{AssetCalibration, SlMode};
use super::scoring_bs_zones::ScoringBsZones;
use super::scoring_v11::ScoringV11;
use super::trade::{Side, Trade, TradeSource, TradeState};
use super::types::{BarInput, FvgZone, ObState, ObZone, SmcOutput};

/// Force minimale du signal (/10) — `i_forceMin` codé en dur v11 (Pine 2162).
const FORCE_MIN: i32 = 4;
/// `seuilTrade` (Pine 993) — tous les retours OB génèrent un trade (tri au score).
const SEUIL_TRADE: i32 = 0;
/// `i_tradeMinScore` (Pine 3059) — score min trade BSZones.
const TRADE_MIN_SCORE: i32 = 7;
/// Coefficient proche : `(close - rt) <= 8×ATR`.
const PROXIMITY_ATR_MULT: f64 = 8.0;

/// Mode de calcul du TP3 (étude Module G — DoL vs 3R fixe, décision 28/08).
/// `DolCappe3R` = PRODUCTION (décision validée : liquidité si plus proche
/// que 3R, sinon 3R — replay 24 mois : DoL pur -67R, plafonné +61.5R).
/// `Dol` = ancienne production (liquidité la plus proche, fallback 3R).
/// `Fixe3R` = contre-factuel d'étude (toujours entry ± 3R).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeTp3 {
    Dol,
    Fixe3R,
    DolCappe3R,
}

/// Le carnet de trades + générateur de signaux (Pine `stBull*`/`stBear*` + fonctions).
#[derive(Clone)]
pub struct SignalGenerator {
    pub trades: Vec<Trade>,
    trade_pousse: bool,
    next_id: u64,
    mode_tp3: ModeTp3,
}

impl SignalGenerator {
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            trade_pousse: false,
            next_id: 1,
            // Décision DoL≤3R 28/08 (validated replay 24 mois) — production.
            mode_tp3: ModeTp3::DolCappe3R,
        }
    }

    /// Mode TP3 (défaut DolCappe3R = production, décision 28/08).
    pub fn definir_mode_tp3(&mut self, mode: ModeTp3) {
        self.mode_tp3 = mode;
    }

    /// Reset du flag anti-double-trade (Pine 2358-2359) — à appeler en début de bar.
    pub fn reset_bar(&mut self) {
        self.trade_pousse = false;
    }

    /// `f_tradeBloquant` (Pine 2969-2979) — vrai si un trade ouvert est rempli et
    /// non neutralisé (pas TP1/BE). Les trades en attente ne bloquent pas.
    pub fn trade_bloquant(&self) -> bool {
        self.trades
            .iter()
            .any(|t| t.state != TradeState::Closed && t.filled && !t.neutralized())
    }

    /// Génère les signaux de la bar dans l'ordre Pine exact.
    ///
    /// Ordre : v11 BUY → v11 SELL → BS BUY → BS SELL. Un seul trade max par bar
    /// (tous systèmes confondus) via `_tradePousseCetteBarre`.
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        &mut self,
        out: &SmcOutput,
        bar: &BarInput,
        bar_index: usize,
        cal: &AssetCalibration,
        ob_bull: &[ObZone],
        ob_bear: &[ObZone],
        scoring: &mut ScoringV11,
        bs: &mut ScoringBsZones,
        fvg_bull: &[FvgZone],
        fvg_bear: &[FvgZone],
    ) {
        let atr = out.atr14;
        // f_tradeBloquant + _tradePousseCetteBarre + _regimeRange (toujours false).
        if self.trade_bloquant() || self.trade_pousse {
            return;
        }
        let (sl_min, sl_max) = sl_min_max(cal, atr);

        // 1. v11 BUY.
        if self.create_v11(
            true, out, bar, bar_index, atr, cal, ob_bull, scoring, fvg_bull, sl_min, sl_max,
        ) {
            self.trade_pousse = true;
            return;
        }
        // 2. v11 SELL.
        if self.create_v11(
            false, out, bar, bar_index, atr, cal, ob_bear, scoring, fvg_bear, sl_min, sl_max,
        ) {
            self.trade_pousse = true;
            return;
        }
        // 3. BS BUY.
        if self.create_bs(true, out, bar, bar_index, atr, cal, bs, sl_min, sl_max) {
            self.trade_pousse = true;
            return;
        }
        // 4. BS SELL.
        if self.create_bs(false, out, bar, bar_index, atr, cal, bs, sl_min, sl_max) {
            self.trade_pousse = true;
        }
    }

    /// Création d'un trade v11 (OB) — `f_createBuySignals`/`f_createSellSignals`.
    /// Retourne vrai si un trade a été poussé.
    #[allow(clippy::too_many_arguments)]
    fn create_v11(
        &mut self,
        is_bull: bool,
        out: &SmcOutput,
        bar: &BarInput,
        bar_index: usize,
        atr: f64,
        cal: &AssetCalibration,
        ob: &[ObZone],
        scoring: &mut ScoringV11,
        fvg: &[FvgZone],
        sl_min: f64,
        sl_max: f64,
    ) -> bool {
        for z in ob {
            let impulse_bar = z.impulse_bar;
            if scoring.is_signaled(is_bull, impulse_bar) {
                continue;
            }
            // _rBar = obBullBar = impulse_bar (Pine 1123) → pas de signal à la création.
            if bar_index <= impulse_bar {
                continue;
            }
            let top = z.top;
            let bot = z.bot;
            let state = z.state;
            let proche = atr <= 0.0
                || if is_bull {
                    (bar.close - top) <= PROXIMITY_ATR_MULT * atr
                } else {
                    (bot - bar.close) <= PROXIMITY_ATR_MULT * atr
                };
            // retour : close > top (bull) / close < bot (bear), état non profond.
            let retour = if is_bull {
                bar.close > top && state != ObState::Profond
            } else {
                bar.close < bot && state != ObState::Profond
            };
            if !retour || !proche {
                continue;
            }
            let sc_r = scoring.ob_score(is_bull, impulse_bar);
            // Qualification v11 : score >= seuilTrade(0) + force >= 4 + zone quality.
            let zn_ok = if is_bull {
                scoring.zn_qual_bull(z, out, fvg)
            } else {
                scoring.zn_qual_bear(z, out, fvg)
            };
            let qual = sc_r >= SEUIL_TRADE && ScoringV11::force(sc_r, cal) >= FORCE_MIN && zn_ok;
            if !qual {
                continue;
            }
            let Some((entry, sl, tp1, tp2, tp3, risk0)) =
                self.build_levels(is_bull, top, bot, atr, cal, out, sl_min, sl_max, true)
            else {
                continue; // r > 2×slMax → skip ce OB (continue).
            };
            let trade = make_trade(
                self.next_id,
                TradeSource::Ob,
                is_bull,
                entry,
                sl,
                tp1,
                tp2,
                tp3,
                sc_r,
                risk0,
                bar,
                bar_index,
                Some(impulse_bar),
            );
            self.trades.push(trade);
            self.next_id += 1;
            scoring.mark_signaled(is_bull, impulse_bar);
            return true;
        }
        false
    }

    /// Création d'un trade BSZones — `f_createBSBuySignals`/`f_createBSSellSignals`.
    #[allow(clippy::too_many_arguments)]
    fn create_bs(
        &mut self,
        is_bull: bool,
        out: &SmcOutput,
        bar: &BarInput,
        bar_index: usize,
        atr: f64,
        cal: &AssetCalibration,
        bs: &mut ScoringBsZones,
        sl_min: f64,
        sl_max: f64,
    ) -> bool {
        let n = if is_bull {
            bs.bull_zones().len()
        } else {
            bs.bear_zones().len()
        };
        for idx in 0..n {
            // BsZone est Copy → on snapshot pour libérer l'emprunt avant mark_signaled.
            let z = if is_bull {
                bs.bull_zones()[idx]
            } else {
                bs.bear_zones()[idx]
            };
            if z.signaled {
                continue;
            }
            // BSZones : pas de garde bar_index strict (Pine 3676-3679 l'omet), on exige
            // juste barre > création pour que le score soit finalisé par le lifecycle.
            if bar_index <= z.bar {
                continue;
            }
            let top = z.top;
            let bot = z.bot;
            let proche = atr <= 0.0
                || if is_bull {
                    (bar.close - top) <= PROXIMITY_ATR_MULT * atr
                } else {
                    (bot - bar.close) <= PROXIMITY_ATR_MULT * atr
                };
            let retour = if is_bull {
                bar.close > top && z.state < 2
            } else {
                bar.close < bot && z.state < 2
            };
            if !retour || !proche {
                continue;
            }
            // Qualification BSZones : score >= tradeMinScore(7).
            if z.score < TRADE_MIN_SCORE {
                continue;
            }
            let Some((entry, sl, tp1, tp2, tp3, risk0)) =
                self.build_levels(is_bull, top, bot, atr, cal, out, sl_min, sl_max, false)
            else {
                continue;
            };
            let trade = make_trade(
                self.next_id,
                TradeSource::BsZones,
                is_bull,
                entry,
                sl,
                tp1,
                tp2,
                tp3,
                z.score,
                risk0,
                bar,
                bar_index,
                None,
            );
            self.trades.push(trade);
            self.next_id += 1;
            bs.mark_signaled(is_bull, idx);
            return true;
        }
        false
    }

    /// Construit (entry, sl, tp1, tp2, tp3, risk0) clampé. Retourne None si r > 2×slMax.
    ///
    /// `is_bull` sens du trade. Entrée TR forcée au bord de la zone :
    /// `zone_top` (bull) / `zone_bot` (bear). SL base = bord opposé ± offset ATR.
    /// `inclut_asian_hl` : TP3 v11 inclut _ahHighDrawn/_ahLowDrawn (Pine 3562),
    /// TP3 BS NON (_bsDolTarget, Pine 3294 — EQH/PDH/PWH seuls).
    #[allow(clippy::too_many_arguments)]
    fn build_levels(
        &self,
        is_bull: bool,
        zone_top: f64,
        zone_bot: f64,
        atr: f64,
        cal: &AssetCalibration,
        out: &SmcOutput,
        sl_min: f64,
        sl_max: f64,
        inclut_asian_hl: bool,
    ) -> Option<(f64, f64, f64, f64, f64, f64)> {
        // Entrée TR forcée (Pine 3446 / 3593 / 3686 / 3745).
        let entry = if is_bull { zone_top } else { zone_bot };
        // SL brut selon _autoSlMode depuis le bord opposé.
        let offset = match cal.sl_mode {
            SlMode::Atr1x => atr,
            SlMode::Atr15x => 1.5 * atr,
            SlMode::Atr2x => 2.0 * atr,
            SlMode::BasOb => 0.0,
        };
        let raw_sl = if is_bull {
            zone_bot - offset
        } else {
            zone_top + offset
        };
        let raw_r = if is_bull {
            entry - raw_sl
        } else {
            raw_sl - entry
        };
        // Garde : r trop grand → skip (Pine 3451/3598/3691/3750 : `continue`).
        if raw_r > 2.0 * sl_max {
            return None;
        }
        // Clamp r ∈ [slMin, slMax], recalcul sl.
        let r = raw_r.max(sl_min).min(sl_max);
        let sl = if is_bull { entry - r } else { entry + r };
        let tp1 = if is_bull { entry + r } else { entry - r };
        let tp2 = if is_bull {
            entry + 2.0 * r
        } else {
            entry - 2.0 * r
        };
        // TP3 = liquidité la plus proche au-delà de l'entrée (EQH/PDH/PWH bull,
        // EQL/PDL/PWL bear ; Asian HL pour v11 uniquement — Pine 3562 vs 3294).
        let cap3r = if is_bull { entry + 3.0 * r } else { entry - 3.0 * r };
        let tp3_raw = match self.mode_tp3 {
            ModeTp3::Dol => nearest_liq(out, entry, is_bull, inclut_asian_hl),
            ModeTp3::Fixe3R => None,
            ModeTp3::DolCappe3R => nearest_liq(out, entry, is_bull, inclut_asian_hl).map(|v| {
                if is_bull { v.min(cap3r) } else { v.max(cap3r) }
            }),
        };
        let fallback = cap3r;
        // Monotonie TP3 : bull tp3 >= tp2, bear tp3 <= tp2 (Pine 3475/3622/3699/3757).
        let tp3 = match tp3_raw {
            Some(v) => {
                let ok = if is_bull { v >= tp2 } else { v <= tp2 };
                if ok {
                    v
                } else {
                    fallback
                }
            }
            None => fallback,
        };
        Some((entry, sl, tp1, tp2, tp3, r))
    }
}

impl Default for SignalGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Construit un `Trade` (factoring du dispatch buy/sell).
#[allow(clippy::too_many_arguments)]
fn make_trade(
    id: u64,
    source: TradeSource,
    is_bull: bool,
    entry: f64,
    sl: f64,
    tp1: f64,
    tp2: f64,
    tp3: f64,
    score: i32,
    risk0: f64,
    bar: &BarInput,
    bar_index: usize,
    ob_key: Option<usize>,
) -> Trade {
    let side = if is_bull { Side::Buy } else { Side::Sell };
    let mut t = Trade::new_buy(
        id, source, entry, sl, tp1, tp2, tp3, score, risk0, bar, bar_index, ob_key,
    );
    t.side = side;
    t
}

/// `_slMin` / `_slMax` (Pine 2424-2435) — en × ATR14 (atr toujours présent après warmup).
fn sl_min_max(cal: &AssetCalibration, atr: f64) -> (f64, f64) {
    let sl_min = if cal.is_xau {
        0.5 * atr
    } else if cal.is_xag {
        0.6 * atr
    } else if cal.is_nas || cal.is_spx {
        0.5 * atr
    } else if cal.is_btc {
        0.8 * atr
    } else if cal.is_dax {
        0.5 * atr
    } else {
        0.0
    };
    let sl_max = if cal.is_xau {
        1.5 * atr
    } else if cal.is_xag {
        1.8 * atr
    } else if cal.is_nas || cal.is_spx {
        1.5 * atr
    } else if cal.is_btc {
        2.5 * atr
    } else if cal.is_dax {
        1.5 * atr
    } else {
        1e10
    };
    (sl_min, sl_max)
}

/// Liquidité la plus proche au-delà de l'entrée (Pine 3460-3470 / 3607-3617).
/// Bull : EQH/PDH/PWH > entry → min. Bear : EQL/PDL/PWL < entry → max.
/// `inclut_asian_hl` : + _ahHighDrawn/_ahLowDrawn (v11 seulement, Pine `_tAHH3`).
fn nearest_liq(out: &SmcOutput, entry: f64, is_bull: bool, inclut_asian_hl: bool) -> Option<f64> {
    let cands: Vec<f64> = if is_bull {
        [
            out.liquidite.dernier_eqh_level,
            out.liquidite.pdh_active,
            out.liquidite.pwh_active,
        ]
        .into_iter()
        .flatten()
        .chain(if inclut_asian_hl {
            out.asian_hl.high
        } else {
            None
        })
        .filter(|&v| v > entry)
        .collect()
    } else {
        [
            out.liquidite.dernier_eql_level,
            out.liquidite.pdl_active,
            out.liquidite.pwl_active,
        ]
        .into_iter()
        .flatten()
        .chain(if inclut_asian_hl {
            out.asian_hl.low
        } else {
            None
        })
        .filter(|&v| v < entry)
        .collect()
    };
    if cands.is_empty() {
        return None;
    }
    Some(if is_bull {
        cands.into_iter().fold(f64::INFINITY, f64::min)
    } else {
        cands.into_iter().fold(f64::NEG_INFINITY, f64::max)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v12::calibration::AssetCalibration;

    #[test]
    fn sl_min_max_xau() {
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let (mn, mx) = sl_min_max(&c, 10.0);
        assert!((mn - 5.0).abs() < 1e-9, "XAU slMin = 0.5×ATR");
        assert!((mx - 15.0).abs() < 1e-9, "XAU slMax = 1.5×ATR");
    }

    #[test]
    fn nearest_liq_bull_plus_proche() {
        let mut out = SmcOutput::default();
        out.liquidite.pdh_active = Some(110.0);
        out.liquidite.pwh_active = Some(120.0);
        // entry=100 → candidats 110, 120 → min = 110.
        assert_eq!(nearest_liq(&out, 100.0, true, true), Some(110.0));
        assert_eq!(nearest_liq(&out, 100.0, true, false), Some(110.0));
    }

    #[test]
    fn nearest_liq_asian_hl_v11_seulement() {
        // Pine : _tAHH3 candidat pour v11 (Pine 3562), PAS pour BS (_bsDolTarget 3294).
        let mut out = SmcOutput::default();
        out.asian_hl.high = Some(105.0);
        out.asian_hl.low = Some(95.0);
        assert_eq!(nearest_liq(&out, 100.0, true, true), Some(105.0));
        assert_eq!(nearest_liq(&out, 100.0, true, false), None);
        assert_eq!(nearest_liq(&out, 100.0, false, true), Some(95.0));
        assert_eq!(nearest_liq(&out, 100.0, false, false), None);
    }

    #[test]
    fn nearest_liq_aucune() {
        let out = SmcOutput::default();
        assert_eq!(nearest_liq(&out, 100.0, true, true), None);
    }

    #[test]
    fn build_levels_clamp_et_monotonic_tp3() {
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let gen = SignalGenerator::new();
        let out = SmcOutput::default();
        // OB bull top=100 bot=98 → entry=100, offset=ATR (XAU Atr1x). ATR=2.
        // raw_sl = 98-2 = 96, raw_r = 4. slMin=1, slMax=3 → clamp r=3. sl=97.
        let (entry, sl, tp1, tp2, _tp3, r) = gen
            .build_levels(true, 100.0, 98.0, 2.0, &c, &out, 1.0, 3.0, true)
            .unwrap();
        assert!((entry - 100.0).abs() < 1e-9);
        assert!((r - 3.0).abs() < 1e-9, "r clampé à slMax=3");
        assert!((sl - 97.0).abs() < 1e-9);
        assert!((tp1 - 103.0).abs() < 1e-9);
        assert!((tp2 - 106.0).abs() < 1e-9);
    }

    #[test]
    fn build_levels_skip_si_r_trop_grand() {
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let gen = SignalGenerator::new();
        let out = SmcOutput::default();
        // top=100 bot=90 → entry=100, raw_sl=90-2=88, raw_r=12 > 2×slMax(=6) → None.
        assert!(gen
            .build_levels(true, 100.0, 90.0, 2.0, &c, &out, 1.0, 3.0, true)
            .is_none());
    }

    #[test]
    fn build_levels_tp3_dol_plafonne_3r_par_defaut() {
        // Décision DoL≤3R 28/08 : production = min(DoL, 3R). OB bull top=100
        // bot=99.2, ATR=2, XAU Atr1x → raw_sl=97.2, r=2.8 (non clampé).
        // TP2=105.6, plafond 3R=108.4.
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let gen = SignalGenerator::new();
        let mut out = SmcOutput::default();
        let args = |g: &SignalGenerator, o: &SmcOutput| {
            g.build_levels(true, 100.0, 99.2, 2.0, &c, o, 1.0, 3.0, true)
        };
        // Liquidité (PDH=115) au-delà du plafond 108.4 → TP3 plafonné.
        out.liquidite.pdh_active = Some(115.0);
        let (entry, _sl, _tp1, tp2, tp3, r) = args(&gen, &out).unwrap();
        assert!((entry - 100.0).abs() < 1e-9);
        assert!((r - 2.8).abs() < 1e-9);
        assert!((tp2 - 105.6).abs() < 1e-9);
        assert!(
            (tp3 - 108.4).abs() < 1e-9,
            "TP3 = min(PDH=115, entry+3R=108.4) = 108.4 (plafonné)"
        );
        // Liquidité PROCHE entre TP2 et 3R (107) → conservée telle quelle.
        out.liquidite.pdh_active = Some(107.0);
        let (_, _, _, _, tp3_proche, _) = args(&gen, &out).unwrap();
        assert!((tp3_proche - 107.0).abs() < 1e-9, "DoL proche conservé");
    }
}
