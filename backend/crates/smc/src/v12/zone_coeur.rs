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
#[derive(Clone)]
pub struct ZoneCoeurDetector {
    /// `ob_bar` des OB bull ayant déjà produit une zone-cœur (Pine `obBullSignaled`).
    bull_signaled: HashSet<usize>,
    bear_signaled: HashSet<usize>,
    /// Boxes live (Pine `obBullCoreBox`/`obBearCoreBox`) : validité re-vérifiée
    /// à chaque barre, bornes figées à la création (`box.set_right` seulement).
    live_bull: Vec<ZoneCoeurZone>,
    live_bear: Vec<ZoneCoeurZone>,
    last_event: ZoneCoeurEvent,
}

impl ZoneCoeurDetector {
    pub fn new() -> Self {
        Self {
            bull_signaled: HashSet::new(),
            bear_signaled: HashSet::new(),
            live_bull: Vec::new(),
            live_bear: Vec::new(),
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
                    if let Some((zc_top, zc_bot)) =
                        Self::coeur_for_ob(ob.top, ob.bot, ote_top, ote_bot, fvg_bull)
                    {
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
                    if let Some(zc) = Self::coeur_for_ob(ob.top, ob.bot, ote_top, ote_bot, fvg_bear)
                    {
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

        // --- Lifecycle LIVE (Pine f_zoneCoeurLifecycle, lignes 3570-3608) ---
        // Box créée au 1er setup valide (bornes figées), prolongée tant que
        // valide, SUPPRIMÉE dès invalidation ou disparition de l'OB parent.
        // Le gate `not obBullSignaled` de Pine marque les OB porteurs d'un
        // trade — les trades ne sont pas encore portés (Phase 2.8) : validité
        // purement géométrique, recréation autorisée après invalidation.
        self.live_bull = Self::cycle_live(
            ob_bull,
            fvg_bull,
            ote_bull_bounds,
            sweep_bull_frais,
            pd_equilibrium,
            &self.live_bull,
            true,
        );
        self.live_bear = Self::cycle_live(
            ob_bear,
            fvg_bear,
            ote_bear_bounds,
            sweep_bear_frais,
            pd_equilibrium,
            &self.live_bear,
            false,
        );
        ev.live_bull = self.live_bull.clone();
        ev.live_bear = self.live_bear.clone();

        self.last_event = ev.clone();
        ev
    }

    /// Cycle des boxes live : reconstruit la liste des zones valides à la
    /// barre courante (Pine re-vérifie `f_coeurBull`/`f_coeurBear` à chaque
    /// barre). Une box existante garde ses bornes figées (Pine ne fait que
    /// `box.set_right`) ; une box recréée prend les bornes courantes.
    fn cycle_live(
        obs: &[ObZone],
        fvgs: &[FvgZone],
        ote_bounds: Option<(f64, f64)>,
        sweep_frais: bool,
        pd_equilibrium: Option<f64>,
        prev: &[ZoneCoeurZone],
        bull: bool,
    ) -> Vec<ZoneCoeurZone> {
        let mut next = Vec::new();
        let (Some((ote_top, ote_bot)), Some(eq)) = (ote_bounds, pd_equilibrium) else {
            return next;
        };
        if !sweep_frais {
            return next;
        }
        for ob in obs {
            let Some((c_t, c_b)) = Self::coeur_for_ob(ob.top, ob.bot, ote_top, ote_bot, fvgs)
            else {
                continue;
            };
            let ok = if bull {
                c_t > c_b && c_t < eq
            } else {
                c_t > c_b && c_b > eq
            };
            if !ok {
                continue;
            }
            let zone = prev
                .iter()
                .find(|z| z.ob_bar == ob.ob_bar)
                .cloned()
                .unwrap_or(ZoneCoeurZone {
                    top: c_t,
                    bot: c_b,
                    ob_bar: ob.ob_bar,
                    bull,
                });
            next.push(zone);
        }
        next
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
        assert!(
            ev.bull.is_empty(),
            "pas de FVG chevauchant ⇒ pas de zone-cœur"
        );
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
        assert!(
            ev.bull.is_empty(),
            "hors Discount (cT > eq) ⇒ invalide bull"
        );
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
        let ev1 = det.update(
            inputs.0,
            &[],
            inputs.1,
            &[],
            inputs.2,
            None,
            true,
            false,
            inputs.3,
        );
        assert_eq!(ev1.bull.len(), 1);
        // Même OB à nouveau ⇒ déjà signalé ⇒ ignoré.
        let ev2 = det.update(
            inputs.0,
            &[],
            inputs.1,
            &[],
            inputs.2,
            None,
            true,
            false,
            inputs.3,
        );
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
        assert_eq!(ev.live_bear.len(), 1, "box live créée dès le setup valide");
    }

    #[test]
    fn box_live_supprimee_des_que_invalide() {
        // Pine : « créé/prolongé tant que le setup est valable, supprimé dès
        // qu'il ne l'est plus » — sweep non frais ⇒ box supprimée.
        let mut det = ZoneCoeurDetector::new();
        let obs = [ob(108.0, 100.0, 5)];
        let fvgs = [fvg(106.0, 102.0)];
        let ote = Some((107.0, 101.0));
        let ev1 = det.update(&obs, &[], &fvgs, &[], ote, None, true, false, Some(150.0));
        assert_eq!(ev1.live_bull.len(), 1);
        let ev2 = det.update(&obs, &[], &fvgs, &[], ote, None, false, false, Some(150.0));
        assert!(
            ev2.live_bull.is_empty(),
            "sweep non frais ⇒ box live supprimée"
        );
    }

    #[test]
    fn box_live_recreee_apres_revalidation() {
        // Invalide (sweep non frais) puis re-valide ⇒ box recréée (Pine box.new).
        let mut det = ZoneCoeurDetector::new();
        let obs = [ob(108.0, 100.0, 5)];
        let fvgs = [fvg(106.0, 102.0)];
        let ote = Some((107.0, 101.0));
        det.update(&obs, &[], &fvgs, &[], ote, None, true, false, Some(150.0));
        det.update(&obs, &[], &fvgs, &[], ote, None, false, false, Some(150.0));
        let ev3 = det.update(&obs, &[], &fvgs, &[], ote, None, true, false, Some(150.0));
        assert_eq!(ev3.live_bull.len(), 1, "re-validation ⇒ box live recréée");
        assert_eq!(ev3.live_bull[0].ob_bar, 5);
    }

    #[test]
    fn box_live_supprimee_si_ob_parent_disparait() {
        // Pine lignes 1298-1303 : suppression de l'OB ⇒ box shiftée/supprimée.
        let mut det = ZoneCoeurDetector::new();
        let fvgs = [fvg(106.0, 102.0)];
        let ote = Some((107.0, 101.0));
        det.update(
            &[ob(108.0, 100.0, 5)],
            &[],
            &fvgs,
            &[],
            ote,
            None,
            true,
            false,
            Some(150.0),
        );
        let ev2 = det.update(&[], &[], &fvgs, &[], ote, None, true, false, Some(150.0));
        assert!(
            ev2.live_bull.is_empty(),
            "OB parent disparu ⇒ box live supprimée"
        );
    }

    #[test]
    fn box_live_bornes_figees_a_la_creation() {
        // Pine ne fait que box.set_right sur une box existante : les bornes
        // restent celles de la création même si l'intersection bouge.
        let mut det = ZoneCoeurDetector::new();
        let obs = [ob(108.0, 100.0, 5)];
        let ote = Some((107.0, 101.0));
        det.update(
            &obs,
            &[],
            &[fvg(106.0, 102.0)],
            &[],
            ote,
            None,
            true,
            false,
            Some(150.0),
        );
        // FVG plus large à la barre suivante : intersection ≠, box déjà créée.
        let ev2 = det.update(
            &obs,
            &[],
            &[fvg(107.0, 101.0)],
            &[],
            ote,
            None,
            true,
            false,
            Some(150.0),
        );
        assert_eq!(ev2.live_bull.len(), 1);
        assert!(
            (ev2.live_bull[0].top - 106.0).abs() < 1e-9,
            "bornes figées à la création"
        );
        assert!(
            (ev2.live_bull[0].bot - 102.0).abs() < 1e-9,
            "bornes figées à la création"
        );
    }
}
