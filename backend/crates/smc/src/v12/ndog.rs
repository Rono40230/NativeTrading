//! MODULE 10b — NDOG / NWOG (New Day / New Week Opening Gaps).
//!
//! Reproduit MODULE 10b Pine (lignes 1533-1652) :
//!   - NDOG : gap entre clôture de la veille et ouverture du nouveau jour (M1–M15).
//!   - NWOG : gap entre clôture de vendredi et ouverture de lundi (H1–H4).
//!   - `_gapMin = 0.3 * atr14` (`i_gapMinMult = 0.3`).
//!   - Création : `_gTop = max(open, close[1])`, `_gBot = min(open, close[1])`,
//!     si `_gTop - _gBot >= _gapMin and _gTop != _gBot`.
//!   - FIFO 1 par type (`i_maxGap = 1`).
//!   - Lifecycle : mitigé quand `low <= _gBot and high >= _gTop` (prix remplit le gap).
//!
//! Détection de nouveau jour/semaine (Pine lignes 1563-1565) :
//!   `_newDay  = not na(close[1]) and dayofmonth != dayofmonth[1]`
//!   `_newWeek = not na(close[1]) and weekofyear != weekofyear[1]`
//! On utilise le jour calendaire UTC et la semaine ISO (cohérent avec le module
//! Liquidites du même moteur).
//!
//! Gating TF (Pine lignes 1549-1550) :
//!   `_tfNDOG = timeframe.in_seconds() <= 900`          // M1–M15
//!   `_tfNWOG = timeframe.in_seconds() >= 3600 and <= 14400`  // H1–H4

use super::types::{BarInput, GapZone, NdogEvent};

/// `i_gapMinMult` (Pine ligne 1542) = 0.3.
pub const GAP_MIN_MULT: f64 = 0.3;
/// `i_maxGap` (Pine ligne 1543) = 1 (FIFO 1 par type).
pub const MAX_GAP: usize = 1;

/// Numéro de jour UTC (équivalent Pine `dayofmonth`).
fn day_key(ts: i64) -> i64 {
    ts.div_euclid(86_400)
}

/// Clé de semaine ISO (équivalent Pine `weekofyear`).
fn week_key(ts: i64) -> (i32, u32) {
    use chrono::Datelike;
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0);
    match dt {
        Some(dt) => {
            let iso = dt.naive_utc().iso_week();
            (iso.year(), iso.week())
        }
        None => (0, 0),
    }
}

/// Détecteur NDOG/NWOG.
#[derive(Clone)]
pub struct NdogDetector {
    ndog: Vec<GapZone>,
    nwog: Vec<GapZone>,
    /// Bougie précédente (Pine `close[1]`).
    prev_bar: Option<BarInput>,
    prev_day_key: Option<i64>,
    prev_week_key: Option<(i32, u32)>,
    tf_sec: i64,
    tf_ndog: bool,
    tf_nwog: bool,
    bar_count: usize,
    last_event: NdogEvent,
}

impl NdogDetector {
    pub fn new(tf_sec: i64) -> Self {
        // Gating TF (Pine lignes 1549-1550).
        let tf_ndog = (1..=900).contains(&tf_sec);
        let tf_nwog = (3600..=14400).contains(&tf_sec);
        Self {
            ndog: Vec::new(),
            nwog: Vec::new(),
            prev_bar: None,
            prev_day_key: None,
            prev_week_key: None,
            tf_sec,
            tf_ndog,
            tf_nwog,
            bar_count: 0,
            last_event: NdogEvent::default(),
        }
    }

    /// Traite une bar.
    ///
    /// - `atr14` : ATR14 courant (Pine `atr14`). Si Na (≤ 0), `_gapMin = 0.0`.
    pub fn update(&mut self, bar: &BarInput, atr14: f64) -> NdogEvent {
        let cur = self.bar_count;
        self.bar_count += 1;
        let mut ev = NdogEvent::default();

        // --- Détection nouveau jour / semaine (Pine lignes 1563-1565) ---
        let dk = day_key(bar.timestamp);
        let wk = week_key(bar.timestamp);
        let new_day = match self.prev_day_key {
            Some(p) => dk != p,
            None => false,
        };
        let new_week = match self.prev_week_key {
            Some(p) => wk != p,
            None => false,
        };

        let prev_close = self.prev_bar.map(|b| b.close);
        let gap_min = if atr14 > 0.0 { GAP_MIN_MULT * atr14 } else { 0.0 };

        // --- Création NDOG (f_block3, Pine lignes 1570-1589) ---
        if new_day && self.tf_ndog {
            if let Some(pc) = prev_close {
                let g_top = bar.open.max(pc);
                let g_bot = bar.open.min(pc);
                if g_top - g_bot >= gap_min && g_top != g_bot {
                    if self.ndog.len() >= MAX_GAP {
                        self.ndog.remove(0); // array.shift
                    }
                    let gz = GapZone {
                        top: g_top,
                        bot: g_bot,
                        mitigated: false,
                        bar: cur,
                        is_week: false,
                    };
                    ev.new_ndog = Some(gz);
                    self.ndog.push(gz);
                }
            }
        }

        // --- Création NWOG (f_block2, Pine lignes 1595-1614) ---
        if new_week && self.tf_nwog {
            if let Some(pc) = prev_close {
                let g_top = bar.open.max(pc);
                let g_bot = bar.open.min(pc);
                if g_top - g_bot >= gap_min && g_top != g_bot {
                    if self.nwog.len() >= MAX_GAP {
                        self.nwog.remove(0);
                    }
                    let gz = GapZone {
                        top: g_top,
                        bot: g_bot,
                        mitigated: false,
                        bar: cur,
                        is_week: true,
                    };
                    ev.new_nwog = Some(gz);
                    self.nwog.push(gz);
                }
            }
        }

        // --- Lifecycle NDOG/NWOG : mitigation (Pine lignes 1620-1652) ---
        // Mitigé quand `low <= bot and high >= top`.
        for g in &mut self.ndog {
            if !g.mitigated && bar.low <= g.bot && bar.high >= g.top {
                g.mitigated = true;
            }
        }
        for g in &mut self.nwog {
            if !g.mitigated && bar.low <= g.bot && bar.high >= g.top {
                g.mitigated = true;
            }
        }

        self.prev_bar = Some(*bar);
        self.prev_day_key = Some(dk);
        self.prev_week_key = Some(wk);
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> NdogEvent {
        self.last_event.clone()
    }
    /// Gaps NDOG actifs (mitigés ou non).
    pub fn ndog_zones(&self) -> &[GapZone] {
        &self.ndog
    }
    /// Gaps NWOG actifs.
    pub fn nwog_zones(&self) -> &[GapZone] {
        &self.nwog
    }
    pub fn tf_sec(&self) -> i64 {
        self.tf_sec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(ts: i64, open: f64, high: f64, low: f64, close: f64) -> BarInput {
        BarInput {
            timestamp: ts,
            open,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    #[test]
    fn ndog_cree_au_changement_de_jour_si_gap_suffisant() {
        // M15 ⇒ tf_ndog actif, tf_nwog inactif.
        let mut det = NdogDetector::new(900);
        // Jour 1 : close à 100.
        det.update(&bar(100, 100.0, 101.0, 99.0, 100.0), 1.0);
        // Jour 2 (ts = 86400), gap d'ouverture : open=110, close veille=100 ⇒ gTop=110.
        let ev = det.update(&bar(86_400, 110.0, 111.0, 109.0, 110.0), 1.0);
        assert!(ev.new_ndog.is_some(), "gap 10 >= gapMin(0.3) ⇒ NDOG créé");
        assert_eq!(det.ndog_zones().len(), 1);
        assert_eq!(det.ndog_zones()[0].top, 110.0);
        assert_eq!(det.ndog_zones()[0].bot, 100.0);
    }

    #[test]
    fn pas_de_ndog_si_gap_insuffisant() {
        let mut det = NdogDetector::new(900);
        det.update(&bar(100, 100.0, 100.5, 99.5, 100.0), 10.0); // atr=10 ⇒ gapMin=3
        // gap de 2 (102 vs 100) < 3 ⇒ pas créé.
        let ev = det.update(&bar(86_400, 102.0, 103.0, 101.0, 102.0), 10.0);
        assert!(ev.new_ndog.is_none());
        assert!(det.ndog_zones().is_empty());
    }

    #[test]
    fn fifo_un_seul_ndog() {
        let mut det = NdogDetector::new(900);
        det.update(&bar(100, 100.0, 101.0, 99.0, 100.0), 1.0);
        det.update(&bar(86_400, 110.0, 111.0, 109.0, 110.0), 1.0);
        det.update(&bar(2 * 86_400, 95.0, 96.0, 94.0, 95.0), 1.0);
        // 3ᵉ jour crée un nouveau NDOG ⇒ le 1ᵉʳ est évincé (FIFO 1).
        assert_eq!(det.ndog_zones().len(), 1, "FIFO ⇒ max 1 NDOG");
        // Le dernier NDOG est celui du 3ᵉ jour.
        assert_eq!(det.ndog_zones()[0].top, 110.0);
    }

    #[test]
    fn ndog_mitige_quand_prix_remplit_le_gap() {
        let mut det = NdogDetector::new(900);
        det.update(&bar(100, 100.0, 101.0, 99.0, 100.0), 1.0);
        det.update(&bar(86_400, 110.0, 111.0, 109.0, 110.0), 1.0);
        assert!(!det.ndog_zones()[0].mitigated);
        // Bar suivante : low <= 100 et high >= 110 ⇒ mitigé.
        det.update(&bar(86_400 + 900, 105.0, 112.0, 99.0, 106.0), 1.0);
        assert!(det.ndog_zones()[0].mitigated, "gap rempli ⇒ mitigé");
    }

    #[test]
    fn nwog_uniquement_en_h1_h4() {
        // M15 ⇒ tf_nwog inactif : pas de NWOG même en nouvelle semaine.
        let mut det = NdogDetector::new(900);
        det.update(&bar(100, 100.0, 101.0, 99.0, 100.0), 1.0);
        // ts loin dans le futur (autre semaine ISO).
        let ev = det.update(&bar(30 * 86_400, 110.0, 111.0, 109.0, 110.0), 1.0);
        assert!(ev.new_nwog.is_none(), "M15 ⇒ pas de NWOG");
        // NDOG oui (nouveau jour).
        assert!(ev.new_ndog.is_some());
    }

    #[test]
    fn nwog_cree_en_h1() {
        let mut det = NdogDetector::new(3600); // H1 ⇒ tf_nwog actif, tf_ndog inactif.
        det.update(&bar(100, 100.0, 101.0, 99.0, 100.0), 1.0);
        // Nouvelle semaine (ts = 30 jours plus tard).
        let ev = det.update(&bar(30 * 86_400, 110.0, 111.0, 109.0, 110.0), 1.0);
        assert!(ev.new_nwog.is_some(), "H1 + nouvelle semaine ⇒ NWOG");
        // NDOG gated off en H1 (tf_ndog inactif, 3600 > 900).
        assert!(ev.new_ndog.is_none(), "H1 ⇒ NDOG gated off");
    }

    #[test]
    fn pas_de_creation_a_la_toute_premiere_bar() {
        let mut det = NdogDetector::new(900);
        // Pas de prev_bar ⇒ pas de new_day/new_week.
        let ev = det.update(&bar(0, 100.0, 101.0, 99.0, 100.0), 1.0);
        assert!(ev.new_ndog.is_none() && ev.new_nwog.is_none());
    }
}
