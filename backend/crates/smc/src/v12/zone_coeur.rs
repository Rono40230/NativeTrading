//! Zone-cœur (Pine lignes 2112-2154) — `f_coeurBull(_i)` / `f_coeurBear(_i)`.
//!
//! Une zone-cœur = intersection **OB ∩ OTE ∩ 1er FVG chevauchant**, valide seulement si :
//! - chevauchement RÉEL (`_cT > _cB`),
//! - Discount pour le bull (`_cT < pdEquilibrium`) / Premium pour le bear (`_cB > pdEquilibrium`),
//! - sweep frais (`_sweepBullFrais` / `_sweepBearFrais`),
//! - OB non déjà signalé (`obBullSignaled`).
//!
//! Logique d'intersection (Pine `f_coeurBull`) :
//!   `_cT = min(obTop, oteTopBull) ; _cB = max(obBot, oteBotBull)`
//!   puis 1er FVG bull chevauchant (`_ft > _cB and _fb < _cC`) ⇒
//!   `_cT = min(_cT, _ft) ; _cB = max(_cB, _fb)`.
//!
//! `obBullSignaled` : en Pine, l'OB est marqué "signaled" une fois sa zone-cœur dessignée
//! (MODULE P3, lignes 3510/3657). On suit la même sémantique : un OB (identifié par son
//! `ob_bar`) ne peut produire qu'une seule zone-cœur — on marque ensuite son `ob_bar`.

use std::collections::HashSet;

use super::types::{FvgZone, ObZone, ZoneCoeurEvent, ZoneCoeurZone};

/// Détecteur Zone-cœur — évalue l'intersection pour chaque OB courant.
pub struct ZoneCoeurDetector {
    /// `ob_bar` des OB bull ayant déjà produit une zone-cœur (Pine `obBullSignaled`).
    bull_signaled: HashSet<usize>,
    bear_signaled: HashSet<usize>,
    last_event: ZoneCoeurEvent,
}

impl ZoneCoeurDetector {
    pub fn new() -> Self {
        Self {
            bull_signaled: HashSet::new(),
            bear_signaled: HashSet::new(),
            last_event: ZoneCoeurEvent::default(),
        }
    }

    /// Traite une bar : évalue la zone-cœur pour chaque OB courant.
    ///
    /// - `ob_bull` / `ob_bear` : OB actifs (ObDetector).
    /// - `fvg_bull` / `fvg_bear` : FVG actifs (FvgDetector).
    /// - `ote_bull_top`, `ote_bull_bot` / `ote_bear_top`, `ote_bear_bot` : bornes OTE.
    /// - `sweep_bull_frais` / `sweep_bear_frais` : fraîcheur du sweep (SweepEvent).
    /// - `pd_equilibrium` : équilibre Premium/Discount (PdEvent).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        ob_bull: &[ObZone],
        ob_bear: &[ObZone],
        fvg_bull: &[FvgZone],
        fvg_bear: &[FvgZone],
        ote_bull_bounds: Option<(f64, f64)>,
        ote_bear_bounds: Option<(f64, f64)>,
        sweep_bull_frais: bool,
        sweep_bear_frais: bool,
        pd_equilibrium: Option<f64>,
    ) -> ZoneCoeurEvent {
        let mut ev = ZoneCoeurEvent::default();

        // --- Zone-cœur bull (f_coeurBull) ---
        if let (Some((ote_top, ote_bot)), Some(eq)) = (ote_bull_bounds, pd_equilibrium) {
            if sweep_bull_frais {
                for ob in ob_bull {
                    if self.bull_signaled.contains(&ob.ob_bar) {
                        continue;
                    }
                    if let Some((zc_top, zc_bot)) = Self::coeur_for_ob(
                        ob.top,
                        ob.bot,
                        ote_top,
                        ote_bot,
                        fvg_bull,
                    ) {
                        // Validité : cT > cB ET cT < equilibrium (Discount).
                        if zc_top > zc_bot && zc_top < eq {
                            ev.bull.push(ZoneCoeurZone {
                                top: zc_top,
                                bot: zc_bot,
                                ob_bar: ob.ob_bar,
                                bull: true,
                            });
                            // Marquer l'OB comme signalé (Pine obBullSignaled).
                            self.bull_signaled.insert(ob.ob_bar);
                        }
                    }
                }
            }
        }

        // --- Zone-cœur bear (f_coeurBear) ---
        if let (Some((ote_top, ote_bot)), Some(eq)) = (ote_bear_bounds, pd_equilibrium) {
            if sweep_bear_frais {
                for ob in ob_bear {
                    if self.bear_signaled.contains(&ob.ob_bar) {
                        continue;
                    }
                    if let Some(zc) = Self::coeur_for_ob(
                        ob.top,
                        ob.bot,
                        ote_top,
                        ote_bot,
                        fvg_bear,
                    ) {
                        let (zc_top, zc_bot) = zc;
                        // Validité : cT > cB ET cB > equilibrium (Premium).
                        if zc_top > zc_bot && zc_bot > eq {
                            ev.bear.push(ZoneCoeurZone {
                                top: zc_top,
                                bot: zc_bot,
                                ob_bar: ob.ob_bar,
                                bull: false,
                            });
                            self.bear_signaled.insert(ob.ob_bar);
                        }
                    }
                }
            }
        }

        self.last_event = ev.clone();
        ev
    }

    /// Calcule l'intersection OB ∩ OTE ∩ 1er FVG chevauchant (Pine `f_coeurBull`/`Bear`).
    /// Retourne `(cT, cB)` si la précondition (OTE dispo + FVG chevauchant) est remplie.
    fn coeur_for_ob(
        ob_top: f64,
        ob_bot: f64,
        ote_top: f64,
        ote_bot: f64,
        fvgs: &[FvgZone],
    ) -> Option<(f64, f64)> {
        // cT = min(obTop, oteTop) ; cB = max(obBot, oteBot).
        let mut c_t = ob_top.min(ote_top);
        let mut c_b = ob_bot.max(ote_bot);
        // 1er FVG chevauchant : `ft > cB and fb < cT` ⇒ intersect.
        for fvg in fvgs {
            if fvg.top > c_b && fvg.bot < c_t {
                c_t = c_t.min(fvg.top);
                c_b = c_b.max(fvg.bot);
                return Some((c_t, c_b));
            }
        }
        None
    }

    pub fn last_event(&self) -> ZoneCoeurEvent {
        self.last_event.clone()
    }
}

impl Default for ZoneCoeurDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v12::types::{FvgState, ObState};

    fn fvg(top: f64, bot: f64) -> FvgZone {
        FvgZone {
            top,
            bot,
            state: FvgState::Fresh,
            bar: 0,
        }
    }
    fn ob(top: f64, bot: f64, bar: usize) -> ObZone {
        ObZone {
            top,
            bot,
            state: ObState::Vierge,
            impulse_bar: bar + 1,
            ob_bar: bar,
            timestamp: 0,
            is_ib: false,
        }
    }

    #[test]
    fn zone_coeur_bull_valide() {
        let mut det = ZoneCoeurDetector::new();
        // OB [100,108], OTE [101,107] ⇒ intersect [101,107].
        // FVG [102,106] chevauche ⇒ cT=min(107,106)=106, cB=max(101,102)=102.
        // eq=150 ⇒ cT=106 < 150 (Discount) ⇒ valide.
        let ev = det.update(
            &[ob(108.0, 100.0, 5)],
            &[],
            &[fvg(106.0, 102.0)],
            &[],
            Some((107.0, 101.0)),
            None,
            true,
            false,
            Some(150.0),
        );
        assert_eq!(ev.bull.len(), 1);
        let zc = ev.bull[0];
        assert!((zc.top - 106.0).abs() < 1e-9);
        assert!((zc.bot - 102.0).abs() < 1e-9);
    }

    #[test]
    fn pas_de_zone_coeur_sans_fvg_chevauchant() {
        let mut det = ZoneCoeurDetector::new();
        // FVG hors zone (top=200) ⇒ pas de chevauchement.
        let ev = det.update(
            &[ob(108.0, 100.0, 5)],
            &[],
            &[fvg(200.0, 150.0)],
            &[],
            Some((107.0, 101.0)),
            None,
            true,
            false,
            Some(1000.0),
        );
        assert!(ev.bull.is_empty(), "pas de FVG chevauchant ⇒ pas de zone-cœur");
    }

    #[test]
    fn pas_de_zone_coeur_hors_discount() {
        let mut det = ZoneCoeurDetector::new();
        // eq=50 ⇒ cT=106 > 50 ⇒ hors Discount ⇒ invalide (bull).
        let ev = det.update(
            &[ob(108.0, 100.0, 5)],
            &[],
            &[fvg(106.0, 102.0)],
            &[],
            Some((107.0, 101.0)),
            None,
            true,
            false,
            Some(50.0),
        );
        assert!(ev.bull.is_empty(), "hors Discount (cT > eq) ⇒ invalide bull");
    }

    #[test]
    fn pas_de_zone_coeur_sans_sweep_frais() {
        let mut det = ZoneCoeurDetector::new();
        let ev = det.update(
            &[ob(108.0, 100.0, 5)],
            &[],
            &[fvg(106.0, 102.0)],
            &[],
            Some((107.0, 101.0)),
            None,
            false, // sweep bull NON frais
            false,
            Some(150.0),
        );
        assert!(ev.bull.is_empty(), "sweep non frais ⇒ pas de zone-cœur");
    }

    #[test]
    fn ob_signaled_une_seule_fois() {
        let mut det = ZoneCoeurDetector::new();
        let inputs = (
            &[ob(108.0, 100.0, 5)],
            &[fvg(106.0, 102.0)],
            Some((107.0, 101.0)),
            Some(150.0),
        );
        let ev1 = det.update(inputs.0, &[], inputs.1, &[], inputs.2, None, true, false, inputs.3);
        assert_eq!(ev1.bull.len(), 1);
        // Même OB à nouveau ⇒ déjà signalé ⇒ ignoré.
        let ev2 = det.update(inputs.0, &[], inputs.1, &[], inputs.2, None, true, false, inputs.3);
        assert!(ev2.bull.is_empty(), "OB déjà signalé ⇒ pas de 2ᵉ zone-cœur");
    }

    #[test]
    fn zone_coeur_bear_valide_en_premium() {
        let mut det = ZoneCoeurDetector::new();
        // OB bear [100,108], OTE bear [101,107], FVG [102,106] ⇒ cT=106, cB=102.
        // eq=50 ⇒ cB=102 > 50 (Premium) ⇒ valide bear.
        let ev = det.update(
            &[],
            &[ob(108.0, 100.0, 9)],
            &[],
            &[fvg(106.0, 102.0)],
            None,
            Some((107.0, 101.0)),
            false,
            true,
            Some(50.0),
        );
        assert_eq!(ev.bear.len(), 1);
        assert!((ev.bear[0].top - 106.0).abs() < 1e-9);
    }
}
