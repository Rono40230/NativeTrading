//! MODULE 14 — Asian High/Low (Pine lignes 2860-2935, partie TRADING).
//!
//! Range high/low de la session Asie en heure de PARIS (00:00-06:30,
//! `SES_PARIS_ASIE_END = 390` min — DST gérée par Europe/Paris). À la fin de
//! la session, les niveaux deviennent « drawn » (actifs pour le trading :
//! DoL du znQual + cible TP3). Invalidation par CLOSE franchissant le niveau
//! (Pine `_ahHighDrawn := na` si `close > _ahHighDrawn`). Les niveaux drawn
//! persistent d'un jour à l'autre jusqu'à invalidation ou remplacement par
//! la session suivante.
//!
//! NOTE : le Pine ne met à jour `_ahHighDrawn` que si `i_showAsianHL` (input
//! d'affichage, défaut false) — un niveau de trading gated par un flag
//! d'affichage. Le port le calcule TOUJOURS (décision fidélité-trading).

use chrono::Timelike;
use chrono_tz::Europe::Paris;

/// Événement Asian H/L pour une bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct AsianHlEvent {
    /// `_ahHighDrawn` — Asian High actif (close ne l'a pas franchi).
    pub high: Option<f64>,
    /// `_ahLowDrawn` — Asian Low actif.
    pub low: Option<f64>,
}

/// Niveaux drawn des sessions pour le scoring Module F — état **N-1** (le Pine
/// lit `_ah/_ld*Drawn` dans `f_score` ~ligne 2430, AVANT leur mise à jour
/// MODULE 14/14b ~3090 : sémantique « liquidité de la session précédente »).
#[derive(Debug, Clone, Copy, Default)]
pub struct SessHlLevels {
    /// `_ahHighDrawn` à la bar N-1.
    pub ah_high: Option<f64>,
    /// `_ahLowDrawn` à la bar N-1.
    pub ah_low: Option<f64>,
    /// `_ldHighDrawn` (MODULE 14b Londres) à la bar N-1.
    pub ld_high: Option<f64>,
    /// `_ldLowDrawn` à la bar N-1.
    pub ld_low: Option<f64>,
}

/// Fin de session Asie en minutes Paris (Pine `SES_PARIS_ASIE_END` = 390).
const SESSION_FIN_MIN: u32 = 390;
/// `SES_PARIS_LONDON_START` = 480 (08:00 Paris, Pine ligne 172).
pub const LONDON_DEBUT_MIN: u32 = 480;
/// `SES_PARIS_LONDON_END` = 990 (16:30 Paris, Pine ligne 173).
pub const LONDON_FIN_MIN: u32 = 990;

/// Détecteur de H/L de session (MODULE 14 Asie ; MODULE 14b Londres via
/// [`AsianHlDetector::avec_fenetre`] — même mécanique, fenêtre paramétrée).
#[derive(Clone)]
pub struct AsianHlDetector {
    debut_min: u32,
    fin_min: u32,
    en_session: bool,
    high: f64,
    low: f64,
    drawn_high: Option<f64>,
    drawn_low: Option<f64>,
    last_event: AsianHlEvent,
}

impl AsianHlDetector {
    /// Session Asie (00:00-06:30 Paris) — comportement inchangé.
    pub fn new() -> Self {
        Self {
            debut_min: 0,
            fin_min: SESSION_FIN_MIN,
            en_session: false,
            high: 0.0,
            low: 0.0,
            drawn_high: None,
            drawn_low: None,
            last_event: AsianHlEvent::default(),
        }
    }

    /// Change la fenêtre de session (minutes Paris) — MODULE 14b Londres :
    /// `avec_fenetre(LONDON_DEBUT_MIN, LONDON_FIN_MIN)`.
    pub fn avec_fenetre(mut self, debut_min: u32, fin_min: u32) -> Self {
        self.debut_min = debut_min;
        self.fin_min = fin_min;
        self
    }

    /// Traite une bar. Retourne les niveaux drawn après cette bar.
    pub fn update(&mut self, bar: &super::types::BarInput) -> AsianHlEvent {
        // Minutes depuis minuit Paris (DST automatique).
        let dt = chrono::DateTime::from_timestamp(bar.timestamp, 0);
        let en_session = match dt {
            Some(d) => {
                let paris = d.with_timezone(&Paris);
                let mins = paris.hour() * 60 + paris.minute();
                (self.debut_min..self.fin_min).contains(&mins)
            }
            None => false,
        };

        if en_session {
            if !self.en_session {
                // _ahStart : début de session → reset du range.
                self.high = bar.high;
                self.low = bar.low;
            } else {
                // Extension du range.
                self.high = self.high.max(bar.high);
                self.low = self.low.min(bar.low);
            }
        } else if self.en_session {
            // _ahEnd : première bar après la session → niveaux drawn.
            self.drawn_high = Some(self.high);
            self.drawn_low = Some(self.low);
        }
        self.en_session = en_session;

        // « Décisions trading » 23/08 : niveau ATTEINT (sweep ou cassure) = consommé
        // (avant : close franchi uniquement — un sweep mèche+retour laissait le niveau).
        if self.drawn_high.is_some_and(|h| bar.high >= h) {
            self.drawn_high = None;
        }
        if self.drawn_low.is_some_and(|l| bar.low <= l) {
            self.drawn_low = None;
        }

        let ev = AsianHlEvent {
            high: self.drawn_high,
            low: self.drawn_low,
        };
        self.last_event = ev;
        ev
    }

    pub fn last_event(&self) -> AsianHlEvent {
        self.last_event
    }
}

impl Default for AsianHlDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bar(ts: i64, high: f64, low: f64, close: f64) -> super::super::types::BarInput {
        super::super::types::BarInput {
            timestamp: ts,
            open: close,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    /// Timestamp UTC d'une date/heure Paris donnée.
    fn paris_ts(annee: i32, mois: u32, jour: u32, heure: u32, minute: u32) -> i64 {
        use chrono::TimeZone;
        Paris
            .with_ymd_and_hms(annee, mois, jour, heure, minute, 0)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn range_pendant_session_puis_drawn_a_la_fin() {
        let mut det = AsianHlDetector::new();
        // Session : 00:00 → 06:29 Paris (le 10/06/2026 = heure d'été, UTC+2).
        let j = paris_ts(2026, 6, 10, 0, 0);
        let ev1 = det.update(&bar(j, 101.0, 99.0, 100.0)); // _ahStart
        assert!(ev1.high.is_none(), "rien de drawn pendant la session");
        det.update(&bar(j + 900, 103.0, 98.0, 100.0)); // extension
        let fin = paris_ts(2026, 6, 10, 6, 45); // après 06:30 → _ahEnd
        let ev3 = det.update(&bar(fin, 100.0, 99.0, 100.0));
        assert_eq!(
            ev3.high,
            Some(103.0),
            "Asian High drawn à la fin de session"
        );
        assert_eq!(ev3.low, Some(98.0), "Asian Low drawn");
    }

    #[test]
    fn invalidation_par_close() {
        let mut det = AsianHlDetector::new();
        let j = paris_ts(2026, 6, 10, 3, 0);
        det.update(&bar(j, 103.0, 98.0, 100.0));
        let fin = paris_ts(2026, 6, 10, 7, 0);
        let ev = det.update(&bar(fin, 104.0, 99.0, 103.5)); // close 103.5 > high 103
        assert_eq!(ev.high, None, "close > Asian High ⇒ invalidé");
        assert_eq!(ev.low, Some(98.0), "Asian Low intact");
        let apres = det.update(&bar(fin + 900, 96.0, 95.0, 95.5)); // close 95.5 < low 98
        assert_eq!(apres.low, None, "close < Asian Low ⇒ invalidé");
    }

    #[test]
    fn dst_hiver_la_session_est_bien_parisienne() {
        // Le 10/01/2026 (heure d'hiver, UTC+1) : 06:00 Paris = 05:00 UTC.
        // Si le détecteur utilisait UTC, 06:00 UTC serait déjà hors session
        // (06:00 Paris = fin 06:30 Paris ; en UTC pur, 06:45 serait hors [0,390)...).
        let mut det = AsianHlDetector::new();
        let hiver = paris_ts(2026, 1, 10, 6, 0); // 05:00 UTC — DANS la session Paris
        let ev = det.update(&bar(hiver, 100.0, 99.0, 100.0));
        assert!(ev.high.is_none(), "06:00 Paris hiver = encore en session");
        let fin = paris_ts(2026, 1, 10, 6, 45);
        // Bar de fin SANS toucher le high (99.5 < 100) — règle atteinte = consommé.
        let ev2 = det.update(&bar(fin, 99.5, 99.0, 99.2));
        assert_eq!(ev2.high, Some(100.0), "drawn après la fin de session hiver");
    }

    #[test]
    fn remplacement_par_la_session_suivante() {
        let mut det = AsianHlDetector::new();
        // J1 : session + fin → drawn [103, 98].
        det.update(&bar(paris_ts(2026, 6, 10, 3, 0), 103.0, 98.0, 100.0));
        det.update(&bar(paris_ts(2026, 6, 10, 7, 0), 100.0, 99.0, 100.0));
        // J2 : nouvelle session avec un range différent → drawn remplacé à la fin.
        det.update(&bar(paris_ts(2026, 6, 11, 2, 0), 105.0, 100.0, 102.0));
        // low 100.5 > Asian Low 100 : pas de consommation à l'atteinte.
        let ev = det.update(&bar(paris_ts(2026, 6, 11, 7, 0), 104.0, 100.5, 101.0));
        assert_eq!(ev.high, Some(105.0), "Asian High de J2 remplace J1");
        assert_eq!(ev.low, Some(100.0), "Asian Low de J2");
    }

    // ── MODULE 14b — Londres (Module F, Phase 4) ──────────────────────────────

    /// Londres : session 08:00-16:30 Paris (480-990 min). Bar à 07:00 Paris =
    /// hors session ; 08:00 = début ; 16:45 = fin → drawn.
    #[test]
    fn london_range_puis_drawn_a_la_fin() {
        let mut det = AsianHlDetector::new().avec_fenetre(LONDON_DEBUT_MIN, LONDON_FIN_MIN);
        // 07:00 Paris : AVANT la session Londres (l'Asie serait déjà finie).
        let avant = det.update(&bar(paris_ts(2026, 6, 10, 7, 0), 101.0, 99.0, 100.0));
        assert!(avant.high.is_none(), "07:00 Paris = hors session Londres");
        // 08:00 → début (reset du range).
        det.update(&bar(paris_ts(2026, 6, 10, 8, 0), 104.0, 99.0, 100.0));
        det.update(&bar(paris_ts(2026, 6, 10, 12, 0), 106.0, 98.0, 100.0)); // extension
        // 16:45 Paris (après 16:30) → drawn [106, 98].
        let fin = det.update(&bar(paris_ts(2026, 6, 10, 16, 45), 100.0, 99.0, 100.0));
        assert_eq!(fin.high, Some(106.0), "London High drawn à la fin");
        assert_eq!(fin.low, Some(98.0), "London Low drawn");
    }

    /// Londres : consommation à l'atteinte (high 106 touché → None).
    #[test]
    fn london_consomme_a_latteinte() {
        let mut det = AsianHlDetector::new().avec_fenetre(LONDON_DEBUT_MIN, LONDON_FIN_MIN);
        det.update(&bar(paris_ts(2026, 6, 10, 9, 0), 106.0, 98.0, 100.0));
        let fin = det.update(&bar(paris_ts(2026, 6, 10, 16, 45), 100.0, 99.0, 100.0));
        assert_eq!(fin.high, Some(106.0));
        // Bar suivante : high 106.5 >= 106 ⇒ consommé.
        let apres = det.update(&bar(paris_ts(2026, 6, 10, 17, 0), 106.5, 99.0, 105.0));
        assert_eq!(apres.high, None, "London High consommé à l'atteinte");
        assert_eq!(apres.low, Some(98.0), "London Low intact");
    }

    /// Les deux détecteurs coexistent : Asie drawn à 06:45, Londres à 16:45.
    /// Ranges imbriqués SANS atteinte (les bars Londres ne touchent ni le
    /// high ni le low Asie — sinon consommation légitime).
    #[test]
    fn asie_et_londres_independants() {
        let mut asie = AsianHlDetector::new();
        let mut londres =
            AsianHlDetector::new().avec_fenetre(LONDON_DEBUT_MIN, LONDON_FIN_MIN);
        // Asie [99..102] · Londres [100.5..101.5] — bar de fin neutre.
        let bars = [
            (paris_ts(2026, 6, 10, 1, 0), 102.0, 99.0, 100.0),   // Asie
            (paris_ts(2026, 6, 10, 9, 0), 100.8, 100.2, 100.5),  // Londres
            (paris_ts(2026, 6, 10, 10, 0), 101.5, 100.5, 101.0), // Londres
            (paris_ts(2026, 6, 10, 16, 45), 101.3, 100.7, 101.0),// fin Londres
        ];
        let mut ev_a = AsianHlEvent::default();
        let mut ev_l = AsianHlEvent::default();
        for (ts, h, l, c) in bars {
            ev_a = asie.update(&bar(ts, h, l, c));
            ev_l = londres.update(&bar(ts, h, l, c));
        }
        assert_eq!(ev_a.high, Some(102.0), "Asian High [00:00-06:30]");
        assert_eq!(ev_a.low, Some(99.0), "Asian Low");
        assert_eq!(ev_l.high, Some(101.5), "London High [08:00-16:30]");
        assert_eq!(ev_l.low, Some(100.2), "London Low — fixé par la 1re bougie (100.2 < 100.5)");
    }
}
