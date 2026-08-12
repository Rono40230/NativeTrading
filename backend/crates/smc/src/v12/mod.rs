//! SMC v12 — reproduction fidèle de `smc_indicateur_v12.pine`.
//!
//! Coexiste avec l'ancien `smc::scorer` jusqu'à validation, puis bascule.
//! Phase 2.0 : socle (calibration + ATR14 + pivots + structure + BOS).
//! Phase 2.1 : MODULES 3/4/5 (MSS/CHOCH + Liquidités PDH/PDL/PWH/PWL + EQH/EQL + Sweep).
//! Phase 2.2 : MODULES 6/7/8b/8c/13b (FVG + Order Blocks + Breaker + Propulsion + Imbalance).
//! Phase 2.3 : MODULES 4b/10b/12/13c + Kill Zones + Zone-cœur (contexte).

pub mod atr;
pub mod bos;
pub mod bs_helpers;
pub mod breaker;
pub mod calibration;
pub mod fvg;
pub mod imbalance;
pub mod kill_zones;
pub mod lifecycle;
pub mod liquidites;
pub mod mss;
pub mod mtf;
pub mod ndog;
pub mod order_blocks;
pub mod ote;
pub mod pivots;
pub mod premium_discount;
pub mod propulsion;
pub mod scoring_bs_zones;
pub mod scoring_v11;
pub mod signals;
pub mod structure;
pub mod sweep;
pub mod trade;
pub mod types;
#[cfg(test)]
mod tests;
pub mod zone_coeur;

pub use atr::Atr14;
pub use bos::BosDetector;
pub use breaker::BreakerDetector;
pub use calibration::{tf_seconds, AssetCalibration};
pub use fvg::FvgDetector;
pub use imbalance::ImbalanceDetector;
pub use kill_zones::KillZoneDetector;
pub use lifecycle::TradeLifecycle;
pub use liquidites::LiquiditesDetector;
pub use mss::MssDetector;
pub use mtf::MtfDetector;
pub use ndog::NdogDetector;
pub use order_blocks::ObDetector;
pub use ote::OteDetector;
pub use pivots::PivotDetector;
pub use premium_discount::PdDetector;
pub use propulsion::PropulsionDetector;
pub use scoring_bs_zones::ScoringBsZones;
pub use scoring_v11::ScoringV11;
pub use signals::SignalGenerator;
pub use structure::StructureDetector;
pub use sweep::SweepDetector;
pub use trade::Trade;
pub use types::*;
pub use zone_coeur::ZoneCoeurDetector;

/// Le moteur SMC v12 — orchestre tous les indicateurs dans l'ordre strict du Pine.
///
/// Ordre d'exécution `update` (Pine) :
///   ATR → Pivots → Structure → BOS → MSS/CHOCH → Liquidités (PDH/PDL/EQH/EQL) → Sweep
///   → FVG → Order Blocks → Breaker → Propulsion → Imbalance
///   → Premium/Discount → OTE → Kill Zones → NDOG/NWOG → MTF → Zone-cœur
pub struct SmcV12Engine {
    pub calibration: AssetCalibration,
    pub atr: Atr14,
    pub pivots: PivotDetector,
    pub structure: StructureDetector,
    pub bos: BosDetector,
    pub mss: MssDetector,
    pub liquidites: LiquiditesDetector,
    pub sweep: SweepDetector,
    pub fvg: FvgDetector,
    pub order_blocks: ObDetector,
    pub breaker: BreakerDetector,
    pub propulsion: PropulsionDetector,
    pub imbalance: ImbalanceDetector,
    /// MODULE 4b — Premium/Discount.
    pub premium_discount: PdDetector,
    /// MODULE 13c — Fibonacci OTE.
    pub ote: OteDetector,
    /// Kill Zones (UTC).
    pub kill_zone: KillZoneDetector,
    /// MODULE 10b — NDOG/NWOG.
    pub ndog: NdogDetector,
    /// MODULE 12 — Multi-Timeframe.
    pub mtf: MtfDetector,
    /// Zone-cœur (intersection OB ∩ OTE ∩ FVG).
    pub zone_coeur: ZoneCoeurDetector,
    // --- Phase 2.5 : CERVEAU (scoring + signaux + lifecycle) ---
    /// MODULE 11 — Scoring v11 (OB).
    pub scoring_v11: ScoringV11,
    /// MODULE BSZones — second moteur de scoring + zones.
    pub scoring_bs: ScoringBsZones,
    /// Générateur de signaux + carnet de trades.
    pub signals: SignalGenerator,
    /// Cycle de vie des trades (SL/BE/TP/expire intrabar).
    pub lifecycle: TradeLifecycle,
    /// Historique rolling (dernier = bar courante) pour les lookbacks `[1]..[20]`.
    history: Vec<BarInput>,
    /// Compteur de bars (= `bar_index` Pine, 0-based).
    bar_count: usize,
    /// Timeframe en secondes (Pine `timeframe.in_seconds()`).
    tf_sec: i64,
}

impl SmcV12Engine {
    /// Crée le moteur pour un actif + timeframe donnés (calibration auto Module 0).
    pub fn new(asset: &str, timeframe: &str) -> Self {
        let cal = AssetCalibration::detect(asset, timeframe);
        let tf_sec = tf_seconds(timeframe);
        let tf_mins = calibration::tf_minutes(timeframe);
        let trade_max_secs = trade_max_mins(tf_mins) * 60;
        let tp3_max_secs = tp3_max_mins(&cal, tf_mins) * 60;
        Self {
            scoring_v11: ScoringV11::new(&cal, tf_mins),
            scoring_bs: ScoringBsZones::new(),
            signals: SignalGenerator::new(),
            lifecycle: TradeLifecycle::new(trade_max_secs, tp3_max_secs),
            history: Vec::with_capacity(32),
            bar_count: 0,
            calibration: cal.clone(),
            atr: Atr14::new(),
            pivots: PivotDetector::new(cal.swing_length),
            structure: StructureDetector::new(),
            bos: BosDetector::new(),
            mss: MssDetector::new(),
            liquidites: LiquiditesDetector::new(),
            sweep: SweepDetector::new(tf_sec),
            fvg: FvgDetector::new(),
            order_blocks: ObDetector::new(),
            breaker: BreakerDetector::new(),
            propulsion: PropulsionDetector::new(),
            imbalance: ImbalanceDetector::new(),
            premium_discount: PdDetector::new(),
            ote: OteDetector::new(tf_sec),
            kill_zone: KillZoneDetector::new(),
            ndog: NdogDetector::new(tf_sec),
            mtf: MtfDetector::new(),
            zone_coeur: ZoneCoeurDetector::new(),
            tf_sec,
        }
    }

    /// Traite une nouvelle bar clôturée. Ordre strict = ordre Pine
    /// (ATR → pivots → structure → BOS → MSS/CHOCH → liquidités → sweep
    ///  → FVG → OB → Breaker → Propulsion → Imbalance).
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

        // 8. FVG (MODULE 6) — détection + lifecycle. Produit les bornes pour Propulsion.
        let fvg_event = self.fvg.update(bar, atr14);

        // 9. Order Blocks (MODULE 7) — ROC + impulsion + lifecycle 3 états.
        //    Lit ibBull[1]/ibBear[1] (= imbalance de la bar précédente, car l'Imbalance
        //    s'exécute en dernier) ; crée des Breakers à l'invalidation.
        let ob_event = self.order_blocks.update(
            bar,
            self.calibration.roc_seuil,
            self.imbalance.last_ib_bull(),
            self.imbalance.last_ib_bear(),
            &mut self.breaker,
        );

        // 10. Breaker (MODULE 8b) — lifecycle de suppression (Pine f_bbLifecycle).
        let breaker_event = self.breaker.update(bar);

        // 11. Propulsion (MODULE 8c) — chevauchement FVG∩OB (post-lifecycle).
        let propulsion_event = self.propulsion.update(bar, &fvg_event, &self.order_blocks);

        // 12. Imbalance (MODULE 13b) — flags + zones. S'exécute en dernier afin que
        //     son flag `last_ib_*` soit lisible comme `[1]` par l'OB à la bar suivante.
        let imbalance_event = self.imbalance.update(bar, atr14, self.calibration.seuil_ib);

        // --- Contexte (Phase 2.3) ---
        // 13. Premium/Discount (MODULE 4b) — capture au BOS BRUT (bos_raw, non masqué MSS).
        let sh1 = self.pivots.sh1();
        let sl1 = self.pivots.sl1();
        let pd_event =
            self.premium_discount
                .update(bar, bos_raw.bullish, bos_raw.bearish, sh1, sl1);

        // 14. OTE (MODULE 13c) — capture au BOS BRUT + expiration temporelle.
        let ote_event = self
            .ote
            .update(bar, bos_raw.bullish, bos_raw.bearish, sh1, sl1);

        // 15. Kill Zones (timestamp UTC uniquement).
        let kz_event = self.kill_zone.update(bar);

        // 16. NDOG/NWOG (MODULE 10b) — gaps jour/semaine (gating TF).
        let ndog_event = self.ndog.update(bar, atr14);

        // 17. MTF (MODULE 12) — agrégation H1/H4/W1/MN + confluences (repaint assumé).
        let mtf_event = self.mtf.update(bar);

        // 18. Zone-cœur — intersection OB ∩ OTE ∩ FVG (post-lifecycle).
        //     Lit les zones vivantes (bull_zones/bear_zones) + bornes OTE + sweep frais.
        let zone_coeur_event = self.zone_coeur.update(
            self.order_blocks.bull_zones(),
            self.order_blocks.bear_zones(),
            self.fvg.bull_zones(),
            self.fvg.bear_zones(),
            self.ote.bull_bounds(),
            self.ote.bear_bounds(),
            sweep_event.sweep_bull_frais,
            sweep_event.sweep_bear_frais,
            pd_event.equilibrium,
        );

        let out = SmcOutput {
            atr14,
            pivot: pivot_event,
            structure: struct_event.clone(),
            bos: bos_out,
            mss: mss_event.clone(),
            liquidite: liq_event,
            sweep: sweep_event,
            fvg: fvg_event,
            order_blocks: ob_event,
            breaker: breaker_event,
            propulsion: propulsion_event,
            imbalance: imbalance_event,
            premium_discount: pd_event,
            ote: ote_event,
            kill_zone: kz_event,
            ndog: ndog_event,
            mtf: mtf_event,
            zone_coeur: zone_coeur_event,
            sh1: self.pivots.sh1(),
            sl1: self.pivots.sl1(),
            // Tendance PRÉ-reset MSS (fidélité Pine : calculée ligne 381 avant reset 504).
            tendance_haussiere: struct_event.tendance_haussiere,
            tendance_baissiere: struct_event.tendance_baissiere,
        };

        // --- Phase 2.5 : CERVEAU (scoring + signaux + lifecycle) ---
        // Historique rolling (lookbacks [1]..[20]) + bar_index global.
        self.history.push(*bar);
        if self.history.len() > 30 {
            self.history.remove(0);
        }
        let bar_index = self.bar_count;
        self.bar_count += 1;

        // 19. Scoring v11 — f_accumScores sur les OB vivants (freshness + proximity).
        {
            let ob_bull = self.order_blocks.bull_zones();
            let ob_bear = self.order_blocks.bear_zones();
            self.scoring_v11
                .update(&out, bar, &self.calibration, ob_bull, ob_bear);
        }
        // 20. Scoring BSZones — naissances (gate HTF) + lifecycle (mitigation).
        {
            let fvg_bull = self.fvg.bull_zones();
            let fvg_bear = self.fvg.bear_zones();
            self.scoring_bs.update(
                &out,
                bar,
                fvg_bull,
                fvg_bear,
                &self.history,
                bar_index,
                self.tf_sec,
            );
        }
        // 21. Signaux — v11 + BSZones (anti-doublon : 1 trade max par bar).
        {
            let ob_bull = self.order_blocks.bull_zones();
            let ob_bear = self.order_blocks.bear_zones();
            let fvg_bull = self.fvg.bull_zones();
            let fvg_bear = self.fvg.bear_zones();
            self.signals.reset_bar();
            self.signals.generate(
                &out,
                bar,
                bar_index,
                &self.calibration,
                ob_bull,
                ob_bear,
                &mut self.scoring_v11,
                &mut self.scoring_bs,
                fvg_bull,
                fvg_bear,
            );
        }
        // 22. Lifecycle — évaluation intrabar (fill/SL/BE/TP/expire/BE-forcé).
        self.lifecycle.update(
            &mut self.signals.trades,
            &out,
            bar,
            bar_index,
            &self.calibration,
            &self.scoring_v11,
        );

        out
    }

    /// Timeframe en secondes (Pine `timeframe.in_seconds()`).
    pub fn tf_sec(&self) -> i64 {
        self.tf_sec
    }

    /// Nombre de bars traitées (= prochain `bar_index`).
    pub fn bar_index(&self) -> usize {
        self.bar_count
    }
}

/// `_autoTradeMaxMins` (Pine 2374) — durée max trade en minutes selon le TF.
fn trade_max_mins(tf_mins: u32) -> i64 {
    match tf_mins {
        60 => 480,             // H1
        240 => 1920,           // H4
        1440 => 5760,          // D1
        _ => 240,              // défaut (M1–M30)
    }
}

/// `_autoTp3Mins` (Pine 71-76) — durée max TP3 en minutes selon asset × TF.
fn tp3_max_mins(cal: &AssetCalibration, tf_mins: u32) -> i64 {
    let m15 = tf_mins == 15;
    let h1 = tf_mins == 60;
    if cal.is_xau {
        if m15 { 60 } else if h1 { 240 } else { 60 }
    } else if cal.is_xag {
        if m15 { 45 } else if h1 { 180 } else { 60 }
    } else if cal.is_nas {
        if m15 { 30 } else if h1 { 120 } else { 60 }
    } else if cal.is_btc {
        if m15 { 90 } else if h1 { 360 } else { 60 }
    } else if cal.is_dax {
        if m15 { 30 } else if h1 { 120 } else { 60 }
    } else {
        60
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
