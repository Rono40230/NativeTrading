//! OTE swing institutionnelle — bande 61,8–78,6 % + trait 50 % de la jambe.
//!
//! MÉCANISME CIBLE ÉLUCIDÉ 28/10 (D4) — lu sur les captures « vague + pivots »
//! de l'indicateur cible TV, réglages à l'appui (« PIVOT POINTS : Number of
//! bars = 2 »), puis validé par rejeu 20/24 contre les lectures owner
//! (XAU M1→H1, dont l'état intermédiaire M30 17h05 EXACT au dixième) :
//!
//! 1. **Pivots fractals 2-2, uniformes sur tous les TF** : gauche ≥ (égalités
//!    tolérées), droite > strict ; confirmation à l'arrivée de la 2e barre à
//!    droite (formante en live avec ses valeurs partielles).
//! 2. **Chaînes à RENOUVELLEMENT** : l'ancre haute ne bouge que si un pivot
//!    haut la DÉPASSE ; l'ancre basse que si un pivot bas passe DESSOUS. Un
//!    double sommet (égal) ne renouvelle pas — d'où le fibo H1 resté sur
//!    4033,8→3975,6 pendant que le M30 basculait (captures 27/10).
//! 3. **Recalage croisé au renouvellement** : à chaque renouvellement d'un
//!    côté, l'autre ancre se recale sur l'extrême de TOUS ses pivots depuis
//!    le renouvellement précédent (renouvellements inclus) — recale
//!    INCONDITIONNELLE, qui peut faire REMONTER l'ancre (M5 27/10 :
//!    3978 → 4056,5 après les renouvellements du sommet, confirmé owner).
//! 4. **Zéro extrême vivant** : une mèche qui casse l'ancre ne redessine
//!    RIEN tant que son pivot n'est pas confirmé (preuve owner : mèche M5
//!    4055,5 cassant l'ancre 4056,5 sans redessin).
//! 5. **Barre outside** : quand les DEUX fractales tombent sur la même
//!    barre, chaque recalage lit sa fenêtre SANS les pivots de cette barre
//!    (preuve TV H4 23/10 : la bougie 13h-17h porte le creux 3975,57 ET le
//!    rebond 4033,80 — le renouvellement bas garde le sommet 4217,24 du
//!    12/10 au lieu de recaler sur le rebond).
//! 6. Sens = récence des ancres ; zone = fib standard vers l'origine
//!    (61,8-78,6 % + 50 %).
//!
//! Les anciennes croyances du 12/10 (N=4/5 par TF, remplacement par fraîcheur
//! « staircase », extrêmes vivants) étaient des mésinterprétations — les
//! « preuves » de l'époque se réexpliquent par le recalage croisé. Résidus
//! ouverts : sémantique du recalage DESCENDANT (H4/D1 à confirmer par
//! captures), égalités de pivots à gauche (≥ retenu).
//!
//! Isolation totale : aucun module moteur lu, aucun consommateur
//! scoring/signals (affichage uniquement).

use std::collections::VecDeque;

use super::types::{BarInput, SwingAnchor, SwingOteEvent, SwingOteZone};

/// Barres à GAUCHE du pivot candidat — réglage cible « Number of bars = 2 ».
const GAUCHE: usize = 2;
/// Barres à DROITE (strictes) — idem.
const DROITE: usize = 2;

const FIB_MID: f64 = 0.5;
/// Borne « profonde » 61,8 % (proche de l'équilibre de la jambe).
const FIB_618: f64 = 0.618;
/// Borne « extrême » 78,6 % (proche de l'origine du mouvement).
const FIB_786: f64 = 0.786;

/// Détecteur OTE : pivots 2-2 → chaînes à renouvellement + recalage croisé
/// → zone fib. Zéro vivant : l'affichage ne suit que les pivots confirmés.
#[derive(Clone)]
pub struct SwingOteDetector {
    /// Fenêtre glissante GAUCHE+DROITE+1 barres (ts, high, low) — candidate =
    /// barre GAUCHE crans en arrière ; la dernière barre peut être formante.
    fenetre: VecDeque<(i64, f64, f64)>,
    /// Ancre haute : dernier pivot haut RENOUVELLEMENT (qui a dépassé le précédent).
    h_ancre: Option<SwingAnchor>,
    /// Ancre basse : miroir.
    l_ancre: Option<SwingAnchor>,
    /// Tous les pivots hauts depuis le dernier renouvellement bas (recalage H).
    pivots_h_depuis_l: Vec<SwingAnchor>,
    /// Tous les pivots bas depuis le dernier renouvellement haut (recalage L).
    pivots_l_depuis_h: Vec<SwingAnchor>,
    zone: Option<SwingOteZone>,
    last_event: SwingOteEvent,
}

impl Default for SwingOteDetector {
    fn default() -> Self {
        Self::new(900)
    }
}

/// Pousse un pivot en idempotence barre formante (même ts ⇒ remplace le prix).
fn pousser_pivot(liste: &mut Vec<SwingAnchor>, a: SwingAnchor) {
    if liste.last().map(|d| d.ts) == Some(a.ts) {
        let n = liste.len();
        liste[n - 1] = a;
    } else {
        liste.push(a);
    }
}

impl SwingOteDetector {
    /// `tf_sec` conservé pour compatibilité appelant — le mécanisme cible est
    /// uniforme (pivots 2-2), le TF n'entre plus en jeu.
    pub fn new(_tf_sec: i64) -> Self {
        Self {
            fenetre: VecDeque::with_capacity(GAUCHE + DROITE + 1),
            h_ancre: None,
            l_ancre: None,
            pivots_h_depuis_l: Vec::new(),
            pivots_l_depuis_h: Vec::new(),
            zone: None,
            last_event: SwingOteEvent::default(),
        }
    }

    pub fn update(&mut self, bar: &BarInput) -> SwingOteEvent {
        self.last_event = SwingOteEvent::default();

        // Barre formante en live : même ts ⇒ remplacement (la fenêtre ne
        // doit garder qu'une version par barre).
        if self.fenetre.back().map(|(ts, _, _)| *ts) == Some(bar.timestamp) {
            if let Some(derniere) = self.fenetre.back_mut() {
                derniere.1 = bar.high;
                derniere.2 = bar.low;
            }
        } else {
            self.fenetre.push_back((bar.timestamp, bar.high, bar.low));
            if self.fenetre.len() > GAUCHE + DROITE + 1 {
                self.fenetre.pop_front();
            }
        }

        self.confirmer_pivots();
        self.calculer_zone();
        self.last_event.zone = self.zone;
        self.last_event.clone()
    }

    /// Fractale 2-2 (gauche ≥, droite >) puis chaînes à renouvellement avec
    /// recalage croisé de l'ancre opposée (voir module doc).
    fn confirmer_pivots(&mut self) {
        if self.fenetre.len() < GAUCHE + DROITE + 1 {
            return;
        }
        let (c_ts, c_h, c_l) = self.fenetre[GAUCHE];

        let fractal_h = (0..GAUCHE).all(|i| c_h >= self.fenetre[i].1)
            && (GAUCHE + 1..=GAUCHE + DROITE).all(|i| c_h > self.fenetre[i].1);
        let fractal_b = (0..GAUCHE).all(|i| c_l <= self.fenetre[i].2)
            && (GAUCHE + 1..=GAUCHE + DROITE).all(|i| c_l < self.fenetre[i].2);

        // Barre OUTSIDE — les deux fractales sur la même barre : les
        // recalages lisent les fenêtres SANS les pivots de cette barre
        // (voir module doc point 5 — preuve TV H4 23/10).
        if fractal_h && fractal_b {
            let ext_l = self
                .pivots_l_depuis_h
                .iter()
                .min_by(|x, y| x.prix.total_cmp(&y.prix))
                .copied();
            let ext_h = self
                .pivots_h_depuis_l
                .iter()
                .max_by(|x, y| x.prix.total_cmp(&y.prix))
                .copied();
            let a_h = SwingAnchor { prix: c_h, ts: c_ts };
            let a_b = SwingAnchor { prix: c_l, ts: c_ts };
            if self.h_ancre.map_or(true, |h| c_h > h.prix) {
                if let Some(min) = ext_l {
                    self.l_ancre = Some(min);
                }
                self.h_ancre = Some(a_h);
                self.pivots_l_depuis_h.clear();
            }
            if self.l_ancre.map_or(true, |l| c_l < l.prix) {
                if let Some(max) = ext_h {
                    self.h_ancre = Some(max);
                }
                self.l_ancre = Some(a_b);
                self.pivots_h_depuis_l.clear();
            }
            // Les pivots de la barre nourrissent les fenêtres survivantes
            // APRÈS les vidages (même finalité que le flux simple).
            pousser_pivot(&mut self.pivots_h_depuis_l, a_h);
            pousser_pivot(&mut self.pivots_l_depuis_h, a_b);
            return;
        }

        if fractal_h {
            let a = SwingAnchor { prix: c_h, ts: c_ts };
            pousser_pivot(&mut self.pivots_h_depuis_l, a);
            if self.h_ancre.map_or(true, |h| c_h > h.prix) {
                // RENOUVELLEMENT HAUT : l'ancre basse se recale sur le plus
                // bas de TOUS ses pivots depuis le renouvellement haut
                // précédent — inconditionnel, peut remonter.
                if let Some(min) = self
                    .pivots_l_depuis_h
                    .iter()
                    .min_by(|x, y| x.prix.total_cmp(&y.prix))
                {
                    self.l_ancre = Some(*min);
                }
                self.h_ancre = Some(a);
                self.pivots_l_depuis_h.clear();
            }
        }
        if fractal_b {
            let a = SwingAnchor { prix: c_l, ts: c_ts };
            pousser_pivot(&mut self.pivots_l_depuis_h, a);
            if self.l_ancre.map_or(true, |l| c_l < l.prix) {
                // Miroir : renouvellement bas ⇒ recale de l'ancre haute.
                if let Some(max) = self
                    .pivots_h_depuis_l
                    .iter()
                    .max_by(|x, y| x.prix.total_cmp(&y.prix))
                {
                    self.h_ancre = Some(*max);
                }
                self.l_ancre = Some(a);
                self.pivots_h_depuis_l.clear();
            }
        }
    }

    /// Zone = fib 61,8–78,6 % + 50 % vers l'ORIGINE de la jambe ; sens =
    /// récence des ancres (la plus récente = côté mobile).
    fn calculer_zone(&mut self) {
        self.zone = None;
        let (Some(h), Some(l)) = (self.h_ancre, self.l_ancre) else {
            return;
        };
        if h.prix <= l.prix {
            return;
        }
        let a = h.prix - l.prix;
        let (top, bot, mid) = if l.ts > h.ts {
            (l.prix + FIB_786 * a, l.prix + FIB_618 * a, l.prix + FIB_MID * a)
        } else {
            (h.prix - FIB_618 * a, h.prix - FIB_786 * a, h.prix - FIB_MID * a)
        };
        self.zone = Some(SwingOteZone {
            bearish: l.ts > h.ts,
            top,
            bot,
            mid,
            // Bord gauche = ancre la plus ANCIENNE (captures cible).
            ts_naissance: h.ts.min(l.ts),
            haut: h,
            bas: l,
        });
    }

    pub fn last_event(&self) -> SwingOteEvent {
        self.last_event.clone()
    }

    /// Zone vivante au dernier update (collecte B2).
    pub fn zone(&self) -> Option<SwingOteZone> {
        self.zone
    }
}

/// Confluence dorée (spec § 2.3) : un OB est doré ⟺ son intersection de
/// prix avec l'OTE vivante est non vide (chevauchement ou inclusion, sans
/// condition de sens). Sans OTE vivante → jamais doré.
pub fn chevauche(ob_top: f64, ob_bot: f64, ote: Option<SwingOteZone>) -> bool {
    match ote {
        Some(z) => ob_bot <= z.top && ob_top >= z.bot,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Barre où seuls high/low nous intéressent (le détecteur ne lit que ça).
    pub(crate) fn barre(i: usize, high: f64, low: f64) -> BarInput {
        BarInput {
            timestamp: 1_700_000_000i64 + (i as i64) * 900,
            open: low,
            high,
            low,
            close: high,
            volume: 0.0,
        }
    }

    pub(crate) fn pousser(det: &mut SwingOteDetector, bars: &[(f64, f64)]) -> SwingOteEvent {
        let mut ev = SwingOteEvent::default();
        for (i, &(h, l)) in bars.iter().enumerate() {
            ev = det.update(&barre(i, h, l));
        }
        ev
    }

    /// Le renouvellement exige un DÉPASSEMENT strict : un sommet égal (double
    /// sommet) ne renouvelle pas — preuve H1 27/10 (4029,8 < 4033,8 gardé).
    #[test]
    fn double_sommet_ne_renouvelle_pas() {
        let mut det = SwingOteDetector::new(1800);
        // Creux 100, sommet 120, re-montée à 120.0 EXACT : l'ancre haute
        // garde le PREMIER sommet (égalité ⇒ pas de dépassement).
        pousser(
            &mut det,
            &[
                (110.0, 100.0), // 0
                (112.0, 105.0), // 1
                (120.0, 108.0), // 2 — futur pivot H
                (115.0, 109.0), // 3
                (113.0, 107.0), // 4 — confirme H@2
                (116.0, 104.0), // 5 — futur pivot L
                (118.0, 108.0), // 6
                (120.0, 111.0), // 7 — égale le sommet (égalité gauche OK)
                (117.0, 112.0), // 8
            ],
        );
        let z = det.zone().expect("zone vivante");
        assert_eq!(z.haut.prix, 120.0);
        assert_eq!(
            z.haut.ts,
            barre(0, 0.0, 0.0).timestamp + 2 * 900,
            "l'ancre garde le ts du PREMIER sommet"
        );
    }

    /// Recalage croisé MONTANT au renouvellement : après les renouvellements
    /// du sommet, l'ancre basse remonte sur le creux du pullback le plus bas
    /// depuis le renouvellement précédent (M5 27/10 : 3978 → 4056,5 owner).
    #[test]
    fn recale_montante_au_renouvellement() {
        let mut det = SwingOteDetector::new(300);
        pousser(
            &mut det,
            &[
                (105.0, 97.0),  // 0
                (104.0, 98.0),  // 1
                (103.0, 96.0),  // 2 — pivot L 96 (première ancre basse)
                (108.0, 99.0),  // 3
                (115.0, 100.0), // 4 — pivot H 115 : RENOUVELLEMENT initial
                (112.0, 106.0), // 5
                (110.0, 104.0), // 6
                (111.0, 103.0), // 7 — pivot L 103 (creux en V du pullback)
                (113.0, 105.0), // 8
                (118.0, 106.0), // 9 — pivot H 118 : RENOUVELLEMENT ⇒ recale L
                (116.0, 107.0), // 10
                (114.0, 108.0), // 11
            ],
        );
        let z = det.zone().expect("zone vivante");
        assert_eq!(z.haut.prix, 118.0, "ancre haute = dernier renouvellement");
        assert_eq!(
            z.bas.prix, 103.0,
            "ancre basse RECALÉE sur le creux du pullback (103), pas l'ancien 96"
        );
        assert!(!z.bearish, "ancre haute plus récente ⇒ haussière");
    }

    /// Zéro vivant : une mèche qui CASSE l'ancre basse sans pivot confirmé ne
    /// redessine rien (preuve owner : mèche M5 4055,5 vs ancre 4056,5).
    #[test]
    fn meche_cassante_sans_pivot_ne_redessine_rien() {
        let mut det = SwingOteDetector::new(900);
        pousser(
            &mut det,
            &[
                (110.0, 100.0), // 0
                (112.0, 102.0), // 1
                (118.0, 104.0), // 2 — pivot H
                (114.0, 105.0), // 3
                (112.0, 103.0), // 4 — confirme H@2
                (110.0, 101.0), // 5 — pivot L 101
                (113.0, 103.0), // 6
                (115.0, 105.0), // 7 — confirme L@5
                (112.0, 99.5),  // 8 — mèche SOUS l'ancre 101, pivot non confirmé
                (114.0, 102.0), // 9
            ],
        );
        let z = det.zone().expect("zone vivante");
        assert_eq!(z.bas.prix, 101.0, "la mèche non confirmée ne touche pas l'ancre");
    }

    /// Plateau parfait ⇒ aucune fractale (droite stricte) ⇒ aucune zone.
    #[test]
    fn plateau_aucune_zone() {
        let mut det = SwingOteDetector::new(900);
        let ev = pousser(&mut det, &[(4100.0, 4090.0); 12]);
        assert!(ev.zone.is_none(), "aucun pivot ⇒ aucune OTE");
    }

    /// CAS RÉEL XAU M5 27/10, 15h20→17h50 UTC (bars DB Axi) : les trois
    /// renouvellements du sommet (4064,47 → 4064,69 → 4065,71) recalent
    /// l'ancre basse sur le creux 4056,52 — capture TV owner : 4066/4056,5.
    #[test]
    fn cas_reel_m5_27_octobre_recale_montante() {
        let bars: &[(f64, f64)] = &[
            (4059.02, 4056.21), // 15:20
            (4059.22, 4055.74), // 15:25
            (4059.07, 4056.74), // 15:30
            (4058.65, 4055.55), // 15:35 — pivot L 4055,55
            (4064.47, 4058.45), // 15:40 — pivot H 4064,47 : renouvellement
            (4062.75, 4060.47), // 15:45
            (4064.08, 4061.62), // 15:50
            (4064.23, 4061.25), // 15:55
            (4064.69, 4059.90), // 16:00 — pivot H 4064,69 : renouvellement
            (4062.32, 4058.31), // 16:05
            (4061.20, 4057.67), // 16:10 — pivot L 4057,67
            (4061.18, 4058.25), // 16:15
            (4059.78, 4058.09), // 16:20
            (4059.32, 4056.52), // 16:25 — pivot L 4056,52 (l'ancre future)
            (4058.61, 4056.60), // 16:30
            (4060.98, 4058.08), // 16:35
            (4060.95, 4058.89), // 16:40
            (4062.58, 4059.26), // 16:45 — pivot H 4062,58 (PAS renouvellement)
            (4061.69, 4058.88), // 16:50
            (4060.39, 4058.29), // 16:55
            (4060.96, 4058.58), // 17:00
            (4061.10, 4059.63), // 17:05
            (4060.54, 4058.19), // 17:10 — pivot L 4058,19
            (4060.45, 4059.16), // 17:15
            (4061.74, 4059.26), // 17:20
            (4062.13, 4060.31), // 17:25
            (4065.24, 4060.88), // 17:30
            (4065.71, 4063.58), // 17:35 — pivot H 4065,71 : RENOUVELLEMENT
            (4064.41, 4062.85), // 17:40    ⇒ recale L := 4056,52 (capture TV)
            (4062.99, 4058.46), // 17:45
            (4059.42, 4057.72), // 17:50
        ];
        let mut det = SwingOteDetector::new(300);
        let ev = pousser(&mut det, bars);
        let z = ev.zone.expect("zone vivante");
        assert_eq!(z.haut.prix, 4065.71, "ancre haute = 4065,71 (TV : 4066)");
        assert_eq!(z.bas.prix, 4056.52, "ancre basse recalée = 4056,52 (TV : 4056,5)");
        assert!(!z.bearish, "haussière — TV owner");
        let a = 4065.71 - 4056.52;
        assert!((z.top - (4065.71 - 0.618 * a)).abs() < 1e-9);
        assert!((z.bot - (4065.71 - 0.786 * a)).abs() < 1e-9);
    }

    /// CAS RÉEL XAU M30, 23/10 12h00 → 27/10 17h00 UTC (bars DB Axi). Avant
    /// les renouvellements du 27 (15h30/16h00), l'état est l'état intermédiaire
    /// de la capture 18h05 : 4033,80 → 3975,57 BAISSIÈRE, OTE [4011,55 ;
    /// 4021,32] — le 26/10 le pivot H 4029,79 ÉCHOUE à renouveler 4033,80.
    /// Après 16h00 le sens bascule haussier, ancre basse tenue.
    #[test]
    fn cas_reel_m30_23_27_octobre_ancre_collante() {
        let bars: &[(f64, f64)] = &[
            (4030.22, 4021.47), // 23 12:00
            (4032.35, 4023.40), // 23 12:30
            (4033.80, 4024.53), // 23 13:00 — pivot H 4033,80 : l'ANCRE HAUTE
            (4029.05, 4016.15), // 23 13:30
            (4023.14, 3976.98), // 23 14:00
            (4005.60, 3975.57), // 23 14:30 — pivot L 3975,57 : l'ANCRE BASSE
            (4004.39, 3992.89), // 23 15:00
            (4005.70, 3993.39), // 23 15:30
            (4006.03, 4001.24), // 23 16:00
            (4006.11, 3998.64), // 23 16:30
            (4002.13, 3992.05), // 23 17:00
            (4000.45, 3990.86), // 23 17:30
            (4002.15, 3997.20), // 23 18:00
            (4002.11, 3994.46), // 23 18:30
            (3998.68, 3993.80), // 23 19:00
            (4001.49, 3997.59), // 23 19:30
            (3999.91, 3995.38), // 23 20:00
            (3998.38, 3991.89), // 23 20:30
            (4004.18, 3990.58), // 25 22:00
            (4003.59, 3997.65), // 25 22:30
            (4003.51, 3994.33), // 25 23:00
            (4003.15, 3989.37), // 25 23:30
            (3997.46, 3983.12), // 26 00:00
            (3995.81, 3983.44), // 26 00:30
            (4000.60, 3978.19), // 26 01:00 — pivot L 3978,19 (PAS renouvellement)
            (3991.24, 3982.76), // 26 01:30
            (3995.88, 3986.77), // 26 02:00
            (4002.15, 3990.31), // 26 02:30
            (4016.59, 3998.63), // 26 03:00
            (4015.49, 4007.45), // 26 03:30
            (4011.54, 4007.13), // 26 04:00
            (4016.16, 4010.55), // 26 04:30
            (4019.37, 4011.02), // 26 05:00
            (4016.99, 4009.53), // 26 05:30
            (4015.34, 4004.03), // 26 06:00
            (4009.88, 4003.68), // 26 06:30
            (4009.04, 4003.43), // 26 07:00
            (4011.39, 4006.25), // 26 07:30
            (4017.34, 4009.62), // 26 08:00
            (4022.30, 4013.84), // 26 08:30
            (4018.69, 4013.50), // 26 09:00
            (4020.53, 4016.22), // 26 09:30
            (4019.10, 4014.10), // 26 10:00
            (4016.28, 4010.86), // 26 10:30
            (4016.54, 4010.39), // 26 11:00
            (4020.62, 4012.59), // 26 11:30
            (4022.47, 4014.85), // 26 12:00
            (4025.51, 4015.20), // 26 12:30
            (4018.43, 4006.58), // 26 13:00
            (4010.18, 3988.69), // 26 13:30 — pivot L 3988,69 (pas renouvellement)
            (4015.28, 3992.84), // 26 14:00
            (4026.58, 4007.70), // 26 14:30
            (4025.94, 4016.49), // 26 15:00
            (4019.69, 4014.72), // 26 15:30
            (4021.01, 4015.51), // 26 16:00
            (4021.03, 4016.33), // 26 16:30
            (4029.79, 4020.11), // 26 17:00 — pivot H 4029,79 : ÉCHOUE (4033,80 tient)
            (4024.53, 4017.32), // 26 17:30
            (4021.42, 4016.83), // 26 18:00
            (4018.29, 4012.03), // 26 18:30
            (4012.88, 4008.38), // 26 19:00
            (4014.61, 4008.94), // 26 19:30
            (4014.54, 4011.38), // 26 20:00
            (4017.44, 4012.33), // 26 20:30
            (4018.88, 4014.33), // 26 22:00
            (4015.13, 4011.57), // 26 22:30
            (4014.17, 4010.62), // 26 23:00
            (4014.01, 4004.05), // 26 23:30
            (4019.96, 4004.29), // 27 00:00
            (4015.10, 4008.08), // 27 00:30
            (4017.13, 4003.79), // 27 01:00
            (4019.24, 4008.74), // 27 01:30
            (4018.11, 4009.51), // 27 02:00
            (4019.14, 4007.09), // 27 02:30
            (4011.02, 4005.76), // 27 03:00
            (4021.47, 4010.61), // 27 03:30
            (4025.62, 4016.81), // 27 04:00
            (4021.02, 4010.36), // 27 04:30
            (4011.66, 4004.93), // 27 05:00
            (4011.25, 3999.99), // 27 05:30
            (4012.02, 4001.88), // 27 06:00
            (4003.37, 3997.24), // 27 06:30
            (4007.88, 3998.92), // 27 07:00
            (4006.77, 3997.15), // 27 07:30
            (4002.09, 3992.51), // 27 08:00
            (3999.17, 3989.13), // 27 08:30 — pivot L 3989,13 (pas renouvellement)
            (4002.24, 3993.55), // 27 09:00
            (4003.17, 3995.31), // 27 09:30
            (4003.17, 3993.31), // 27 10:00
            (4016.24, 3993.71), // 27 10:30
            (4013.58, 4007.55), // 27 11:00
            (4014.27, 4008.45), // 27 11:30
            (4022.20, 4011.44), // 27 12:00
            (4028.57, 4018.75), // 27 12:30
            (4031.66, 4020.64), // 27 13:00
            (4032.09, 4020.92), // 27 13:30
            (4044.23, 4024.61), // 27 14:00
            (4054.62, 4041.90), // 27 14:30
            (4059.63, 4052.37), // 27 15:00
            (4064.47, 4055.55), // 27 15:30 — pivot H 4064,47 : RENOUVELLE
            (4064.69, 4056.52), // 27 16:00 — pivot H 4064,69 : RENOUVELLE
            (4062.58, 4056.60), // 27 16:30
            (4062.13, 4058.19), // 27 17:00
        ];
        let mut det = SwingOteDetector::new(1800);
        // ÉTAT INTERMÉDIAIRE — jusqu'au 26/10 inclus (44 premières barres +
        // re-formance du 27 matin hors renouvellements) : on coupe juste
        // avant le rallye du 27 (barre 27 12:00, index 86).
        for (i, &(h, l)) in bars.iter().enumerate().take(86) {
            det.update(&barre(i, h, l));
        }
        let z = det.zone().expect("zone intermédiaire");
        assert_eq!(z.haut.prix, 4033.80, "capture TV 18h05 : ancre haute 4033,8");
        assert_eq!(z.bas.prix, 3975.57, "capture TV 18h05 : ancre basse 3975,6");
        assert!(z.bearish, "capture TV 18h05 : baissière");
        let a = 4033.80 - 3975.57;
        assert!((z.bot - (3975.57 + 0.618 * a)).abs() < 1e-9, "bot = 4011,55");
        assert!((z.top - (3975.57 + 0.786 * a)).abs() < 1e-9, "top = 4021,32");

        // ÉTAT FINAL — après les renouvellements du 27 : bascule haussière,
        // ancre basse tenue (fenêtre de recale incluant l'origine).
        let ev = pousser(&mut det, &bars[86..]);
        let z = ev.zone.expect("zone finale");
        assert_eq!(z.haut.prix, 4064.69, "renouvellements 15h30/16h00");
        assert_eq!(z.bas.prix, 3975.57, "l'origine tient (recale inclut 3975,57)");
        assert!(!z.bearish, "haussière — TV owner 19h15");
    }

    /// Confluence dorée : overlap partiel / inclusion / disjonction / OTE
    /// absente — inchangée.
    #[test]
    fn confluence_doree() {
        let a = 4185.0 - 4067.0;
        let zone = SwingOteZone {
            bearish: true,
            top: 4067.0 + 0.786 * a,
            bot: 4067.0 + 0.618 * a,
            mid: 4067.0 + 0.5 * a,
            ts_naissance: 5,
            haut: SwingAnchor { prix: 4185.0, ts: 5 },
            bas: SwingAnchor { prix: 4067.0, ts: 8 },
        };
        assert!(chevauche(4145.0, 4138.0, Some(zone)), "OB inclus");
        assert!(chevauche(4165.0, 4158.0, Some(zone)), "chevauchement partiel");
        assert!(chevauche(4170.0, 4130.0, Some(zone)), "OTE incluse dans l'OB");
        assert!(!chevauche(4170.0, 4165.0, Some(zone)), "disjonction au-dessus");
        assert!(!chevauche(4135.0, 4120.0, Some(zone)), "disjonction en dessous");
        assert!(!chevauche(4145.0, 4138.0, None), "sans OTE ⇒ jamais doré");
    }
}

/// Cas réel H4 10→28/10 (barre outside du 23/10) — fichier séparé pour le
/// garde-fou 600 lignes.
#[cfg(test)]
#[path = "swing_ote_h4_test.rs"]
mod swing_ote_h4_test;
