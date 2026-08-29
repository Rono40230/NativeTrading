//! SMC v12 — reproduction fidèle de `smc_indicateur_v12.pine`.
//!
//! Coexiste avec l'ancien `smc::scorer` jusqu'à validation, puis bascule.
//! Phase 2.0 : socle (calibration + ATR14 + pivots + structure + BOS).
//! Phase 2.1 : MODULES 3/4/5 (MSS/CHOCH + Liquidités PDH/PDL/PWH/PWL + EQH/EQL + Sweep).
//! Phase 2.2 : MODULES 6/7/8b/8c/13b (FVG + Order Blocks + Breaker + Propulsion + Imbalance).
//! Phase 2.3 : MODULES 4b/10b/12/13c + Kill Zones + Zone-cœur (contexte).

pub mod asian_hl;
pub mod atr;
pub mod bos;
pub mod bpr;
pub mod breaker;
pub mod bs_helpers;
pub mod calibration;
pub mod fvg;
pub mod imbalance;
pub mod durees;
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
pub mod sentiment;
pub mod signals;
pub mod structure;
pub mod sweep;
#[cfg(test)]
mod tests;
pub mod trade;
pub mod types;
pub mod zone_coeur;

pub use asian_hl::{AsianHlDetector, SessHlLevels};
use durees::{mask_bos_by_mss, tp3_max_mins, trade_max_mins};
pub use atr::Atr14;
pub use bos::BosDetector;
pub use bpr::{bonus_bpr, BprDetector, BprEvent, BprState, BprZone};
pub use breaker::BreakerDetector;
pub use calibration::{tf_seconds, AssetCalibration};
pub use fvg::FvgDetector;
pub use imbalance::ImbalanceDetector;
pub use kill_zones::KillZoneDetector;
pub use lifecycle::TradeLifecycle;
pub use liquidites::LiquiditesDetector;
pub use mss::MssDetector;
pub use mtf::{agreger_mensuel, AmorceMtf, MtfDetector};
pub use ndog::NdogDetector;
pub use order_blocks::ObDetector;
pub use ote::OteDetector;
pub use pivots::PivotDetector;
pub use premium_discount::PdDetector;
pub use propulsion::PropulsionDetector;
pub use scoring_bs_zones::ScoringBsZones;
pub use scoring_v11::ScoringV11;
pub use sentiment::{
    agreg_par_classe, calculer_sentiment_technique, classe_actif, Alignement, SentimentScore,
    SentimentVerdict,
};
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
#[derive(Clone)]
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
    /// MODULE 6b — BPR (Balanced Price Range).
    pub bpr: BprDetector,
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
    /// Bonus de scoring BPR (MODULE 6b) — défaut INACTIF : étude comparatif_bpr
    /// 28/08 = +1.0R / 2 834 clôtures (bruit) → « affichage conservé, scoring
    /// retiré » (parité Pine). Le greffon reste ré-activable pour ré-étude.
    bpr_scoring: bool,
    // --- Phase 2.5 : CERVEAU (scoring + signaux + lifecycle) ---
    /// MODULE 11 — Scoring v11 (OB).
    pub scoring_v11: ScoringV11,
    /// MODULE 14 — Asian High/Low (DoL znQual + TP3).
    pub asian_hl: AsianHlDetector,
    /// MODULE 14b — London High/Low (Module F — mêmes bornes Paris que le Pine).
    pub london_hl: AsianHlDetector,
    /// Événements de session de la bar N-1 (parité Pine : f_score lit les
    /// drawn AVANT la mise à jour MODULE 14/14b — état N-1).
    asian_hl_prec: asian_hl::AsianHlEvent,
    london_hl_prec: asian_hl::AsianHlEvent,
    /// Bonus Module H (mega-orders volume ≥ 2× SMA20) — défaut ACTIF (étalon
    /// Pine pré-verdict ; l'étude comparatif_mega tranche).
    mega_vol_scoring: bool,
    /// Bonus Module F (sessions H/L) — défaut INACTIF : étude comparatif_sessions
    /// du 28/08 = ON ≡ OFF bit-à-bit (2 771 clôtures, zéro trade changé ; la
    /// sonde probe_sessions prouve que le greffon s'activait — le +2 n'a
    /// jamais franchi un seuil). Décision : bonus retiré (parité Pine),
    /// tracking Londres + affichage conservés, greffon ré-activable en étude.
    sess_hl_scoring: bool,
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
            asian_hl: AsianHlDetector::new(),
            london_hl: AsianHlDetector::new()
                .avec_fenetre(
                    asian_hl::LONDON_DEBUT_MIN,
                    asian_hl::LONDON_FIN_MIN,
                ),
            asian_hl_prec: asian_hl::AsianHlEvent::default(),
            london_hl_prec: asian_hl::AsianHlEvent::default(),
            sess_hl_scoring: false,
            mega_vol_scoring: true,
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
            bpr: BprDetector::new(),
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
            bpr_scoring: false,
            tf_sec,
        }
    }

    /// Mode du BE forcé sur BOS opposé (étude comparatif — défaut
    /// Classique = production fidèle Pine v12).
    pub fn avec_mode_be_force(mut self, mode: lifecycle::ModeBeForce) -> Self {
        self.lifecycle.definir_mode_be_force(mode);
        self
    }

    /// Mode TP3 (défaut DolCappe3R = production, décision DoL≤3R du 28/08).
    pub fn avec_mode_tp3(mut self, mode: signals::ModeTp3) -> Self {
        self.signals.definir_mode_tp3(mode);
        self
    }

    /// Bonus de scoring BPR (MODULE 6b) — défaut actif (parité étalon Pine).
    /// La détection/lifecycle BPR tourne toujours ; seul le greffon de score
    /// (`+4/+3/+1` sur OB v11 et BSZones) est coupé par `false`.
    pub fn avec_scoring_bpr(mut self, actif: bool) -> Self {
        self.bpr_scoring = actif;
        self
    }

    /// Bonus Module F — sessions H/L Asie/Londres (+2 proximité). Défaut
    /// inactif (étude 28/08 : ON ≡ OFF bit-à-bit) ; ré-activable en étude.
    pub fn avec_scoring_sessions(mut self, actif: bool) -> Self {
        self.sess_hl_scoring = actif;
        self
    }

    /// Bonus Module H — mega-orders (+2 si volume[1] ≥ 2× SMA20[1]).
    pub fn avec_scoring_mega_volume(mut self, actif: bool) -> Self {
        self.mega_vol_scoring = actif;
        self
    }

    /// R1 (étude étape 3) : sweep directionnel frais requis en qualification
    /// v11 — canon ICT (« prérequis, pas bonus »). Défaut inactif = production
    /// pré-verdict ; l'étude comparatif_sweep tranche.
    pub fn avec_sweep_requis(mut self, actif: bool) -> Self {
        self.signals.definir_sweep_requis(actif);
        self
    }

    /// Traite une nouvelle bar clôturée. Ordre strict = ordre Pine
    /// (ATR → pivots → structure → BOS → MSS/CHOCH → liquidités → sweep
    ///  → FVG → OB → Breaker → Propulsion → Imbalance).
    /// Amorce le détecteur MTF avec l'historique HTF de la DB (H1/H4/W1/MN),
    /// AVANT tout replay LTF — sinon `f_htf` ne voit que la fenêtre LTF et les
    /// confluences W1 (+5) / MN (+6) du scoring sont structurellement froides
    /// (Pine/TV : `request.security` sur des années d'historique).
    /// `t0` = timestamp de la 1re bar LTF qui suivra.
    pub fn primer_mtf(
        &mut self,
        h1: &[BarInput],
        h4: &[BarInput],
        w1: &[BarInput],
        mn: &[BarInput],
        t0: i64,
    ) {
        self.mtf.primer(h1, h4, w1, mn, t0);
    }

    /// Variante struct ([`AmorceMtf`]).
    pub fn primer_mtf_amorce(&mut self, a: &AmorceMtf, t0: i64) {
        self.mtf.primer_amorce(a, t0);
    }

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
        let liq_event = self
            .liquidites
            .update(bar, &self.pivots, &pivot_event, atr14);

        // 7. Sweep — consomme dernierEQH/EQL_level et marque le pool sweepé.
        let sweep_event = self.sweep.update(bar, &mut self.liquidites, atr14);
        // 7b. « Décisions trading » 23/08 : consommation à l'ATTEINTE des
        // dernierEQ*/pool (après le sweep — son armement lit les niveaux).
        self.liquidites.consommer_niveaux_atteints(bar);

        // 8. FVG (MODULE 6) — détection + lifecycle. Produit les bornes pour Propulsion.
        // 8b. BPR (MODULE 6b) — le Pine apparie le gap naissant au pool opposé
        //     AVANT f_fvg*BearLifecycle : un gap opposé dont la clôture de
        //     remplissage le retire du pool cette bar reste appariantable
        //     (« les deux gaps sont délivrés »). D'où le snapshot PRÉ-lifecycle
        //     (fvg.update fait création + lifecycle d'un bloc).
        let opp_bull_pre = self.fvg.bull_zones().to_vec();
        let opp_bear_pre = self.fvg.bear_zones().to_vec();
        let fvg_event = self.fvg.update(bar, atr14);
        let _bpr_event = self
            .bpr
            .update(bar, &fvg_event, &opp_bull_pre, &opp_bear_pre);

        // 9. Order Blocks (MODULE 7) — ROC + impulsion + lifecycle 3 états.
        //    Lit ibBull[1]/ibBear[1] (= imbalance de la bar précédente, car l'Imbalance
        //    s'exécute en dernier) ; crée des Breakers à l'invalidation.
        let ob_event = self.order_blocks.update(
            bar,
            self.calibration.roc_seuil,
            self.imbalance.last_ib_bull(),
            self.imbalance.last_ib_bear(),
            &mut self.breaker,
            &sweep_event,
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
        // 18b. Asian High/Low (MODULE 14) — niveaux drawn pour znQual/TP3.
        let asian_ev = self.asian_hl.update(bar);
        let london_ev = self.london_hl.update(bar);

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
            bos_raw: bos_raw.clone(),
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
            asian_hl: asian_ev,
            london_hl: london_ev,
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

        // 19. Scoring v11 — f_accumScores sur les OB vivants (freshness + proximity
        //     + bonus BPR Module 6b).
        {
            let ob_bull = self.order_blocks.bull_zones();
            let ob_bear = self.order_blocks.bear_zones();
            let bpr_zones: &[bpr::BprZone] = if self.bpr_scoring {
                self.bpr.zones()
            } else {
                &[]
            };
            // Module F : niveaux de session à l'état N-1 (parité Pine f_score).
            let sess_prec = asian_hl::SessHlLevels {
                ah_high: self.asian_hl_prec.high,
                ah_low: self.asian_hl_prec.low,
                ld_high: self.london_hl_prec.high,
                ld_low: self.london_hl_prec.low,
            };
            let sess_hl = if self.sess_hl_scoring {
                Some(&sess_prec)
            } else {
                None
            };
            // Module H — mega-order : volume[1] ≥ 2× SMA20[1] (même fenêtre
            // que le _volScore BSZones : bars [1..20], bougie courante exclue).
            let mega_vol = if self.mega_vol_scoring {
                let sma = bs_helpers::vol_sma_20(&self.history);
                let vol_prec = bs_helpers::bar_volume_ago(&self.history, 1);
                Some(sma > 0.0 && vol_prec >= scoring_v11::MEGA_VOL_MULT * sma)
            } else {
                None
            };
            self.scoring_v11.update(
                &out, bar, &self.calibration, ob_bull, ob_bear, bpr_zones, sess_hl, mega_vol,
            );
        }
        // 20. Scoring BSZones — naissances (gate HTF) + lifecycle (mitigation)
        //     + bonus BPR Module 6b.
        {
            let fvg_bull = self.fvg.bull_zones();
            let fvg_bear = self.fvg.bear_zones();
            let bpr_zones: &[bpr::BprZone] = if self.bpr_scoring {
                self.bpr.zones()
            } else {
                &[]
            };
            self.scoring_bs.update(
                &out,
                bar,
                fvg_bull,
                fvg_bear,
                bpr_zones,
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
        // 21b. Rotation des événements de session N-1 (après le scoring qui
        //     les a consommés — la bar suivante verra ceux de cette bar).
        self.asian_hl_prec = out.asian_hl;
        self.london_hl_prec = out.london_hl;

        // 22. Lifecycle — évaluation intrabar (fill/SL/BE/TP/expire/BE-forcé).
        let (ob_bull_lc, ob_bear_lc) = {
            let b = self.order_blocks.bull_zones().to_vec();
            let r = self.order_blocks.bear_zones().to_vec();
            (b, r)
        };
        self.lifecycle.update(
            &mut self.signals.trades,
            &out,
            bar,
            bar_index,
            &self.calibration,
            &mut self.scoring_v11,
            &ob_bull_lc,
            &ob_bear_lc,
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
