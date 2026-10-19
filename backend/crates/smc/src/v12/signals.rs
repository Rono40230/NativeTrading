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
//! SL FIXE par asset × palier de score (owner 19/10). TP1=entry+R, TP2=entry+2R,
//! TP3=liquidité la plus proche PLAFONNÉE à 3R (décision DoL≤3R 28/08).

use super::calibration::AssetCalibration;
use super::scoring_bs_zones::ScoringBsZones;
use super::scoring_v11::ScoringV11;
use super::signals_levels::{farthest_liq, make_trade, nearest_liq};
use super::trade::{Trade, TradeSource, TradeState};
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

/// TP3 réglable propriétaire : mode « liquidité lointaine » (la plus LOINTAINE
/// des EQH/PDH/PWH — repli sur R fixe si absente ou sous TP2) ou « R fixe ».
/// Prend précédence sur les ModeTp3 d'étude quand défini (production).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tp3Reglage {
    /// true = liquidité lointaine (fallback rfixe) ; false = R fixe.
    pub lointaine: bool,
    /// Cible en R du mode fixe ET repli du mode liquidités (défaut 3.0,
    /// borné 3-10 côté réglage ; toujours > TP2 — validation en cascade).
    pub rfixe: f64,
}

/// Étude SL fixe (owner 19/10, doctrine « expérience trading manuel ») :
/// SL = distance ABSOLUE (unités prix), modulée par paliers de score
/// (< 8 faible, ≥ 12 fort — vécu 576 trades : 21/60/18 %). La garde gère
/// l'interaction avec la largeur de zone. None = production inchangée
/// (bord opposé + offset ATR + clamps).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlFixeEtude {
    /// Distance absolue du SL pour un score moyen (palier 8-12).
    pub base: f64,
    /// Multiplicateur du palier faible (score < 8) et fort (score ≥ 12).
    pub mult_faible: f64,
    pub mult_fort: f64,
    pub garde: GardeSlFixe,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GardeSlFixe {
    /// SL fixe brut, même au milieu d'une zone large.
    Aucune,
    /// Zone plus large que le SL budgété → signal abandonné (esprit de la
    /// garde production « r > 2×slMax → skip »).
    SkipZoneLarge,
    /// SL serré au bord opposé de la zone quand il est plus proche que la
    /// distance fixe.
    SerreBord,
}

/// Le carnet de trades + générateur de signaux (Pine `stBull*`/`stBear*` + fonctions).
#[derive(Clone)]
pub struct SignalGenerator {
    pub trades: Vec<Trade>,
    trade_pousse: bool,
    next_id: u64,
    mode_tp3: ModeTp3,
    /// R1 (étude étape 3, 29/08) : sweep directionnel frais REQUIS pour la
    /// qualification v11 (canon ICT : prérequis, pas bonus). BSZones non
    /// concerné (zones nées de disp+sweep). Défaut inactif = production
    /// pré-verdict ; l'étude comparatif_sweep tranche.
    sweep_requis: bool,
    /// R2 (étude étape 3, 29/08) : porte P/D directionnel — canon ICT
    /// « jamais acheter en premium, vendre en discount ». Qualification v11 :
    /// trade bull interdit si close en premium, bear interdit en discount
    /// (la zone de tolérance équilibre laisse passer). Défaut inactif =
    /// production pré-verdict ; l'étude comparatif_pd tranche.
    pd_requis: bool,
    /// Étude étape 4 — TP1 = entry ± tp1_mult × r (1.0 = production).
    tp1_mult: f64,
    /// Étude étape 4 — TP2 = entry ± tp2_mult × r (2.0 = production).
    tp2_mult: f64,
    /// TP3 réglable propriétaire (production) — None = ModeTp3 d'étude.
    tp3_reglage: Option<Tp3Reglage>,
    /// Étude SL fixe (owner 19/10) — None = production (zone + ATR).
    sl_fixe: Option<SlFixeEtude>,
}

impl SignalGenerator {
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            trade_pousse: false,
            next_id: 1,
            // Décision DoL≤3R 28/08 (validated replay 24 mois) — production.
            mode_tp3: ModeTp3::DolCappe3R,
            sweep_requis: false,
            pd_requis: false,
            // Étape 4 (29/08, replay +239R/3 180 clôtures) : TP1 = 0.6R,
            // offset SL × 0.75 (clamps [_slMin,_slMax] inchangés).
            tp1_mult: 0.6,
            tp2_mult: 2.0,
            tp3_reglage: None,
            sl_fixe: None,
        }
    }

    /// Étude SL fixe (owner 19/10) : None = production.
    pub fn definir_sl_fixe(&mut self, e: Option<SlFixeEtude>) {
        self.sl_fixe = e;
    }

    /// Mode TP3 (défaut DolCappe3R = production, décision 28/08).
    pub fn definir_mode_tp3(&mut self, mode: ModeTp3) {
        self.mode_tp3 = mode;
    }

    /// Active/désactive la porte R1 (sweep requis en qualification v11).
    pub fn definir_sweep_requis(&mut self, actif: bool) {
        self.sweep_requis = actif;
    }

    /// Active/désactive la porte R2 (P/D directionnel en qualification v11).
    pub fn definir_pd_requis(&mut self, actif: bool) {
        self.pd_requis = actif;
    }

    /// Multiplicateurs de niveaux (TP1, TP2 — ×R du SL fixe).
    pub fn definir_multiplicateurs(&mut self, tp1: f64, tp2: f64) {
        self.tp1_mult = tp1;
        self.tp2_mult = tp2;
    }

    /// TP1 réglable par le propriétaire (Paramètres › stratégies › SMC,
    /// défaut production 0.6 — décision étape 4). SL et TP2 inchangés.
    pub fn definir_tp1(&mut self, tp1: f64) {
        self.tp1_mult = tp1;
    }

    /// TP2 réglable par le propriétaire (défaut production 2.0). Doit rester
    /// au-dessus de TP1 — la validation croisée vit côté réglage (carte SMC).
    pub fn definir_tp2(&mut self, tp2: f64) {
        self.tp2_mult = tp2;
    }

    /// TP3 réglable propriétaire (mode liquidité lointaine / R fixe + repli).
    pub fn definir_tp3_reglage(&mut self, reglage: Tp3Reglage) {
        self.tp3_reglage = Some(reglage);
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

        // 1. v11 BUY.
        if self.create_v11(
            true, out, bar, bar_index, atr, cal, ob_bull, scoring, fvg_bull,
        ) {
            self.trade_pousse = true;
            return;
        }
        // 2. v11 SELL.
        if self.create_v11(
            false, out, bar, bar_index, atr, cal, ob_bear, scoring, fvg_bear,
        ) {
            self.trade_pousse = true;
            return;
        }
        // 3. BS BUY.
        if self.create_bs(true, out, bar, bar_index, atr, cal, bs) {
            self.trade_pousse = true;
            return;
        }
        // 4. BS SELL.
        if self.create_bs(false, out, bar, bar_index, atr, cal, bs) {
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
            // R1 : sweep directionnel frais (fenêtre TF-adaptive) requis si actif.
            let sweep_ok = !self.sweep_requis
                || (if is_bull {
                    out.sweep.sweep_bull_frais
                } else {
                    out.sweep.sweep_bear_frais
                });
            // R2 : jamais acheter en premium / vendre en discount (canon ICT).
            let pd_ok = !self.pd_requis
                || (if is_bull {
                    !out.premium_discount.in_premium
                } else {
                    !out.premium_discount.in_discount
                });
            let qual = sc_r >= SEUIL_TRADE
                && ScoringV11::force(sc_r, cal) >= FORCE_MIN
                && zn_ok
                && sweep_ok
                && pd_ok;
            if !qual {
                continue;
            }
            let Some((entry, sl, tp1, tp2, tp3, risk0)) =
                self.build_levels(is_bull, top, bot, out, true, sc_r)
            else {
                continue; // r > 2×slMax → skip ce OB (continue).
            };
            let mut trade = make_trade(
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
            // §8 : journaliser l'instantané de qualification (lecture seule).
            trade.detail = Some(super::scoring_detail::detail_qualification(
                "OB", sc_r, zn_ok, sweep_ok, pd_ok, is_bull, out, bar, cal,
            ));
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
            let Some((entry, sl, tp1, tp2, tp3, risk0)) = self
                .build_levels(is_bull, top, bot, out, false, z.score)
            else {
                continue;
            };
            let mut trade = make_trade(
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
            // §8 : journaliser l'instantané de qualification (lecture seule).
            trade.detail = Some(super::scoring_detail::detail_qualification(
                "BSZones", z.score, true, false, false, is_bull, out, bar, cal,
            ));
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
        out: &SmcOutput,
        inclut_asian_hl: bool,
        score: i32,
    ) -> Option<(f64, f64, f64, f64, f64, f64)> {
        // Entrée TR forcée (Pine 3446 / 3593 / 3686 / 3745).
        let entry = if is_bull { zone_top } else { zone_bot };
        // SL FIXE (owner 19/10, doctrine trading manuel) : distance absolue
        // par asset (asset_params.sl_pips × taille_pip) × palier de score
        // (option C), bornée par la structure via la garde. Aucune valeur
        // configurée → aucun signal (échec visible, jamais de SL fantôme).
        let fixe = self.sl_fixe?;
        let mult = if score < 8 {
            fixe.mult_faible
        } else if score >= 12 {
            fixe.mult_fort
        } else {
            1.0
        };
        let mut r = fixe.base * mult;
        let largeur = zone_top - zone_bot;
        match fixe.garde {
            GardeSlFixe::SkipZoneLarge if largeur > r => return None,
            GardeSlFixe::SerreBord => r = r.min(largeur),
            _ => {}
        }
        let sl = if is_bull { entry - r } else { entry + r };
        let tp1 = if is_bull { entry + self.tp1_mult * r } else { entry - self.tp1_mult * r };
        let tp2 = if is_bull {
            entry + self.tp2_mult * r
        } else {
            entry - self.tp2_mult * r
        };
        // TP3 = liquidité la plus proche au-delà de l'entrée (EQH/PDH/PWH bull,
        // EQL/PDL/PWL bear ; Asian HL pour v11 uniquement — Pine 3562 vs 3294).
        let cap3r = if is_bull { entry + 3.0 * r } else { entry - 3.0 * r };
        // Réglage propriétaire : liquidité LOINTAINE (repli R fixe) ou R fixe —
        // précédence sur les modes d'étude.
        if let Some(reg) = self.tp3_reglage {
            let cible = if reg.lointaine {
                farthest_liq(out, entry, is_bull, inclut_asian_hl)
                    .filter(|&v| if is_bull { v > tp2 } else { v < tp2 })
                    .unwrap_or(if is_bull { entry + reg.rfixe * r } else { entry - reg.rfixe * r })
            } else if is_bull {
                entry + reg.rfixe * r
            } else {
                entry - reg.rfixe * r
            };
            let _ = cap3r;
            return Some((entry, sl, tp1, tp2, cible, r));
        }
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

#[cfg(test)]
#[path = "signals_tests.rs"]
mod tests;
