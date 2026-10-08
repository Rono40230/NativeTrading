//! OTE swing institutionnelle — bande 61,8–78,6 % de la jambe structurelle
//! **chaîne HH/LL de la wave ↔ extrême courant** (spec § 2.2 rév. 08/10).
//!
//! Rétro-ingénierie « SMC Institutional Scalper v1.3 » (KASPER Trading),
//! validée sur données réelles XAU (M15/M30/H2 du 06-08/10, flux pivot complet
//! depuis le 15/09) contre 4 captures :
//!
//! 1. **Wave** = zigzag à seuil de retracement, déclencheur MÈCHE. Balayage
//!    empirique : seuil 0,85–1,00 % reproduit simultanément sur les trois TF
//!    la jambe cible 4184,52 (06/10 19:00 UTC) → 4066,46 (07/10 12:45 UTC) ;
//!    le déclencheur clôture échoue sur tous les seuils. Défaut 0,9 %
//!    (`WAVE_SEUIL_PCT`) — la valeur « 0.45 » du panneau ne reproduit PAS les
//!    ancres observées ; paramètre ajustable sans migration.
//! 2. **Ancres** : chaîne du dernier HH (pivot H confirmé > pivot H précédent)
//!    et du dernier LL (miroir). Les pivots internes LH/HL ne ré-ancrent pas
//!    (preuve : rebond XAU 4143,25, retracement 65 % DANS la bande, OTE cible
//!    inchangée) ; la FUSION (pivot précoce dépassé) ne chaîne que si elle bat
//!    l'ancre de chaîne (sinon un LH mineur dégraderait la référence).
//! 3. **Extrême courant = RUNNING** : la cible affichait déjà fib(4184,52 ;
//!    4066,46) le 07/10 soir alors que le creux n'était pivot confirmé que le
//!    08/10 02:30 — l'extrême vivant s'utilise avant confirmation.
//! 4. Les rebonds internes ne suppriment PAS l'affichage (« Invalidate OTE
//!    after first touch » = règle de TRADING FIRE) ; les « swings » Left/
//!    Right bars = 2 (captures 11/12) sont une couche d'affichage séparée.
//!
//! Cas de référence vérifié (XAU M15, DB réelle) : jambe 4184,52 → 4066,46 ⇒
//! OTE [4139,42 ; 4159,26], trait 50 % à 4125,49.
//!
//! Isolation totale : aucun module moteur lu, aucun consommateur
//! scoring/signals (Phase B = affichage uniquement).

use super::types::{BarInput, SwingAnchor, SwingOteEvent, SwingOteZone};

/// Seuil de retracement de la wave par timeframe, en % — fit empirique 08/10
/// contre les captures TV « Zone OTE par TF » (XAU) :
///   - M15/M30/H1/H4 : 0,85–1,00 % reproduisent la jambe 4184,52 → 4066,46
///     (validée exactement par le propriétaire) — défaut 0,9 ;
///   - M1/M5 : 0,30–0,40 % reproduisent la jambe intraday 4143,25 → 4109,91
///     (08/10 matin) — défaut 0,35 ;
///   - D1/W1 : PROVISOIRE — la table D1 de la DB est corrompue (~90 % de
///     bougies divergentes de l'agrégat M30, audit 08/10) : à recalibrer
///     après réparation des données.
/// NB : un seuil unique (ATR ou %) ne peut PAS reproduire les 7 TF à la fois
/// (balayages 08/10) — la Jambe/ATR varie de 1,7 (D1) à 18 (M1).
pub fn seuil_pct_par_tf(tf_sec: i64) -> f64 {
    match tf_sec {
        60 | 300 => 0.35,      // M1, M5
        900 | 1800 | 3600 | 14400 => 0.9, // M15, M30, H1, H4
        86_400 => 1.5,         // D1 (provisoire)
        604_800 => 2.0,        // W1 (provisoire)
        _ => 0.9,
    }
}

const FIB_MID: f64 = 0.5;
/// Borne « profonde » 61,8 % (proche de l'équilibre de la jambe).
const FIB_618: f64 = 0.618;
/// Borne « extrême » 78,6 % (proche de l'origine du mouvement).
const FIB_786: f64 = 0.786;

/// Fenêtre des barres récentes pour réinitialiser les extrêmes running à la
/// date d'un ancrage (le retard de confirmation wave est de quelques barres).
const TAILLE_BUF: usize = 64;

/// Un pivot de la wave (confirmé, ou déplacé par fusion).
#[derive(Clone, Copy)]
struct Pivot {
    prix: f64,
    ts: i64,
}

/// Détecteur OTE : machine wave (zigzag à seuil + fusion) → chaînes HH/LL →
/// jambe = ancre de chaîne (origine) ↔ extrême running (côté courant).
#[derive(Clone)]
pub struct SwingOteDetector {
    seuil_bas: f64,
    seuil_haut: f64,
    /// Dernier pivot de la wave + direction de la jambe (`true` = le candidat
    /// est un plus haut).
    pivot_haut: Option<bool>,
    pivot: Pivot,
    jambe_haussiere: bool,
    cand: Pivot,
    /// Chaînes : dernier HH / LL et prix du dernier pivot confirmé de chaque
    /// sens (référence de chaînage).
    h_prec: Option<f64>,
    l_prec: Option<f64>,
    hh: Option<SwingAnchor>,
    ll: Option<SwingAnchor>,
    /// Extrêmes running depuis les ancrages de chaîne.
    min: Option<Pivot>,
    max: Option<Pivot>,
    /// Barres récentes (ts, high, low) pour les réinitialisations.
    buf: Vec<(i64, f64, f64)>,
    zone: Option<SwingOteZone>,
    last_event: SwingOteEvent,
}

impl Default for SwingOteDetector {
    fn default() -> Self {
        Self::new(0.9)
    }
}

impl SwingOteDetector {
    /// `seuil_pct` : retrangement (en %) qui confirme un pivot de la wave —
    /// utiliser [`seuil_pct_par_tf`] pour la valeur calibrée du timeframe.
    pub fn new(seuil_pct: f64) -> Self {
        Self {
            seuil_bas: 1.0 - seuil_pct / 100.0,
            seuil_haut: 1.0 + seuil_pct / 100.0,
            pivot_haut: None,
            pivot: Pivot { prix: 0.0, ts: 0 },
            jambe_haussiere: true,
            cand: Pivot { prix: 0.0, ts: 0 },
            h_prec: None,
            l_prec: None,
            hh: None,
            ll: None,
            min: None,
            max: None,
            buf: Vec::with_capacity(TAILLE_BUF),
            zone: None,
            last_event: SwingOteEvent::default(),
        }
    }

    pub fn update(&mut self, bar: &BarInput) -> SwingOteEvent {
        self.last_event = SwingOteEvent::default();

        match self.pivot_haut {
            None => {
                // Init : pivot L = premier bas, jambe haussière, candidat = 1er haut.
                self.pivot_haut = Some(false);
                self.pivot = Pivot { prix: bar.low, ts: bar.timestamp };
                self.l_prec = Some(bar.low);
                self.jambe_haussiere = true;
                self.cand = Pivot { prix: bar.high, ts: bar.timestamp };
            }
            Some(_) => self.evoluer(bar),
        }

        self.buf.push((bar.timestamp, bar.high, bar.low));
        if self.buf.len() > TAILLE_BUF {
            self.buf.remove(0);
        }
        self.maj_extremes_running();
        self.calculer_zone();
        self.last_event.zone = self.zone;
        self.last_event.clone()
    }

    /// Transition wave d'une bar (hors init). La fusion déplace le pivot sans
    /// chaîner (sauf si elle bat l'ancre de chaîne) ; la confirmation chaîne
    /// contre le pivot précédent du même sens.
    fn evoluer(&mut self, bar: &BarInput) {
        if self.jambe_haussiere {
            if bar.low < self.pivot.prix {
                // Fusion : le creux initial était précoce, il descend.
                self.pivot = Pivot { prix: bar.low, ts: bar.timestamp };
                self.l_prec = Some(bar.low);
                if self.ll.map_or(true, |l| bar.low < l.prix) {
                    self.ll = Some(SwingAnchor { prix: bar.low, ts: bar.timestamp });
                    self.reset_min();
                }
            }
            if bar.high > self.cand.prix {
                self.cand = Pivot { prix: bar.high, ts: bar.timestamp };
            }
            if bar.low <= self.cand.prix * self.seuil_bas {
                let p = self.cand;
                self.pivot_haut = Some(true);
                self.pivot = p;
                self.jambe_haussiere = false;
                self.cand = Pivot { prix: bar.low, ts: bar.timestamp };
                // Chaîne HH : contre le pivot H confirmé PRÉCÉDENT (pas contre
                // le HH courant — sinon un ancien maximum immobiliserait la
                // référence pour toujours).
                let chainable = self.h_prec.map_or(true, |h| p.prix > h);
                self.h_prec = Some(p.prix);
                if chainable {
                    self.hh = Some(SwingAnchor { prix: p.prix, ts: p.ts });
                    self.reset_min();
                }
            }
        } else {
            if bar.high > self.pivot.prix {
                // Fusion miroir : le sommet initial était précoce, il monte.
                self.pivot = Pivot { prix: bar.high, ts: bar.timestamp };
                self.h_prec = Some(bar.high);
                if self.hh.map_or(true, |h| bar.high > h.prix) {
                    self.hh = Some(SwingAnchor { prix: bar.high, ts: bar.timestamp });
                    self.reset_min();
                }
            }
            if bar.low < self.cand.prix {
                self.cand = Pivot { prix: bar.low, ts: bar.timestamp };
            }
            if bar.high >= self.cand.prix * self.seuil_haut {
                let p = self.cand;
                self.pivot_haut = Some(false);
                self.pivot = p;
                self.jambe_haussiere = true;
                self.cand = Pivot { prix: bar.high, ts: bar.timestamp };
                // Chaîne LL : contre le pivot L confirmé PRÉCÉDENT.
                let chainable = self.l_prec.map_or(true, |l| p.prix < l);
                self.l_prec = Some(p.prix);
                if chainable {
                    self.ll = Some(SwingAnchor { prix: p.prix, ts: p.ts });
                    self.reset_max();
                }
            }
        }
    }

    /// Extrêmes running : plus bas depuis le HH (ancre jambe baissière),
    /// plus haut depuis le LL (jambe haussière) — mis à jour chaque bar.
    fn maj_extremes_running(&mut self) {
        if let Some(hh) = self.hh {
            let mut min = self.min.unwrap_or(Pivot { prix: f64::MAX, ts: hh.ts });
            for &(ts, _, low) in self.buf.iter() {
                if ts >= hh.ts && low < min.prix {
                    min = Pivot { prix: low, ts };
                }
            }
            self.min = Some(min);
        }
        if let Some(ll) = self.ll {
            let mut max = self.max.unwrap_or(Pivot { prix: f64::MIN, ts: ll.ts });
            for &(ts, high, _) in self.buf.iter() {
                if ts >= ll.ts && high > max.prix {
                    max = Pivot { prix: high, ts };
                }
            }
            self.max = Some(max);
        }
    }

    /// Réinitialisation après re-ancrage : le buffer seul ne couvre pas
    /// forcément la fenêtre — on repart de l'ancre (les barres intermédiaires
    /// hors buffer sont perdues, cas rare : ancre ancienne re-chaînée).
    fn reset_min(&mut self) {
        self.min = None;
    }
    fn reset_max(&mut self) {
        self.max = None;
    }

    /// OTE = fib 61,8–78,6 % entre l'ORIGINE (ancre de chaîne la plus
    /// ANCIENNE) et l'extrême running du côté courant.
    fn calculer_zone(&mut self) {
        self.zone = None;
        let (hh, ll) = match (self.hh, self.ll) {
            (Some(h), Some(l)) => (h, l),
            _ => return,
        };
        // LL plus récent ⇒ jambe baissière (origine = HH, extrême = running
        // min) ; HH plus récent ⇒ jambe haussière (origine = LL, running max).
        let (bearish, haut, bas) = if ll.ts > hh.ts {
            let min = match self.min {
                Some(m) => m,
                None => return,
            };
            (true, hh, SwingAnchor { prix: min.prix, ts: min.ts })
        } else {
            let max = match self.max {
                Some(m) => m,
                None => return,
            };
            (false, SwingAnchor { prix: max.prix, ts: max.ts }, ll)
        };
        let (haut, bas) = if haut.prix >= bas.prix { (haut, bas) } else { (bas, haut) };
        let a = haut.prix - bas.prix;
        if a <= 0.0 {
            return;
        }
        let (top, bot, mid) = if bearish {
            (
                bas.prix + FIB_786 * a,
                bas.prix + FIB_618 * a,
                bas.prix + FIB_MID * a,
            )
        } else {
            (
                haut.prix - FIB_618 * a,
                haut.prix - FIB_786 * a,
                haut.prix - FIB_MID * a,
            )
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

/// Confluence dorée (spec § 2.3) : un OB est doré ⟺ son intersection de prix
/// avec l'OTE vivante est non vide (chevauchement ou inclusion, sans condition
/// de sens). Sans OTE vivante → jamais doré.
pub fn chevauche(ob_top: f64, ob_bot: f64, ote: Option<SwingOteZone>) -> bool {
    match ote {
        Some(z) => ob_bot <= z.top && ob_top >= z.bot,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Barre où seuls high/low nous intéressent.
    fn barre(i: usize, high: f64, low: f64) -> BarInput {
        BarInput {
            timestamp: i as i64,
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

    /// Cas de référence XAU M15 (06-08/10, géométrie réelle) : HH 4185 (bar 5)
    /// puis LL 4067 (bar 8), rebond 4145 (retracement ~66 %, DANS la bande) et
    /// creux secondaire 4110 — la jambe SURVIT :
    ///   OTE [4139,924 ; 4159,748], mid 4126, bord gauche = bar du HH.
    #[test]
    fn cas_reference_m15_jambe_survit_aux_rebonds() {
        let mut det = SwingOteDetector::new(0.9);
        let ev = pousser(
            &mut det,
            &[
                (4100.0, 4090.0),
                (4110.0, 4095.0),
                (4165.0, 4120.0), // H 4165 (pivot, non HH — pas de précédent)
                (4140.0, 4125.0), // retracement ≥ seuil ⇒ H 4165 confirmé
                (4140.0, 4130.0),
                (4185.0, 4140.0), // HH 4185 (> 4165)
                (4165.0, 4110.0), // retracement ⇒ HH confirmé, jambe baissière
                (4138.0, 4072.0),
                (4132.0, 4067.0), // LL 4067 (< L précédent 4110)
                (4145.0, 4120.0), // rebond ⇒ LL confirmé ; 4145 ne chaîne pas H
                (4130.0, 4110.0), // creux secondaire : PAS de nouveau pivot
            ],
        );
        let z = ev.zone.expect("jambe HH→LL vivante après rebonds internes");
        assert!(z.bearish, "LL plus récent que le HH ⇒ jambe baissière");
        assert_eq!(z.haut.prix, 4185.0);
        assert_eq!(z.haut.ts, 5);
        assert_eq!(z.bas.prix, 4067.0);
        assert_eq!(z.bas.ts, 8);
        let a = 4185.0 - 4067.0;
        assert_eq!(z.mid, 4126.0, "trait 50 % = 4126,0 (vérifié capture)");
        assert!((z.bot - (4067.0 + 0.618 * a)).abs() < 1e-9);
        assert!((z.top - (4067.0 + 0.786 * a)).abs() < 1e-9);
        assert_eq!(z.ts_naissance, 5, "bord gauche = ancre la plus ancienne (HH)");
    }

    /// Jambe haussière miroire : LL 100 (bar 1) puis HH 200 (bar 8), repli 150
    /// sans confirmation ⇒ OTE bull [121,4 ; 138,2], mid 150.
    #[test]
    fn jambe_haussiere_miroir() {
        let mut det = SwingOteDetector::new(0.9);
        let ev = pousser(
            &mut det,
            &[
                (115.0, 103.0),
                (110.0, 100.0), // LL 100 (< L initial 103, par fusion)
                (112.0, 101.0),
                (120.0, 104.0),
                (130.0, 108.0),
                (145.0, 115.0),
                (160.0, 122.0),
                (180.0, 130.0),
                (200.0, 135.0), // HH 200 (via fusion > 120 puis chaîne)
                (160.0, 150.0), // repli ⇒ HH confirmé ; jambe baissière
                (170.0, 155.0), // L 150 : PAS de LL (150 > 100)
            ],
        );
        let z = ev.zone.expect("jambe LL→HH vivante");
        assert!(!z.bearish, "HH plus récent ⇒ jambe haussière");
        assert_eq!(z.bas.prix, 100.0);
        assert_eq!(z.haut.prix, 200.0);
        assert!((z.top - 138.2).abs() < 1e-9);
        assert!((z.bot - 121.4).abs() < 1e-9);
        assert_eq!(z.mid, 150.0);
        assert_eq!(z.ts_naissance, 1, "bord gauche = ancre la plus ancienne (LL)");
    }

    /// Extrême running + bascule : un nouveau creux SOUS le LL élargit la bande
    /// IMMÉDIATEMENT (running, sans attendre confirmation) ; un nouveau HH
    /// (> l'ancre de chaîne, par fusion) retourne la jambe en haussière.
    #[test]
    fn running_prolonge_puis_nouveau_hh_bascule() {
        let mut det = SwingOteDetector::new(0.9);
        pousser(
            &mut det,
            &[
                (4100.0, 4090.0),
                (4110.0, 4095.0),
                (4165.0, 4120.0),
                (4140.0, 4125.0),
                (4140.0, 4130.0),
                (4185.0, 4140.0),
                (4165.0, 4110.0),
                (4138.0, 4072.0),
                (4132.0, 4067.0),
                (4145.0, 4120.0),
                (4130.0, 4110.0),
            ],
        );
        // Running : creux 4050 sous le LL ⇒ bande élargie immédiatement.
        let ev = det.update(&barre(11, 4110.0, 4050.0));
        let z = ev.zone.unwrap();
        assert!(z.bearish);
        assert_eq!(z.bas.prix, 4050.0, "extrême running suit le nouveau creux");
        assert_eq!(z.bas.ts, 11);
        let a1 = 4185.0 - 4050.0;
        assert!((z.bot - (4050.0 + 0.618 * a1)).abs() < 1e-9, "bande élargie");

        // Nouveau HH 4200 (> 4185, par fusion qui bat l'ancre) ⇒ jambe
        // haussière depuis le LL 4050.
        let ev = det.update(&barre(12, 4200.0, 4160.0));
        let z = ev.zone.unwrap();
        assert!(!z.bearish, "HH plus récent ⇒ jambe retournée haussière");
        assert_eq!(z.bas.prix, 4050.0, "nouvelle origine = LL courant");
        assert_eq!(z.haut.prix, 4200.0);
        let a2 = 4200.0 - 4050.0;
        assert!((z.top - (4200.0 - 0.618 * a2)).abs() < 1e-9);
        assert_eq!(z.ts_naissance, 11, "bord gauche = LL (ancre ancienne)");
    }

    /// Confluence dorée : overlap partiel / inclusion (dans un sens ou l'autre)
    /// / disjonction / OTE absente.
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
        assert!(chevauche(4145.0, 4138.0, Some(zone)), "OB inclus (cas capture dorée)");
        assert!(chevauche(4165.0, 4158.0, Some(zone)), "chevauchement partiel");
        assert!(chevauche(4170.0, 4130.0, Some(zone)), "OTE incluse dans l'OB");
        assert!(!chevauche(4170.0, 4165.0, Some(zone)), "disjonction au-dessus");
        assert!(!chevauche(4135.0, 4120.0, Some(zone)), "disjonction en dessous");
        assert!(chevauche(zone.bot, zone.bot - 5.0, Some(zone)), "contact bord à bord");
        assert!(!chevauche(4145.0, 4138.0, None), "sans OTE ⇒ jamais doré");
    }

    /// Une seule barre ⇒ aucune chaîne HH/LL ⇒ aucune zone.
    #[test]
    fn init_seule_aucune_zone() {
        let mut det = SwingOteDetector::new(0.9);
        let ev = det.update(&barre(0, 4100.0, 4090.0));
        assert!(ev.zone.is_none(), "premier pivot jamais HH/LL (pas de précédent)");
    }

    /// Table de calibration par TF (fit 08/10, captures « Zone OTE par TF ») :
    /// M1/M5 intraday 0,35 % ; M15→H4 0,9 % ; D1/W1 provisoires.
    #[test]
    fn seuil_pct_par_tf_calibration() {
        assert_eq!(seuil_pct_par_tf(60), 0.35, "M1");
        assert_eq!(seuil_pct_par_tf(300), 0.35, "M5");
        assert_eq!(seuil_pct_par_tf(900), 0.9, "M15");
        assert_eq!(seuil_pct_par_tf(1800), 0.9, "M30");
        assert_eq!(seuil_pct_par_tf(3600), 0.9, "H1");
        assert_eq!(seuil_pct_par_tf(14400), 0.9, "H4");
        assert_eq!(seuil_pct_par_tf(86_400), 1.5, "D1 provisoire");
        assert_eq!(seuil_pct_par_tf(604_800), 2.0, "W1 provisoire");
        assert_eq!(seuil_pct_par_tf(12345), 0.9, "TF inconnu → défaut");
    }
}
