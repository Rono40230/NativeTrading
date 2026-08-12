//! Détection pivots high/low + maintenance sh1/sl1/sh2/sl2.
//!
//! Reproduit MODULE 1 Pine (lignes 314-366) :
//!   `ph = ta.pivothigh(high, i_swingLength, i_swingLength)`
//!   `pl = ta.pivotlow (low,  i_swingLength, i_swingLength)`
//!   `barPivot = bar_index[i_swingLength]`
//!
//! Une bar candidate (à `swingLength` bars de la bar courante) est un pivot high
//! ssi son `high` est **strictement** supérieur aux `swingLength` bars avant ET après.
//! La comparaison stricte (`>`) reproduit le fix MQL5 2026-07-27 (test `>=` de rejet)
//! qui gère les plateaux : sur un plateau d'égalité, AUCUN pivot n'est émis.

use super::types::{BarInput, PivotEvent};

/// Détecteur de pivots (swings) high/low avec mémoire sh1/sl1/sh2/sl2.
pub struct PivotDetector {
    swing_length: usize,
    bars: Vec<BarInput>,
    sh1: Option<f64>,
    sh2: Option<f64>,
    sl1: Option<f64>,
    sl2: Option<f64>,
    /// Index de la bar du pivot high courant (Pine `bsh1`) — anti-doublon BOS.
    last_pivot_high_bar: Option<usize>,
    /// Index de la bar du pivot low courant (Pine `bsl1`).
    last_pivot_low_bar: Option<usize>,
    last_event: PivotEvent,
}

impl PivotDetector {
    pub fn new(swing_length: usize) -> Self {
        Self {
            swing_length,
            bars: Vec::new(),
            sh1: None,
            sh2: None,
            sl1: None,
            sl2: None,
            last_pivot_high_bar: None,
            last_pivot_low_bar: None,
            last_event: PivotEvent::default(),
        }
    }

    pub fn update(&mut self, bar: &BarInput) {
        self.bars.push(*bar);
        // L'événement reflète UNIQUEMENT la détection de la bar courante.
        self.last_event = PivotEvent::default();

        let n = self.bars.len();
        let sl = self.swing_length;
        // Il faut au moins 2*sl+1 bars pour confirmer un pivot (sl barres de chaque côté).
        if n < 2 * sl + 1 {
            return;
        }

        // La candidate est à `sl` bars de la fin (= bar_index - swingLength en Pine).
        let pivot_idx = n - sl - 1;
        let pivot_high = self.bars[pivot_idx].high;
        let pivot_low = self.bars[pivot_idx].low;

        // Strictement > / < tous les voisins (gestion plateaux = fix MQL5 2026-07-27).
        let is_ph = (1..=sl).all(|i| {
            pivot_high > self.bars[pivot_idx - i].high && pivot_high > self.bars[pivot_idx + i].high
        });
        let is_pl = (1..=sl).all(|i| {
            pivot_low < self.bars[pivot_idx - i].low && pivot_low < self.bars[pivot_idx + i].low
        });

        if is_ph {
            self.sh2 = self.sh1;
            self.sh1 = Some(pivot_high);
            self.last_pivot_high_bar = Some(pivot_idx);
            self.last_event.is_pivot_high = true;
            self.last_event.pivot_high_price = Some(pivot_high);
            self.last_event.pivot_bar_index = Some(pivot_idx);
        }
        if is_pl {
            self.sl2 = self.sl1;
            self.sl1 = Some(pivot_low);
            self.last_pivot_low_bar = Some(pivot_idx);
            self.last_event.is_pivot_low = true;
            self.last_event.pivot_low_price = Some(pivot_low);
            if self.last_event.pivot_bar_index.is_none() {
                self.last_event.pivot_bar_index = Some(pivot_idx);
            }
        }
    }

    pub fn last_event(&self) -> PivotEvent {
        self.last_event.clone()
    }
    pub fn sh1(&self) -> Option<f64> {
        self.sh1
    }
    pub fn sl1(&self) -> Option<f64> {
        self.sl1
    }
    pub fn sh2(&self) -> Option<f64> {
        self.sh2
    }
    pub fn sl2(&self) -> Option<f64> {
        self.sl2
    }
    pub fn last_pivot_high_bar(&self) -> Option<usize> {
        self.last_pivot_high_bar
    }
    pub fn last_pivot_low_bar(&self) -> Option<usize> {
        self.last_pivot_low_bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit une bar où seul `high` et `low` nous intéressent.
    fn barhi(i: usize, high: f64, low: f64) -> BarInput {
        BarInput {
            timestamp: i as i64,
            open: low,
            high,
            low,
            close: high,
            volume: 0.0,
        }
    }

    #[test]
    fn pivot_high_detecte_sur_pic_central() {
        // sl=3 ⇒ pic à l'index 3 (3 bars avant : 0,1,2 ; 3 bars après : 4,5,6).
        let mut det = PivotDetector::new(3);
        let highs = [100.0, 100.0, 100.0, 110.0, 100.0, 100.0, 100.0];
        for (i, &h) in highs.iter().enumerate() {
            det.update(&barhi(i, h, 90.0));
        }
        assert!(det.last_event().is_pivot_high, "pic central ⇒ pivot high");
        assert_eq!(det.sh1(), Some(110.0));
        assert_eq!(det.last_pivot_high_bar(), Some(3));
    }

    #[test]
    fn pivot_low_detecte_sur_creux_central() {
        let mut det = PivotDetector::new(3);
        let lows = [100.0, 100.0, 100.0, 90.0, 100.0, 100.0, 100.0];
        for (i, &l) in lows.iter().enumerate() {
            det.update(&barhi(i, 110.0, l));
        }
        assert!(det.last_event().is_pivot_low);
        assert_eq!(det.sl1(), Some(90.0));
        assert_eq!(det.last_pivot_low_bar(), Some(3));
    }

    #[test]
    fn sh2_mis_a_jour_apres_deux_pivots_high() {
        // 2 pics : index 3 (110) et index 9 (120), sl=3, 13 bars.
        let mut det = PivotDetector::new(3);
        for i in 0..13usize {
            let h = if i == 3 {
                110.0
            } else if i == 9 {
                120.0
            } else {
                100.0
            };
            det.update(&barhi(i, h, 90.0));
        }
        assert_eq!(det.sh1(), Some(120.0), "sh1 = dernier pic");
        assert_eq!(det.sh2(), Some(110.0), "sh2 = avant-dernier pic");
    }

    #[test]
    fn pas_de_pivot_sur_plateau() {
        // Plateau d'égalité aux indices 3 et 4 (high=110) ⇒ strict > rejette, aucun pivot.
        let mut det = PivotDetector::new(3);
        let highs = [
            100.0, 100.0, 100.0, 110.0, 110.0, 100.0, 100.0, 100.0, 100.0,
        ];
        for (i, &h) in highs.iter().enumerate() {
            det.update(&barhi(i, h, 90.0));
        }
        assert!(det.sh1().is_none(), "plateau ⇒ aucun pivot high (strict >)");
    }

    #[test]
    fn pas_de_pivot_sur_serie_monotone() {
        let mut det = PivotDetector::new(3);
        for i in 0..12usize {
            det.update(&barhi(i, 100.0 + i as f64, 90.0)); // highs strictement croissants
        }
        assert!(det.sh1().is_none());
        assert!(det.sl1().is_none());
    }
}
