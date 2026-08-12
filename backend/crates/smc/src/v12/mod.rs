//! SMC v12 — reproduction fidèle de `smc_indicateur_v12.pine`.
//!
//! Coexiste avec l'ancien `smc::scorer` jusqu'à validation, puis bascule.
//! Cette phase (2.0) pose le socle : calibration + ATR14 + pivots + structure + BOS.
//! Les modules suivants (liquidités, OB, FVG, sweep, scoring…) viendront en phases 2.1+.

pub mod atr;
pub mod bos;
pub mod calibration;
pub mod pivots;
pub mod structure;
pub mod types;
#[cfg(test)]
mod tests;

pub use atr::Atr14;
pub use bos::BosDetector;
pub use calibration::AssetCalibration;
pub use pivots::PivotDetector;
pub use structure::StructureDetector;
pub use types::*;

/// Le moteur SMC v12 — orchestre tous les indicateurs dans l'ordre strict du Pine.
pub struct SmcV12Engine {
    pub calibration: AssetCalibration,
    pub atr: Atr14,
    pub pivots: PivotDetector,
    pub structure: StructureDetector,
    pub bos: BosDetector,
}

impl SmcV12Engine {
    /// Crée le moteur pour un actif + timeframe donnés (calibration auto Module 0).
    pub fn new(asset: &str, timeframe: &str) -> Self {
        let cal = AssetCalibration::detect(asset, timeframe);
        Self {
            calibration: cal.clone(),
            atr: Atr14::new(),
            pivots: PivotDetector::new(cal.swing_length),
            structure: StructureDetector::new(),
            bos: BosDetector::new(),
        }
    }

    /// Traite une nouvelle bar clôturée. Ordre strict = ordre Pine
    /// (ATR → pivots → structure → BOS).
    pub fn update(&mut self, bar: &BarInput) -> SmcOutput {
        self.atr.update(bar);
        self.pivots.update(bar);
        let pivot_event = self.pivots.last_event();
        self.structure.update(bar, &pivot_event);
        let struct_event = self.structure.last_event();
        self.bos.update(bar, &self.pivots, &self.structure);

        SmcOutput {
            atr14: self.atr.value(),
            pivot: pivot_event,
            structure: struct_event,
            bos: self.bos.last_event(),
            sh1: self.pivots.sh1(),
            sl1: self.pivots.sl1(),
            tendance_haussiere: self.structure.tendance_haussiere(),
            tendance_baissiere: self.structure.tendance_baissiere(),
        }
    }
}
