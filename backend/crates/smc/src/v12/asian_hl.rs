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

use chrono::{Datelike, TimeZone, Timelike};
use chrono_tz::Europe::Paris;

/// Événement Asian H/L pour une bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct AsianHlEvent {
    /// `_ahHighDrawn` — Asian High actif (close ne l'a pas franchi).
    pub high: Option<f64>,
    /// `_ahLowDrawn` — Asian Low actif.
    pub low: Option<f64>,
}

/// Fin de session Asie en minutes Paris (Pine `SES_PARIS_ASIE_END` = 390).
const SESSION_FIN_MIN: u32 = 390;

/// Détecteur Asian High/Low (MODULE 14, partie trading).
#[derive(Clone)]
pub struct AsianHlDetector {
    en_session: bool,
    high: f64,
    low: f64,
    drawn_high: Option<f64>,
    drawn_low: Option<f64>,
    last_event: AsianHlEvent,
}

impl AsianHlDetector {
    pub fn new() -> Self {
        Self {
            en_session: false,
            high: 0.0,
            low: 0.0,
            drawn_high: None,
            drawn_low: None,
            last_event: AsianHlEvent::default(),
        }
    }

    /// Traite une bar. Retourne les niveaux drawn après cette bar.
    pub fn update(&mut self, bar: &super::types::BarInput) -> AsianHlEvent {
        // Minutes depuis minuit Paris (DST automatique).
        let dt = chrono::DateTime::from_timestamp(bar.timestamp, 0);
        let en_session = match dt {
            Some(d) => {
                let paris = d.with_timezone(&Paris);
                let mins = paris.hour() * 60 + paris.minute();
                (0..SESSION_FIN_MIN).contains(&mins)
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

        // Invalidation par close (Pine : close > _ahHighDrawn / close < _ahLowDrawn).
        if self.drawn_high.is_some_and(|h| bar.close > h) {
            self.drawn_high = None;
        }
        if self.drawn_low.is_some_and(|l| bar.close < l) {
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
        let ev2 = det.update(&bar(fin, 100.0, 99.0, 100.0));
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
        let ev = det.update(&bar(paris_ts(2026, 6, 11, 7, 0), 101.0, 100.0, 100.5));
        assert_eq!(ev.high, Some(105.0), "Asian High de J2 remplace J1");
        assert_eq!(ev.low, Some(100.0), "Asian Low de J2");
    }
    
}
