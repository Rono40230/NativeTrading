//! BOS (Break of Structure) haussier/baissier + anti-doublon.
//!
//! Reproduit MODULE 2 Pine (lignes 435-450) :
//!   bosHaussier = sh1 non-na ET close > sh1 ET close[1] <= sh1
//!                 ET (dernierSH1_sig na OU bsh1 != dernierSH1_sig)
//!   bosBaissier = sl1 non-na ET close < sl1 ET close[1] >= sl1
//!                 ET (dernierSL1_sig na OU bsl1 != dernierSL1_sig)
//!
//! Au BOS, on fige `dernierSH1_sig := bsh1` (anti-doublon : un même pivot sh1 ne
//! peut déclencher qu'un seul BOS tant qu'il n'est pas rafraîchi par un nouveau pivot).
//!
//! NOTE : la condition Pine `and not mssHaussier` (lignes 524-527, 540) n'est PAS
//! appliquée à l'intérieur de ce détecteur : il expose le BOS BRUT pour que le
//! `MssDetector` (MODULE 3) puisse en déduire le MSS (`mssHaussier = tendanceBaissiere
//! and bosHaussier`). Le masque `bosHaussier and not mssHaussier` est appliqué au
//! niveau du moteur (`SmcV12Engine::update` via `mask_bos_by_mss`) sur l'événement
//! de sortie — un BOS qui est aussi un MSS n'est pas exposé deux fois.
//!
//! Rappel anti-doublon : `dernier_sh1_sig` est figé sur TOUT BOS brut (MSS inclus,
//! car MSS ⇒ BOS), ce qui couvre les deux affectations Pine (lignes 507 et 524).

use super::pivots::PivotDetector;
use super::structure::StructureDetector;
use super::types::{BarInput, BosEvent};

/// Détecteur de BOS avec anti-doublon par pivot.
pub struct BosDetector {
    /// `dernierSH1_sig` (Pine) : bar du dernier sh1 ayant signalé un BOS haussier.
    dernier_sh1_sig: Option<usize>,
    /// `dernierSL1_sig` (Pine) : bar du dernier sl1 ayant signalé un BOS baissier.
    dernier_sl1_sig: Option<usize>,
    dernier_bosh_level: Option<f64>,
    dernier_bosh_bar: Option<usize>,
    dernier_boss_level: Option<f64>,
    dernier_boss_bar: Option<usize>,
    /// `close[1]` Pine — close de la bar précédente.
    last_close: Option<f64>,
    bar_count: usize,
    last_event: BosEvent,
}

impl BosDetector {
    pub fn new() -> Self {
        Self {
            dernier_sh1_sig: None,
            dernier_sl1_sig: None,
            dernier_bosh_level: None,
            dernier_bosh_bar: None,
            dernier_boss_level: None,
            dernier_boss_bar: None,
            last_close: None,
            bar_count: 0,
            last_event: BosEvent::default(),
        }
    }

    pub fn update(
        &mut self,
        bar: &BarInput,
        pivots: &PivotDetector,
        _structure: &StructureDetector,
    ) {
        let cur_idx = self.bar_count;
        self.bar_count += 1;
        let mut ev = BosEvent::default();
        let prev_close = self.last_close;

        // --- BOS haussier (Pine ligne 437) ---
        if let (Some(sh1), Some(pc), Some(bsh1)) =
            (pivots.sh1(), prev_close, pivots.last_pivot_high_bar())
        {
            let anti_ok = match self.dernier_sh1_sig {
                None => true,
                Some(sig) => bsh1 != sig,
            };
            if anti_ok && bar.close > sh1 && pc <= sh1 {
                ev.bullish = true;
                ev.level = Some(sh1);
                ev.bar_index = Some(cur_idx);
                self.dernier_sh1_sig = Some(bsh1);
                self.dernier_bosh_level = Some(sh1);
                self.dernier_bosh_bar = Some(cur_idx);
            }
        }

        // --- BOS baissier (Pine ligne 438) ---
        if let (Some(sl1), Some(pc), Some(bsl1)) =
            (pivots.sl1(), prev_close, pivots.last_pivot_low_bar())
        {
            let anti_ok = match self.dernier_sl1_sig {
                None => true,
                Some(sig) => bsl1 != sig,
            };
            if anti_ok && bar.close < sl1 && pc >= sl1 {
                ev.bearish = true;
                ev.level = Some(sl1);
                ev.bar_index = Some(cur_idx);
                self.dernier_sl1_sig = Some(bsl1);
                self.dernier_boss_level = Some(sl1);
                self.dernier_boss_bar = Some(cur_idx);
            }
        }

        self.last_close = Some(bar.close);
        self.last_event = ev;
    }

    pub fn last_event(&self) -> BosEvent {
        self.last_event.clone()
    }
    pub fn dernier_bosh_level(&self) -> Option<f64> {
        self.dernier_bosh_level
    }
    pub fn dernier_boss_level(&self) -> Option<f64> {
        self.dernier_boss_level
    }
}

impl Default for BosDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Série haussière : pic high=110 à l'index 3 (sh1=110), puis close casse 110 à l'index 8.
    #[test]
    fn bos_haussier_detecte_quand_close_casse_sh1() {
        let mut piv = PivotDetector::new(3);
        let mut bos = BosDetector::new();
        let mut st = StructureDetector::new();

        let mut bull_at_8 = false;
        for i in 0..=8usize {
            let high = if i == 3 { 110.0 } else { 100.0 };
            let close = if i == 8 { 111.0 } else { 100.0 };
            piv.update(&bar(i, high, 90.0, close));
            st.update(&bar(i, high, 90.0, close), &piv.last_event());
            bos.update(&bar(i, high, 90.0, close), &piv, &st);
            if i == 8 {
                bull_at_8 = bos.last_event().bullish;
            }
        }
        assert!(bull_at_8, "close=111 > sh1=110 avec close[1]=100 ⇒ BOS haussier");
        assert_eq!(piv.sh1(), Some(110.0));
    }

    /// Anti-doublon : un 2ᵉ croisement du même sh1 (sans nouveau pivot) est bloqué.
    #[test]
    fn anti_doublon_bloque_second_bos_sur_meme_sh1() {
        let mut piv = PivotDetector::new(3);
        let mut bos = BosDetector::new();
        let mut st = StructureDetector::new();
        let closes = [100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 111.0, 109.0, 111.0];
        let mut bull_at_8 = false;
        let mut bull_at_10 = false;
        for (i, &c) in closes.iter().enumerate() {
            let high = if i == 3 { 110.0 } else { 100.0 };
            let b = bar(i, high, 90.0, c);
            piv.update(&b);
            st.update(&b, &piv.last_event());
            bos.update(&b, &piv, &st);
            if i == 8 {
                bull_at_8 = bos.last_event().bullish;
            }
            if i == 10 {
                bull_at_10 = bos.last_event().bullish;
            }
        }
        assert!(bull_at_8, "1er croisement ⇒ BOS");
        assert!(!bull_at_10, "2ᵉ croisement du même sh1 ⇒ bloqué par anti-doublon");
    }

    /// Série baissière : creux low=90 à l'index 3 (sl1=90), puis close casse 90 à l'index 8.
    #[test]
    fn bos_baissier_detecte_quand_close_casse_sl1() {
        let mut piv = PivotDetector::new(3);
        let mut bos = BosDetector::new();
        let mut st = StructureDetector::new();
        let mut bear_at_8 = false;
        for i in 0..=8usize {
            let low = if i == 3 { 90.0 } else { 100.0 };
            let close = if i == 8 { 89.0 } else { 100.0 };
            let b = bar(i, 110.0, low, close);
            piv.update(&b);
            st.update(&b, &piv.last_event());
            bos.update(&b, &piv, &st);
            if i == 8 {
                bear_at_8 = bos.last_event().bearish;
            }
        }
        assert!(bear_at_8, "close=89 < sl1=90 avec close[1]=100 ⇒ BOS baissier");
        assert_eq!(piv.sl1(), Some(90.0));
    }

    #[test]
    fn pas_de_bos_sans_pivot() {
        let mut piv = PivotDetector::new(3);
        let mut bos = BosDetector::new();
        let mut st = StructureDetector::new();
        let mut any_bos = false;
        for i in 0..10usize {
            let b = bar(i, 100.0, 90.0, 95.0);
            piv.update(&b);
            st.update(&b, &piv.last_event());
            bos.update(&b, &piv, &st);
            any_bos |= bos.last_event().bullish || bos.last_event().bearish;
        }
        assert!(!any_bos, "pas de pivot ⇒ pas de BOS");
    }
}
