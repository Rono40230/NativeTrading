//! MODULE 4b — Premium / Discount Zones (Equilibrium ICT).
//!
//! Reproduit MODULE 4b Pine (lignes 1654-1691) :
//!   - Plage de référence = dernier dealing range BOS (`_pdRangeH=sh1, _pdRangeL=sl1`
//!     capturés au BOS — Phase 3.1).
//!   - `pdEquilibrium = (_pdRangeH + _pdRangeL) / 2.0`
//!   - `pdTolAbs = pdEquilibrium * (i_eqTol / 100.0)` avec `i_eqTol = 0.5` (Pine ligne 1660).
//!   - `inPremium  = pdOk and close > pdEquilibrium + pdTolAbs`
//!   - `inDiscount = pdOk and close < pdEquilibrium - pdTolAbs`
//!
//! La capture se fait sur le BOS **BRUT** (Pine `bosHaussier`/`bosBaissier`, lignes
//! 1667-1669). Ces variables ne sont JAMAIS réaffectées par le masque MSS — elles
//! restent vraies même lorsqu'un MSS se produit sur la même bar. Le moteur passe donc
//! le BOS brut (`bos_raw`), pas le BOS masqué de sortie.

use super::types::{BarInput, PdEvent};

/// `i_eqTol` (Pine ligne 1660) — tolérance d'équilibre en % de l'equilibrium.
pub const EQ_TOL_PCT: f64 = 0.5;

/// Détecteur Premium/Discount (capture la plage au BOS, puis calcule l'equilibrium).
#[derive(Clone)]
pub struct PdDetector {
    /// `_pdRangeH` (Pine) = sh1 au dernier BOS.
    range_h: Option<f64>,
    /// `_pdRangeL` (Pine) = sl1 au dernier BOS.
    range_l: Option<f64>,
    last_event: PdEvent,
}

impl PdDetector {
    pub fn new() -> Self {
        Self {
            range_h: None,
            range_l: None,
            last_event: PdEvent::default(),
        }
    }

    /// Traite une bar.
    ///
    /// - `bos_bull` / `bos_bear` : BOS **brut** (Pine `bosHaussier`/`bosBaissier`).
    /// - `sh1` / `sl1` : derniers swings (PivotDetector).
    pub fn update(
        &mut self,
        bar: &BarInput,
        bos_bull: bool,
        bos_bear: bool,
        sh1: Option<f64>,
        sl1: Option<f64>,
    ) -> PdEvent {
        // Capture au BOS (Pine lignes 1667-1669) — BOS brut, MSS non masqué.
        if bos_bull || bos_bear {
            if let (Some(h), Some(l)) = (sh1, sl1) {
                if h > l {
                    self.range_h = Some(h);
                    self.range_l = Some(l);
                }
            }
        }

        let equilibrium = match (self.range_h, self.range_l) {
            (Some(h), Some(l)) => Some((h + l) / 2.0),
            _ => None,
        };
        let tol_abs = equilibrium.map(|eq| eq * (EQ_TOL_PCT / 100.0));

        let (in_premium, in_discount) = match (equilibrium, tol_abs) {
            (Some(eq), Some(tol)) => {
                // Évite toute surprise NaN/Infinity : comparaison stricte (Pine > / <).
                let close = bar.close;
                (close > eq + tol, close < eq - tol)
            }
            _ => (false, false),
        };

        let ev = PdEvent {
            in_premium,
            in_discount,
            equilibrium,
            pd_range_h: self.range_h,
            pd_range_l: self.range_l,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> PdEvent {
        self.last_event.clone()
    }
    pub fn equilibrium(&self) -> Option<f64> {
        self.last_event.equilibrium
    }
}

impl Default for PdDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(close: f64) -> BarInput {
        BarInput::new(close, close + 1.0, close - 1.0, close)
    }

    #[test]
    fn aucune_zone_avant_bos() {
        let mut det = PdDetector::new();
        let ev = det.update(&bar(100.0), false, false, None, None);
        assert!(!ev.in_premium && !ev.in_discount);
        assert!(ev.equilibrium.is_none());
    }

    #[test]
    fn capture_au_bos_puis_classifie() {
        let mut det = PdDetector::new();
        // BOS haussier, sh1=120, sl1=100 ⇒ eq=110, tol=110*0.005=0.55.
        let ev = det.update(&bar(100.0), true, false, Some(120.0), Some(100.0));
        // range capturée ; close=100 < 110-0.55 ⇒ discount.
        assert!(!ev.in_premium);
        assert!(ev.in_discount, "close=100 < eq-tol ⇒ discount");
        assert_eq!(ev.equilibrium, Some(110.0));
        assert_eq!(ev.pd_range_h, Some(120.0));
        assert_eq!(ev.pd_range_l, Some(100.0));
    }

    #[test]
    fn premium_quand_close_haute() {
        let mut det = PdDetector::new();
        det.update(&bar(100.0), true, false, Some(120.0), Some(100.0));
        // Bar suivante, sans BOS : range conservée, close=120 > eq+tol ⇒ premium.
        let ev = det.update(&bar(120.0), false, false, Some(120.0), Some(100.0));
        assert!(ev.in_premium, "close=120 > eq+tol ⇒ premium");
        assert!(!ev.in_discount);
    }

    #[test]
    fn re_capture_au_nouveau_bos() {
        let mut det = PdDetector::new();
        det.update(&bar(100.0), true, false, Some(120.0), Some(100.0));
        assert_eq!(det.equilibrium(), Some(110.0));
        // Nouveau BOS ⇒ nouvelle plage.
        det.update(&bar(100.0), true, false, Some(200.0), Some(180.0));
        assert_eq!(det.equilibrium(), Some(190.0));
    }

    #[test]
    fn pas_de_capture_si_sh1_sl1_incoherents() {
        let mut det = PdDetector::new();
        // sh1 <= sl1 ⇒ condition `sh1 > sl1` (Pine ligne 1667) non respectée.
        let ev = det.update(&bar(100.0), true, false, Some(100.0), Some(120.0));
        assert!(ev.equilibrium.is_none(), "sh1<=sl1 ⇒ pas de capture");
    }
}
