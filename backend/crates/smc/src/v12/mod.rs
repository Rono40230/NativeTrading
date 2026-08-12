//! SMC v12 — reproduction fidèle de `smc_indicateur_v12.pine`.
//!
//! Coexiste avec l'ancien `smc::scorer` jusqu'à validation, puis bascule.
//! Phase 2.0 : socle (calibration + ATR14 + pivots + structure + BOS).
//! Phase 2.1 : MODULES 3/4/5 (MSS/CHOCH + Liquidités PDH/PDL/PWH/PWL + EQH/EQL + Sweep).

pub mod atr;
pub mod bos;
pub mod calibration;
pub mod liquidites;
pub mod mss;
pub mod pivots;
pub mod structure;
pub mod sweep;
pub mod types;
#[cfg(test)]
mod tests;

pub use atr::Atr14;
pub use bos::BosDetector;
pub use calibration::{tf_seconds, AssetCalibration};
pub use liquidites::LiquiditesDetector;
pub use mss::MssDetector;
pub use pivots::PivotDetector;
pub use structure::StructureDetector;
pub use sweep::SweepDetector;
pub use types::*;

/// Le moteur SMC v12 — orchestre tous les indicateurs dans l'ordre strict du Pine.
///
/// Ordre d'exécution `update` (Pine) :
///   ATR → Pivots → Structure → BOS → MSS/CHOCH → Liquidités (PDH/PDL/EQH/EQL) → Sweep
pub struct SmcV12Engine {
    pub calibration: AssetCalibration,
    pub atr: Atr14,
    pub pivots: PivotDetector,
    pub structure: StructureDetector,
    pub bos: BosDetector,
    pub mss: MssDetector,
    pub liquidites: LiquiditesDetector,
    pub sweep: SweepDetector,
    /// Timeframe en secondes (Pine `timeframe.in_seconds()`).
    tf_sec: i64,
}

impl SmcV12Engine {
    /// Crée le moteur pour un actif + timeframe donnés (calibration auto Module 0).
    pub fn new(asset: &str, timeframe: &str) -> Self {
        let cal = AssetCalibration::detect(asset, timeframe);
        let tf_sec = tf_seconds(timeframe);
        Self {
            calibration: cal.clone(),
            atr: Atr14::new(),
            pivots: PivotDetector::new(cal.swing_length),
            structure: StructureDetector::new(),
            bos: BosDetector::new(),
            mss: MssDetector::new(),
            liquidites: LiquiditesDetector::new(),
            sweep: SweepDetector::new(tf_sec),
            tf_sec,
        }
    }

    /// Traite une nouvelle bar clôturée. Ordre strict = ordre Pine
    /// (ATR → pivots → structure → BOS → MSS/CHOCH → liquidités → sweep).
    pub fn update(&mut self, bar: &BarInput) -> SmcOutput {
        // 1. ATR
        self.atr.update(bar);
        let atr14 = self.atr.value();

        // 2. Pivots
        self.pivots.update(bar);
        let pivot_event = self.pivots.last_event();

        // 3. Structure (HH/HL/LH/LL + tendance PRÉ-reset MSS).
        self.structure.update(bar, &pivot_event);
        let struct_event = self.structure.last_event();

        // 4. BOS (BRUT — fixe `dernier_sh1_sig` sur tout BOS, MSS inclus car MSS⇒BOS).
        self.bos.update(bar, &self.pivots, &self.structure);
        let bos_raw = self.bos.last_event();

        // 5. MSS / CHOCH — peut reset les compteurs de structure (effet bars suivants).
        let mss_event = self.mss.update(
            bar,
            &self.pivots,
            &struct_event,
            &bos_raw,
            &mut self.structure,
        );

        // Masque BOS pour la sortie (concern 2.0 #2, Pine lignes 524-527/540) :
        // un BOS qui est aussi un MSS n'est pas exposé comme BOS (pas de double-compte).
        let bos_out = mask_bos_by_mss(&bos_raw, &mss_event);

        // 6. Liquidités (PDH/PDL/PWH/PWL + EQH/EQL) — produit dernierEQH/EQL_level.
        let liq_event =
            self.liquidites
                .update(bar, &self.pivots, &pivot_event, atr14);

        // 7. Sweep — consomme dernierEQH/EQL_level et marque le pool sweepé.
        let sweep_event = self.sweep.update(bar, &mut self.liquidites, atr14);

        SmcOutput {
            atr14,
            pivot: pivot_event,
            structure: struct_event.clone(),
            bos: bos_out,
            mss: mss_event.clone(),
            liquidite: liq_event,
            sweep: sweep_event,
            sh1: self.pivots.sh1(),
            sl1: self.pivots.sl1(),
            // Tendance PRÉ-reset MSS (fidélité Pine : calculée ligne 381 avant reset 504).
            tendance_haussiere: struct_event.tendance_haussiere,
            tendance_baissiere: struct_event.tendance_baissiere,
        }
    }

    /// Timeframe en secondes (Pine `timeframe.in_seconds()`).
    pub fn tf_sec(&self) -> i64 {
        self.tf_sec
    }
}

/// Masque le BOS selon `bosHaussier and not mssHaussier` (Pine lignes 524-527, 540).
///
/// Renvoie un `BosEvent` dont les flags directionnels sont annulés lorsqu'un MSS
/// s'est produit sur la même bar. Le level/bar_index sont conservés si le flag reste.
fn mask_bos_by_mss(bos: &BosEvent, mss: &MssEvent) -> BosEvent {
    let bullish = bos.bullish && !mss.mss_haussier;
    let bearish = bos.bearish && !mss.mss_baissier;
    BosEvent {
        bullish,
        bearish,
        level: if bullish || bearish { bos.level } else { None },
        bar_index: if bullish || bearish { bos.bar_index } else { None },
    }
}
