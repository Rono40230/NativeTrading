//! MODULE 6b — BPR (Balanced Price Range).
//!
//! Reproduit MODULE 6b Pine (lignes 951-1096). Références : ICT mentorship
//! 2023 (chevauchement de 2 FVG opposés, spike news) · LuxAlgo BPR (fenêtre
//! d'appariement 10 bars, intersection stricte, le gap le plus RÉCENT fixe le
//! rôle : FVG bull récent sur FVG bear = support ; miroir = résistance).
//!
//! - Appariement : le FVG NAISSANT est confronté au pool du sens OPPOSÉ
//!   **pré-lifecycle** (le Pine apparie avant `f_fvg*BearLifecycle` : un gap
//!   opposé dont la clôture de remplissage le retire du pool cette bar reste
//!   appariantable). Le plus récent qui chevauche dans la fenêtre gagne.
//! - Intersection stricte `min(tops) > max(bots)`.
//! - Anti-doublon : recouvrement ≥ 80% avec un BPR ACTIF uniquement (une zone
//!   morte peut renaître au même endroit).
//! - FIFO 20 zones (actives + figées) — « Show Last 20 » LuxAlgo.
//! - Lifecycle : âge max 15 bars (ACTIF) · invalidation = clôture au-delà du
//!   bord LOINTAIN · états partiel (entrée, CE intacte) / profond (CE atteinte).
//! - Mort ≠ suppression : la zone est figée (`dead`), retirée du scoring et de
//!   l'anti-doublon, conservée jusqu'à l'éviction FIFO (comportement LuxAlgo
//!   « violated zones kept », décision propriétaire 28/08).

// ── Types BPR (déplacés de types.rs — règle < 600 lignes) ────────────────────

/// État d'un BPR (Pine `bprState`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BprState {
    /// `0` — frais, prix non encore entré dans la zone.
    #[default]
    Fresh,
    /// `1` — entré, CE intacte (`low < top` pour un BPR bull).
    Partial,
    /// `2` — profond, CE atteinte (`low <= ce` pour un BPR bull).
    Deep,
}

/// Une zone BPR (Pine : arrays parallèles `bpr*`).
#[derive(Debug, Clone, Copy)]
pub struct BprZone {
    pub top: f64,
    pub bot: f64,
    pub state: BprState,
    /// `bprBar` (Pine) = bar de complétion du 2e gap (le plus récent).
    pub bar: usize,
    /// `bprIsBull` (Pine) — rôle = sens du gap le plus récent.
    pub is_bull: bool,
    /// `bprDead` (Pine) — figé (clôture au-delà du bord lointain ou âge > 15).
    /// Reste exposé à l'affichage (grisé) jusqu'à l'éviction FIFO.
    pub dead: bool,
}

/// Événement BPR pour une bar.
#[derive(Debug, Clone, Default)]
pub struct BprEvent {
    /// BPR créé cette bar (None si rien).
    pub new: Option<BprZone>,
    /// Zones actives (non figées) après lifecycle.
    pub actifs: usize,
}

use super::types::{BarInput, FvgEvent, FvgZone};

/// `i_bprWindow` (Pine ligne 960) — bars max entre les origines des 2 gaps.
pub const BPR_WINDOW: i64 = 10;
/// `i_maxBPR` (Pine ligne 961) — zones conservées (actives + figées).
pub const MAX_BPR: usize = 20;
/// `i_bprMaxAge` (Pine ligne 962) — âge max (bars) d'un BPR ACTIF.
pub const BPR_MAX_AGE: i64 = 15;

/// Bonus de scoring `f_bprBonus` (Pine lignes 1088-1096) — chevauchement avec
/// un BPR ACTIF de même sens : +4 frais · +3 partiel · +1 profond.
/// Bonifie la QUALIFICATION de zone (OB v11 + BSZones), pas le contexte.
pub fn bonus_bpr(zones: &[BprZone], is_bull: bool, z_top: f64, z_bot: f64) -> i32 {
    let mut bonus = 0;
    for z in zones {
        if bonus >= 4 {
            break; // le max possible est atteint (Pine `_bonus < 4`)
        }
        if z.dead || z.is_bull != is_bull {
            continue;
        }
        // Chevauchement strict (Pine : `bprTop > zBot and bprBot < zTop`).
        if z.top > z_bot && z.bot < z_top {
            let v = match z.state {
                BprState::Fresh => 4,
                BprState::Partial => 3,
                BprState::Deep => 1,
            };
            bonus = bonus.max(v);
        }
    }
    bonus
}

/// Détecteur de BPR — appariement + lifecycle + FIFO.
#[derive(Clone)]
pub struct BprDetector {
    zones: Vec<BprZone>,
    bar_count: usize,
    last_event: BprEvent,
}

impl BprDetector {
    pub fn new() -> Self {
        Self {
            zones: Vec::with_capacity(MAX_BPR + 1),
            bar_count: 0,
            last_event: BprEvent::default(),
        }
    }

    /// Traite une bar. `fvg_ev` porte le gap NAISSANT (bull ou bear — jamais
    /// les deux : géométriquement mutuellement exclusifs) ;
    /// `opp_bull_pre`/`opp_bear_pre` sont les pools FVG **pré-lifecycle**
    /// (snapshot pris par le moteur AVANT `fvg.update`, fidélité Pine).
    pub fn update(
        &mut self,
        bar: &BarInput,
        fvg_ev: &FvgEvent,
        opp_bull_pre: &[FvgZone],
        opp_bear_pre: &[FvgZone],
    ) -> BprEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;

        // ── Appariement (Pine lignes 1033-1037) ──
        let mut new = None;
        if fvg_ev.is_fvg_bull {
            if let Some(nz) = &fvg_ev.new_bull {
                new = self.pair(true, nz, opp_bear_pre, cur_idx);
            }
        } else if fvg_ev.is_fvg_bear {
            if let Some(nz) = &fvg_ev.new_bear {
                new = self.pair(false, nz, opp_bull_pre, cur_idx);
            }
        }

        // ── Lifecycle (Pine f_bprLifecycle, ligne 1082 — APRÈS la naissance :
        //    un BPR né cette bar est évalué par le lifecycle cette même bar) ──
        self.lifecycle(bar, cur_idx);

        let ev = BprEvent {
            new,
            actifs: self.zones.iter().filter(|z| !z.dead).count(),
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> BprEvent {
        self.last_event.clone()
    }
    /// Zones conservées (actives + figées) — pour l'affichage et le scoring
    /// (le scoring filtre les `dead` via [`bonus_bpr`]).
    pub fn zones(&self) -> &[BprZone] {
        &self.zones
    }
    /// Zones actives uniquement.
    pub fn zones_actifs(&self) -> impl Iterator<Item = &BprZone> {
        self.zones.iter().filter(|z| !z.dead)
    }

    /// `f_bprPair` (Pine lignes 981-1031) — scan du pool opposé du plus
    /// récent au plus ancien, premier chevauchement dans la fenêtre.
    fn pair(
        &mut self,
        new_is_bull: bool,
        nz: &FvgZone,
        opp: &[FvgZone],
        cur_idx: usize,
    ) -> Option<BprZone> {
        if opp.is_empty() {
            return None;
        }
        // `_nBar = bar_index - 2` (Pine ligne 983) = origine du gap naissant.
        let n_bar = nz.bar as i64;
        let mut hit: Option<usize> = None;
        for k in (0..opp.len()).rev() {
            let g_bar = opp[k].bar as i64;
            if n_bar >= g_bar && n_bar - g_bar <= BPR_WINDOW {
                let it = nz.top.min(opp[k].top);
                let ib = nz.bot.max(opp[k].bot);
                if it > ib {
                    hit = Some(k);
                    break;
                }
            }
        }
        let k = hit?;
        let top = nz.top.min(opp[k].top);
        let bot = nz.bot.max(opp[k].bot);

        // Anti-doublon ≥ 80% sur les BPR ACTIFS uniquement (Pine lignes 997-1004).
        let dup = self.zones.iter().any(|p| {
            if p.dead {
                return false;
            }
            let d_t = top.min(p.top);
            let d_b = bot.max(p.bot);
            let min_h = (top - bot).min(p.top - p.bot);
            min_h > 0.0 && (d_t - d_b) / min_h >= 0.8
        });
        if dup {
            return None;
        }

        // FIFO (actives + figées) — Pine lignes 1006-1018.
        if self.zones.len() >= MAX_BPR {
            self.zones.remove(0);
        }
        let z = BprZone {
            top,
            bot,
            state: BprState::Fresh,
            bar: cur_idx,
            is_bull: new_is_bull,
            dead: false,
        };
        self.zones.push(z);
        Some(z)
    }

    /// `f_bprLifecycle` (Pine lignes 1043-1080). Un BPR figé reste dans le
    /// pool (grisé) : plus aucun état ni invalidation ne s'applique.
    fn lifecycle(&mut self, bar: &BarInput, cur_idx: usize) {
        for z in self.zones.iter_mut() {
            if z.dead {
                continue;
            }
            let ce = (z.top + z.bot) / 2.0;
            let old = (cur_idx as i64 - z.bar as i64) > BPR_MAX_AGE;
            let dead = old || if z.is_bull {
                bar.close < z.bot
            } else {
                bar.close > z.top
            };
            if dead {
                z.dead = true;
            } else {
                // État : profond à la CE, sinon partiel dès l'entrée — STICKY
                // (Pine 1064-1065 : `math.max(_st, 1)` — jamais de retour en arrière).
                z.state = if z.is_bull {
                    if bar.low <= ce {
                        BprState::Deep
                    } else if bar.low < z.top && z.state == BprState::Fresh {
                        BprState::Partial
                    } else {
                        z.state
                    }
                } else if bar.high >= ce {
                    BprState::Deep
                } else if bar.high > z.bot && z.state == BprState::Fresh {
                    BprState::Partial
                } else {
                    z.state
                };
            }
        }
    }
}

impl Default for BprDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::FvgState;

    fn bar(i: usize, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: i as i64,
            open: close,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    /// Zone FVG générique (top > bot).
    fn fz(top: f64, bot: f64, bar_origin: usize) -> FvgZone {
        FvgZone {
            top,
            bot,
            state: FvgState::Fresh,
            bar: bar_origin,
        }
    }

    /// Événement « FVG bull naissant » (is_fvg_bull cohérent avec new_bull).
    fn ev_bull(new: Option<FvgZone>) -> FvgEvent {
        FvgEvent {
            is_fvg_bull: new.is_some(),
            new_bull: new,
            ..Default::default()
        }
    }

    /// Naissance : FVG bull naissant [104..108] chevauche FVG bear [104..110]
    /// → BPR bull = intersection [104..108], bar = bar courante (0).
    /// Géométrie fidèle : à la naissance, low = top du gap naissant (108).
    #[test]
    fn bpr_bull_ne_de_l_intersection() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)];
        let ev = det.update(&bar(0, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        let z = ev.new.expect("BPR créé");
        assert!(z.is_bull, "gap le plus récent = bull ⇒ rôle support");
        assert_eq!(z.top, 108.0, "top = min(108, 110)");
        assert_eq!(z.bot, 104.0, "bot = max(104, 104)");
        assert_eq!(z.bar, 0, "bar de naissance = bar courante");
        assert!(!z.dead);
        assert_eq!(z.state, BprState::Fresh, "low=top ⇒ ni partiel ni profond à la naissance");
    }

    /// Pas d'intersection stricte → aucun BPR.
    #[test]
    fn bpr_rejete_si_pas_d_intersection() {
        let mut det = BprDetector::new();
        let nz = fz(120.0, 112.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)]; // disjoint (112 > 110)
        let ev = det.update(&bar(0, 122.0, 120.0, 121.0), &ev_bull(Some(nz)), &[], &opp);
        assert!(ev.new.is_none());
    }

    /// Fenêtre : origines écartées de > 10 bars → pas d'appariement.
    #[test]
    fn bpr_rejete_hors_fenetre_10_bars() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 20); // origine bar 20 ; opposé né bar 5 → écart 15 > 10
        let opp = vec![fz(110.0, 104.0, 5)];
        let ev = det.update(&bar(22, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        assert!(ev.new.is_none(), "écart origines 15 > fenêtre 10");
    }

    /// Le plus RÉCENT du pool l'emporte (scan fin → début).
    #[test]
    fn bpr_apparie_le_plus_recent() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 10);
        // Ancien [100..106] (chevauche aussi) + récent [104..110] — tous deux dans la fenêtre.
        let opp = vec![fz(106.0, 100.0, 2), fz(110.0, 104.0, 8)];
        let ev = det.update(&bar(12, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        let z = ev.new.unwrap();
        assert_eq!(z.top, 108.0, "intersection avec le RÉCENT [104..110]");
        assert_eq!(z.bot, 104.0);
    }

    /// Invalidation : clôture au-delà du bord LOINTAIN → figé (dead), conservé.
    #[test]
    fn bpr_figee_sur_cloture_au_travers() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)];
        det.update(&bar(0, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        assert_eq!(det.zones().len(), 1);
        // Bar 3 : clôture 102 < bot 104 ⇒ figé (BPR bull, bord lointain = bot).
        det.update(&bar(3, 103.0, 101.0, 102.0), &FvgEvent::default(), &[], &[]);
        assert_eq!(det.zones().len(), 1, "figé ≠ supprimé (LuxAlgo kept)");
        assert!(det.zones()[0].dead);
        assert_eq!(det.zones_actifs().count(), 0);
    }

    /// Âge max : BPR actif figé après > 15 bars sans invalidation.
    #[test]
    fn bpr_figee_par_age_15() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)];
        det.update(&bar(0, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        // 15 bars de range au-dessus (ni invalidation ni entrée).
        for i in 1..=16usize {
            det.update(&bar(i, 113.0, 109.0, 112.0), &FvgEvent::default(), &[], &[]);
        }
        // bar 16 : âge = 16 - 0 = 16 > 15 ⇒ figé.
        assert!(det.zones()[0].dead, "âge > 15 ⇒ figé");
    }

    /// États : partiel dès l'entrée (low < top), profond à la CE — sticky.
    #[test]
    fn bpr_etats_partiel_puis_profond_sticky() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)];
        det.update(&bar(0, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        // CE = (108+104)/2 = 106. Bar 2 : low 107 < top 108, close 111 (alive) ⇒ partiel.
        det.update(&bar(2, 111.0, 107.0, 110.0), &FvgEvent::default(), &[], &[]);
        assert_eq!(det.zones()[0].state, BprState::Partial);
        // Bar 4 : low 105.5 <= CE 106 ⇒ profond.
        det.update(&bar(4, 110.0, 105.5, 109.0), &FvgEvent::default(), &[], &[]);
        assert_eq!(det.zones()[0].state, BprState::Deep);
        // Bar 6 : remontée (low 107 > CE) ⇒ l'état RESTE profond (math.max Pine).
        det.update(&bar(6, 112.0, 107.0, 111.0), &FvgEvent::default(), &[], &[]);
        assert_eq!(det.zones()[0].state, BprState::Deep, "sticky : pas de retour profond→partiel");
    }

    /// Anti-doublon : un nouveau BPR recouvrant ≥ 80% d'un ACTIF est rejeté ;
    /// il est accepté si l'ancien est FIGÉ (une zone morte peut renaître).
    #[test]
    fn bpr_anti_doublon_sur_actifs_uniquement() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)];
        det.update(&bar(0, 112.0, 108.0, 111.0), &ev_bull(Some(nz)), &[], &opp);
        // BPR quasi identique à la bar 7 (actif) → rejeté (recouvrement ~100%).
        let nz2 = fz(108.0, 104.0, 5);
        let opp2 = vec![fz(110.0, 104.0, 5)];
        let ev = det.update(&bar(7, 112.0, 108.0, 111.0), &ev_bull(Some(nz2)), &[], &opp2);
        assert!(ev.new.is_none(), "doublon d'un BPR actif rejeté");
        // Figé (clôture sous bot), puis même naissance → ACCEPTÉ.
        det.update(&bar(8, 103.0, 101.0, 102.0), &FvgEvent::default(), &[], &[]);
        let nz3 = fz(108.0, 104.0, 9);
        let opp3 = vec![fz(110.0, 104.0, 9)];
        let ev = det.update(&bar(11, 112.0, 108.0, 111.0), &ev_bull(Some(nz3)), &[], &opp3);
        assert!(ev.new.is_some(), "renaissance après figé autorisée");
    }

    /// FIFO 20 : au 21e BPR, la plus ancienne zone est évincée (actives+figées).
    #[test]
    fn bpr_fifo_limite_a_20() {
        let mut det = BprDetector::new();
        // Série de naissances à niveaux disjoints (anti-doublon contourné par
        // des paliers de 20 points) — low = top du gap naissant (géométrie fidèle).
        for k in 0..25usize {
            let base = 100.0 + 20.0 * k as f64;
            let nz = fz(base + 4.0, base, 3 * k);
            let opp = vec![fz(base + 6.0, base, 3 * k)];
            det.update(
                &bar(3 * k + 2, base + 7.0, base + 4.0, base + 10.0),
                &ev_bull(Some(nz)),
                &[],
                &opp,
            );
        }
        assert_eq!(det.zones().len(), MAX_BPR, "plafonné à 20 (FIFO)");
    }

    /// Géométrie : un BPR bull ne meurt JAMAIS à sa naissance
    /// (close ≥ low du gap naissant = top ≥ bot).
    #[test]
    fn bpr_bull_jamais_mort_a_la_naissance() {
        let mut det = BprDetector::new();
        let nz = fz(108.0, 104.0, 0);
        let opp = vec![fz(110.0, 104.0, 0)];
        // close le plus bas possible = low du gap naissant = 108.
        let ev = det.update(&bar(0, 109.0, 108.0, 108.0), &ev_bull(Some(nz)), &[], &opp);
        let z = ev.new.unwrap();
        assert!(!z.dead, "close=108 ≥ top=108 ⇒ jamais < bot");
    }

    /// BPR bear : rôle résistance, invalidation = clôture AU-DESSUS du top.
    #[test]
    fn bpr_bear_figee_sur_cloture_au_dessus() {
        let mut det = BprDetector::new();
        // FVG bear naissant [104..110] chevauche FVG bull [104..108].
        let ev_bear = FvgEvent {
            is_fvg_bear: true,
            new_bear: Some(fz(110.0, 104.0, 0)),
            ..Default::default()
        };
        let opp = vec![fz(108.0, 104.0, 0)];
        // À la naissance bear : high = bot du gap naissant = 104.
        let ev = det.update(&bar(0, 104.0, 100.0, 101.0), &ev_bear, &opp, &[]);
        let z = ev.new.expect("BPR bear créé");
        assert!(!z.is_bull, "gap le plus récent = bear ⇒ rôle résistance");
        assert_eq!(z.top, 108.0);
        assert_eq!(z.bot, 104.0);
        // Clôture 111 > top 108 ⇒ figé.
        det.update(&bar(2, 112.0, 109.0, 111.0), &FvgEvent::default(), &[], &[]);
        assert!(det.zones()[0].dead, "close > top ⇒ BPR bear figé");
    }

    /// `bonus_bpr` : +4 frais · +3 partiel · +1 profond · 0 si figé ou sens opposé.
    #[test]
    fn bonus_valeurs_et_filtres() {
        let z = |state: BprState, dead: bool, is_bull: bool| BprZone {
            top: 110.0,
            bot: 100.0,
            state,
            bar: 0,
            is_bull,
            dead,
        };
        let zones = vec![
            z(BprState::Fresh, false, true),
            z(BprState::Partial, false, true),
            z(BprState::Deep, false, true),
            z(BprState::Fresh, true, true),   // figé → ignoré
            z(BprState::Fresh, false, false), // bear actif → invisible côté bull
        ];
        // Zone candidate [105..109] chevauche les BPR [100..110].
        assert_eq!(bonus_bpr(&zones, true, 109.0, 105.0), 4, "le max des actifs = Fresh");
        // Sens opposé : un ensemble bull uniquement → 0 pour une requête bear.
        let bulls: Vec<BprZone> = zones[..4].to_vec();
        assert_eq!(bonus_bpr(&bulls, false, 109.0, 105.0), 0, "aucun BPR bear actif");
        // Chevauchement strict requis : disjoint → 0.
        assert_eq!(bonus_bpr(&zones, true, 200.0, 190.0), 0);
        // Partiel seul.
        let part = vec![z(BprState::Partial, false, true)];
        assert_eq!(bonus_bpr(&part, true, 109.0, 105.0), 3);
        // Profond seul.
        let deep = vec![z(BprState::Deep, false, true)];
        assert_eq!(bonus_bpr(&deep, true, 109.0, 105.0), 1);
        // Figé seul.
        let dead = vec![z(BprState::Fresh, true, true)];
        assert_eq!(bonus_bpr(&dead, true, 109.0, 105.0), 0);
    }
}

