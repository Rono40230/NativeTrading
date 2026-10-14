//! OTE swing institutionnelle — bande 61,8–78,6 % de la jambe structurelle.
//!
//! Rétro-ingénierie de l'indicateur cible TradingView (Pine protégé),
//! consolidée le 12/10/2026 par trois séries de preuves : captures live
//! (7 TF + relevés), Bar Replay (séquences A/B/C/D de l'owner) et simulation
//! contre les bougies réelles. Mécanisme :
//!
//! 1. **Pivots fractals N-barres** : high[i] ≥ max des N barres à GAUCHE
//!    (égalités tolérées) et > strictement les N barres à DROITE (preuve :
//!    le low M1 affiché à 12h39 et pas 12h38, tie 4154,58). Confirmation à
//!    l'ARRIVÉE de la barre i+N (style `ta.pivothigh` live : la barre
//!    N-ième formante compte avec ses valeurs partielles).
//!    **N=4 en M1/M5/M15** (à N=3, deux pivots fantômes contredits par la
//!    cible : M15 12h00 4173,15 et M5 13h55 4169,36) ; **N=5 au-dessus**
//!    (séquence D replay : le pivot M30 du 9/10 12h30 n'est visible qu'avec
//!    un retard ≥ 5 barres ; M30 5-vs-6 à trancher en côte à côte).
//! 2. **Deux chaînes indépendantes** (comme sh1/sl1 de notre Pine étalon) :
//!    chaque pivot confirmé remplace l'ancre de son côté — remplacement
//!    INCONDITIONNEL par fraîcheur, même moins extrême (preuve replay : le
//!    high M15 descend 4217 → 4193 → 4180 ; le high M1 staircase
//!    4178,18 → 4173,86 → 4173,21 → 4171,34, chaque cran à +N barres).
//! 3. **Affichage = chaîne ⊕ extrêmes vivants** : le côté mobile du marché
//!    affiche son extrême intrabar sans attendre de pivot (preuves : le low
//!    M1 replay ticking à la minute ; le low M15 4154,58 affiché dès 13h19
//!    alors que son pivot n'était confirmé qu'à 13h30). Concrètement :
//!    `haut = max(ancre H, plus haut depuis l'ancre L)` et
//!    `bas  = min(ancre L, plus bas depuis l'ancre H)`.
//! 4. **Zone** = fib 61,8–78,6 % de la jambe + trait 50 % (validée au
//!    dixième contre 2 captures : M15 4180,22→4154,58 ⇒ [4170,42 ; 4174,73],
//!    mid 4167,40). Le 0 du fib de la cible = l'ancre la plus récente
//!    (orientation affichage TV).
//!
//! Cas de référence réel (test ci-dessous) : XAU M15 12/10, bars 06h00→13h30
//! de la DB ⇒ ancres 4180,22@10h30 / 4154,58@12h30 = capture TV exacte.
//!
//! Isolation totale : aucun module moteur lu, aucun consommateur
//! scoring/signals (affichage uniquement).

use std::collections::VecDeque;

use super::types::{BarInput, SwingAnchor, SwingOteEvent, SwingOteZone};

/// Largeur fractale par timeframe — nombre de barres de chaque côté.
/// Fit du 12/10 contre captures + Bar Replay (voir module doc).
pub fn n_par_tf(tf_sec: i64) -> usize {
    match tf_sec {
        60 | 300 | 900 => 4, // M1, M5, M15
        _ => 5,              // M30, H1, H4, D1, W1
    }
}

const FIB_MID: f64 = 0.5;
/// Borne « profonde » 61,8 % (proche de l'équilibre de la jambe).
const FIB_618: f64 = 0.618;
/// Borne « extrême » 78,6 % (proche de l'origine du mouvement).
const FIB_786: f64 = 0.786;

/// Détecteur OTE : fractales N-barres → deux chaînes H/L indépendantes →
/// affichage chaîne ⊕ extrêmes vivants → zone fib.
#[derive(Clone)]
pub struct SwingOteDetector {
    n: usize,
    /// Fenêtre glissante 2N+1 barres (ts, high, low) — candidate = barre
    /// N crans en arrière ; la dernière barre peut être formante (live).
    fenetre: VecDeque<(i64, f64, f64)>,
    /// Chaînes : dernier pivot confirmé de chaque côté.
    chain_h: Option<SwingAnchor>,
    chain_l: Option<SwingAnchor>,
    /// Plus bas depuis l'ancre H / plus haut depuis l'ancre L (vivants).
    min_depuis_h: Option<(i64, f64)>,
    max_depuis_l: Option<(i64, f64)>,
    zone: Option<SwingOteZone>,
    last_event: SwingOteEvent,
}

impl Default for SwingOteDetector {
    fn default() -> Self {
        Self::new(900)
    }
}

impl SwingOteDetector {
    /// `tf_sec` : timeframe en secondes — utiliser [`n_par_tf`] est inutile,
    /// le constructeur dérive N lui-même.
    pub fn new(tf_sec: i64) -> Self {
        let n = n_par_tf(tf_sec);
        Self {
            n,
            fenetre: VecDeque::with_capacity(2 * n + 1),
            chain_h: None,
            chain_l: None,
            min_depuis_h: None,
            max_depuis_l: None,
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
            if self.fenetre.len() > 2 * self.n + 1 {
                self.fenetre.pop_front();
            }
        }

        self.confirmer_fractales();
        self.maj_extremes_vivants();
        self.calculer_zone();
        self.last_event.zone = self.zone;
        self.last_event.clone()
    }

    /// Vérifie la candidate de chaque côté (barre N crans en arrière) :
    /// gauche ≥/≤ toléré (égalités), droite >/< stricte. La confirmation a
    /// lieu à l'ARRIVÉE de la (i+N)-ième barre — formante en live, ses
    /// valeurs partielles comptent (style `ta.pivothigh` TV). La candidate
    /// remplace sa chaîne si plus récente (toujours vrai en flux ordonné ;
    /// la garde protège les re-formances de la barre courante).
    fn confirmer_fractales(&mut self) {
        let n = self.n;
        if self.fenetre.len() < 2 * n + 1 {
            return;
        }
        let (c_ts, c_h, c_l) = self.fenetre[n];

        let fractal_h = (0..n).all(|i| c_h >= self.fenetre[i].1)
            && (n + 1..=2 * n).all(|i| c_h > self.fenetre[i].1);
        let fractal_b = (0..n).all(|i| c_l <= self.fenetre[i].2)
            && (n + 1..=2 * n).all(|i| c_l < self.fenetre[i].2);

        if fractal_h && self.chain_h.map_or(true, |a| c_ts > a.ts) {
            self.chain_h = Some(SwingAnchor { prix: c_h, ts: c_ts });
            self.min_depuis_h = self.extreme_depuis(c_ts, true);
        }
        if fractal_b && self.chain_l.map_or(true, |a| c_ts > a.ts) {
            self.chain_l = Some(SwingAnchor { prix: c_l, ts: c_ts });
            self.max_depuis_l = self.extreme_depuis(c_ts, false);
        }
    }

    /// Extrême reconstitué depuis la fenêtre pour les barres entre une ancre
    /// fraîchement chaînée (N crans en arrière) et maintenant — plus bas des
    /// lows (`bas = true`) ou plus haut des highs.
    fn extreme_depuis(&self, ts: i64, bas: bool) -> Option<(i64, f64)> {
        let mut best: Option<(i64, f64)> = None;
        for &(t, h, l) in &self.fenetre {
            if t < ts {
                continue;
            }
            let v = if bas { l } else { h };
            let garde = match best {
                Some((_, bv)) => (bas && v < bv) || (!bas && v > bv),
                None => true,
            };
            if garde {
                best = Some((t, v));
            }
        }
        best
    }

    /// Extrêmes vivants : plus bas depuis l'ancre H, plus haut depuis l'ancre
    /// L — étendus à chaque barre. Idempotents : une barre formante ne peut
    /// qu'abaisser un min / élever un max, jamais l'inverse.
    fn maj_extremes_vivants(&mut self) {
        let Some(&(ts, high, low)) = self.fenetre.back() else { return };
        if let Some(h) = self.chain_h {
            if ts >= h.ts && self.min_depuis_h.map_or(true, |(_, p)| low < p) {
                self.min_depuis_h = Some((ts, low));
            }
        }
        if let Some(l) = self.chain_l {
            if ts >= l.ts && self.max_depuis_l.map_or(true, |(_, p)| high > p) {
                self.max_depuis_l = Some((ts, high));
            }
        }
    }

    /// Zone = fib 61,8–78,6 % + 50 % de la jambe affichée (chaîne ⊕ vivants).
    fn calculer_zone(&mut self) {
        self.zone = None;
        let (ch, cl) = match (self.chain_h, self.chain_l) {
            (Some(h), Some(l)) => (h, l),
            _ => return,
        };
        // Côté mobile : le vivant ne s'affiche que s'il DÉPASSE la chaîne
        // (séquence D replay : bas M30 = 4066 pivot, pas le running 4168).
        let haut = match self.max_depuis_l {
            Some((ts, p)) if p > ch.prix => SwingAnchor { prix: p, ts },
            _ => ch,
        };
        let bas = match self.min_depuis_h {
            Some((ts, p)) if p < cl.prix => SwingAnchor { prix: p, ts },
            _ => cl,
        };
        if haut.prix <= bas.prix {
            return;
        }
        let a = haut.prix - bas.prix;
        // Bas plus récent ⇒ jambe baissière (origine = haut, côté mobile =
        // bas) ; miroir pour la haussière.
        let bearish = bas.ts > haut.ts;
        let (top, bot, mid) = if bearish {
            (bas.prix + FIB_786 * a, bas.prix + FIB_618 * a, bas.prix + FIB_MID * a)
        } else {
            (haut.prix - FIB_618 * a, haut.prix - FIB_786 * a, haut.prix - FIB_MID * a)
        };
        self.zone = Some(SwingOteZone {
            bearish,
            top,
            bot,
            mid,
            // Bord gauche = ancre la plus ANCIENNE (captures cible).
            ts_naissance: haut.ts.min(bas.ts),
            haut,
            bas,
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
    fn barre(i: usize, high: f64, low: f64) -> BarInput {
        BarInput {
            timestamp: 1_700_000_000i64 + (i as i64) * 900,
            open: low,
            high,
            low,
            close: high,
            volume: 0.0,
        }
    }

    fn pousser(det: &mut SwingOteDetector, bars: &[(f64, f64)]) -> SwingOteEvent {
        let mut ev = SwingOteEvent::default();
        for (i, &(h, l)) in bars.iter().enumerate() {
            ev = det.update(&barre(i, h, l));
        }
        ev
    }

    /// Table N par TF — fit 12/10 (captures + replay + simulation).
    #[test]
    fn n_par_tf_calibration() {
        assert_eq!(n_par_tf(60), 4, "M1");
        assert_eq!(n_par_tf(300), 4, "M5");
        assert_eq!(n_par_tf(900), 4, "M15");
        assert_eq!(n_par_tf(1800), 5, "M30");
        assert_eq!(n_par_tf(3600), 5, "H1");
        assert_eq!(n_par_tf(14400), 5, "H4");
        assert_eq!(n_par_tf(86_400), 5, "D1");
        assert_eq!(n_par_tf(604_800), 5, "W1");
        assert_eq!(n_par_tf(12345), 5, "TF inconnu → 5");
    }

    /// Cas synthétique de référence : HH 4185 (bar 5), LL 4067 (bar 8),
    /// rebond 4145 et creux secondaire — la jambe SURVIT ; les deux barres
    /// finales confirment le LL (N=4 ⇒ confirmation à la barre 12).
    #[test]
    fn cas_reference_jambe_survit_aux_rebonds() {
        let mut det = SwingOteDetector::new(900);
        let ev = pousser(
            &mut det,
            &[
                (4100.0, 4090.0),
                (4110.0, 4095.0),
                (4165.0, 4120.0),
                (4140.0, 4125.0),
                (4140.0, 4130.0),
                (4185.0, 4140.0), // futur pivot H (confirme bar 9)
                (4165.0, 4110.0),
                (4138.0, 4072.0),
                (4132.0, 4067.0), // futur pivot L (confirme bar 12)
                (4145.0, 4120.0),
                (4130.0, 4110.0),
                (4135.0, 4115.0),
                (4128.0, 4112.0),
            ],
        );
        let z = ev.zone.expect("jambe H→L vivante après rebonds internes");
        assert!(z.bearish, "ancre basse plus récente ⇒ jambe baissière");
        assert_eq!(z.haut.prix, 4185.0);
        assert_eq!(z.haut.ts, barre(0, 0.0, 0.0).timestamp + 5 * 900);
        assert_eq!(z.bas.prix, 4067.0);
        assert_eq!(z.bas.ts, barre(0, 0.0, 0.0).timestamp + 8 * 900);
        let a = 4185.0 - 4067.0;
        assert_eq!(z.mid, 4126.0, "trait 50 % = 4126,0");
        assert!((z.bot - (4067.0 + 0.618 * a)).abs() < 1e-9);
        assert!((z.top - (4067.0 + 0.786 * a)).abs() < 1e-9);
    }

    /// Fraîcheur > extrémité : un pivot H PLUS BAS mais plus RÉCENT remplace
    /// l'ancre haute (staircase replay : 4217 → 4193 → 4180 en M15).
    #[test]
    fn remplacement_par_fraicheur_staircase() {
        let mut bars: Vec<(f64, f64)> = Vec::new();
        for i in 0..12 {
            // échauffement montant (aucun pivot H possible avant la fenêtre)
            bars.push((4110.0 + (i as f64) * 5.0, 4090.0 + (i as f64) * 5.0));
        }
        bars.push((4217.0, 4180.0)); // top 4217 @ 12
        for i in 0..7 {
            bars.push((4200.0 - (i as f64) * 2.0, 4160.0)); // déclin 13-19
        }
        bars.push((4195.0, 4178.0)); // top intermédiaire 4195 @ 20
        for i in 0..7 {
            bars.push((4186.0 - (i as f64) * 2.0, 4165.0 - (i as f64))); // déclin 21-27
        }
        bars.push((4180.0, 4170.0)); // top 4180 @ 28
        for i in 0..5 {
            bars.push((4172.0 - (i as f64) * 0.5, 4150.0 + (i as f64) * 4.0)); // reprise 29-33
        }
        let mut det = SwingOteDetector::new(900);
        let ev = pousser(&mut det, &bars);
        let z = ev.zone.expect("zone vivante");
        assert_eq!(z.haut.prix, 4180.0, "le pivot H le plus RÉCENT (4180) a remplacé 4217 puis 4195 — staircase");
        assert!(z.haut.prix < 4217.0, "l'ancre n'est PAS figée au maximum historique");
    }

    /// Côté mobile vivant : le bas affiché suit le creux intrabar SANS pivot
    /// confirmé (preuve cible : M15 12/10 — low 4154,58 affiché à 13h19, son
    /// pivot n'était confirmé qu'à 13h30). Ici : creux 4130@13 SOUS l'ancre L
    /// 4135@4, pivot non confirmé (il faudrait la barre 17) mais affiché.
    #[test]
    fn extremes_vivants_avant_confirmation() {
        let bars: &[(f64, f64)] = &[
            (4160.0, 4150.0),
            (4158.0, 4145.0),
            (4156.0, 4140.0),
            (4157.0, 4143.0),
            (4162.0, 4135.0), // creux 4135 @ 4 — futur pivot L
            (4168.0, 4140.0),
            (4172.0, 4146.0),
            (4176.0, 4152.0),
            (4178.0, 4156.0),
            (4180.0, 4160.0), // top 4180 @ 10 — pivot H (confirme bar 14)
            (4175.0, 4158.0),
            (4170.0, 4155.0),
            (4162.0, 4130.0), // creux vivant 4130 @ 13 — SOUS l'ancre L 4135
            (4166.0, 4133.0),
        ];
        let mut det = SwingOteDetector::new(900);
        let ev = pousser(&mut det, bars);
        let z = ev.zone.expect("zone vivante");
        let base = barre(0, 0.0, 0.0).timestamp;
        assert_eq!(z.bas.prix, 4130.0, "le bas VIVANT (4130) s'affiche avant confirmation du pivot");
        assert_eq!(z.bas.ts, base + 12 * 900, "creux de la barre 12, pas encore pivot (il faudrait la barre 16)");
        assert_eq!(z.haut.prix, 4180.0);
    }

    /// CAS RÉEL XAU M15 du 12/10 (bars DB 06h00→13h30 UTC) : la sortie doit
    /// reproduire la capture TV exacte — ancres 4180,22@10h30 /
    /// 4154,58@12h30, zone [4170,42 ; 4174,73], mid 4167,40, baissière.
    #[test]
    fn cas_reel_m15_12_octobre_capture_tv() {
        let bars: &[(f64, f64)] = &[
            (4192.75, 4183.50), // 06:00
            (4185.63, 4181.25),
            (4186.43, 4182.85),
            (4186.79, 4183.72),
            (4187.89, 4184.81),
            (4186.03, 4182.54),
            (4188.15, 4181.87), // 07:30 — pivot H (staircase intermédiaire)
            (4184.53, 4179.65),
            (4181.55, 4176.77),
            (4177.60, 4174.83),
            (4179.28, 4175.17),
            (4176.62, 4172.78),
            (4178.46, 4173.68),
            (4178.51, 4174.57),
            (4177.26, 4170.19),
            (4173.16, 4168.79), // 09:45 — pivot L
            (4175.02, 4172.04),
            (4179.40, 4173.93),
            (4180.22, 4177.77), // 10:30 — pivot H DÉFINITIF (capture TV)
            (4179.04, 4176.23),
            (4177.44, 4169.24),
            (4172.99, 4169.84),
            (4173.07, 4168.89),
            (4171.52, 4167.36),
            (4173.15, 4167.55), // 12:00 — fantôme à N=3, mort à N=4
            (4172.77, 4161.90),
            (4165.53, 4154.58), // 12:30 — pivot L (confirme à 13:30)
            (4168.03, 4162.39),
            (4172.69, 4162.92),
            (4175.37, 4170.83),
            (4175.39, 4169.82), // 13:30 — barre de confirmation du pivot L
        ];
        let mut det = SwingOteDetector::new(900);
        let ev = pousser(&mut det, bars);
        let z = ev.zone.expect("zone vivante en fin de replay");
        let base = barre(0, 0.0, 0.0).timestamp;
        assert_eq!(z.haut.prix, 4180.22, "ancre haute = capture TV");
        assert_eq!(z.haut.ts, base + 18 * 900, "10:30 UTC");
        assert_eq!(z.bas.prix, 4154.58, "ancre basse = capture TV");
        assert_eq!(z.bas.ts, base + 26 * 900, "12:30 UTC");
        assert!(z.bearish, "bas plus récent ⇒ jambe baissière");
        let a = 4180.22 - 4154.58;
        assert!((z.top - (4154.58 + 0.786 * a)).abs() < 1e-9, "top = 4174,73 (capture)");
        assert!((z.bot - (4154.58 + 0.618 * a)).abs() < 1e-9, "bot = 4170,42 (capture)");
        assert!((z.mid - (4154.58 + 0.5 * a)).abs() < 1e-9, "mid = 4167,40 (capture)");
    }

    /// Plateau parfait ⇒ aucune fractale (droite strictement inférieure
    /// requise) ⇒ aucune zone.
    #[test]
    fn plateau_aucune_zone() {
        let mut det = SwingOteDetector::new(900);
        let ev = pousser(&mut det, &[(4100.0, 4090.0); 12]);
        assert!(ev.zone.is_none(), "aucun pivot ⇒ aucune OTE");
    }

    /// Confluence dorée : overlap partiel / inclusion / disjonction / OTE
    /// absente — inchangée depuis la rév. 08/10.
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
