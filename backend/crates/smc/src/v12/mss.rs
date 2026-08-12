//! MODULE 3 — MSS / CHOCH (Market Structure Shift / CHange Of Character).
//!
//! Reproduit MODULE 3 Pine (lignes 452-527) :
//!   - MSS = premier BOS **contre** la tendance dominante (alerte précoce, non confirmé).
//!   - CHOCH = MSS pending + swing confirmé dans le nouveau sens (HL bull / LH bear).
//!
//! Flags persistants `_mssHPending` / `_mssBPending` : un MSS déclenché reste en
//! attente d'une confirmation (HL ou LH) qui le transforme en CHOCH.
//!
//! Réinitialisation de la tendance au MSS (Pine lignes 504-506, 511-513) :
//! `bullCount := 0 ; bearCount := 0`. Le `MssDetector` mute le `StructureDetector`
//! via `reset_counts()`. La tendance utilisée pour la DÉTECTION du MSS est la
//! tendance **pré-reset** (Pine calcule `tendanceHaussiere` ligne 381 avant la
//! réinitialisation ligne 504) : on consomme donc le `StructureEvent` déjà calculé
//! pour cette bar.
//!
//! NOTE anti-doublon `dernierSH1_sig` : en Pine, MSS et BOS mettent tous deux
//! `dernierSH1_sig := bsh1` (lignes 507 et 524). Comme MSS ⇒ bosHaussier, le
//! `BosDetector` (qui fige `dernier_sh1_sig` sur tout BOS brut) couvre déjà ce cas ;
//! le `MssDetector` n'a donc pas besoin de toucher au BOS.

use super::pivots::PivotDetector;
use super::structure::StructureDetector;
use super::types::{BarInput, BosEvent, MssDir, MssEvent, StructureEvent};

/// Détecteur MSS / CHOCH avec flags pending persistants.
pub struct MssDetector {
    /// `_mssHPending` (Pine) — MSS haussier déclenché, en attente d'un HL.
    mss_h_pending: bool,
    /// `_mssBPending` (Pine) — MSS baissier déclenché, en attente d'un LH.
    mss_b_pending: bool,
    /// Index de bar courant (Pine `bar_index`).
    bar_count: usize,
    dernier_mss_level: Option<f64>,
    dernier_mss_bar: Option<usize>,
    dernier_choch_level: Option<f64>,
    dernier_choch_bar: Option<usize>,
    dernier_choch_dir: Option<MssDir>,
    last_event: MssEvent,
}

impl MssDetector {
    pub fn new() -> Self {
        Self {
            mss_h_pending: false,
            mss_b_pending: false,
            bar_count: 0,
            dernier_mss_level: None,
            dernier_mss_bar: None,
            dernier_choch_level: None,
            dernier_choch_bar: None,
            dernier_choch_dir: None,
            last_event: MssEvent::default(),
        }
    }

    /// Traite une bar. `bos` est le BOS BRUT (non masqué MSS). `structure` reçoit
    /// le `reset_counts()` si un MSS est détecté (effet sur les bars suivants).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        _bar: &BarInput,
        pivots: &PivotDetector,
        structure_event: &StructureEvent,
        bos: &BosEvent,
        structure: &mut StructureDetector,
    ) -> MssEvent {
        let cur_idx = self.bar_count;
        self.bar_count += 1;

        // --- MSS = BOS contre tendance dominante (Pine lignes 465-466) ---
        // On utilise la tendance PRÉ-reset (structure_event reflète l'état avant
        // la réinitialisation MSS, comme en Pine).
        let mss_haussier = structure_event.tendance_baissiere && bos.bullish;
        let mss_baissier = structure_event.tendance_haussiere && bos.bearish;

        // --- Flags pending (Pine lignes 472-477) ---
        if mss_haussier {
            self.mss_h_pending = true;
            self.mss_b_pending = false;
        }
        if mss_baissier {
            self.mss_b_pending = true;
            self.mss_h_pending = false;
        }

        // --- CHOCH = pending + swing confirmé, PAS sur la même bar que MSS (Pine 481-482) ---
        let choch_haussier = self.mss_h_pending && structure_event.is_hl && !mss_haussier;
        let choch_baissier = self.mss_b_pending && structure_event.is_lh && !mss_baissier;

        // --- Acquittement des flags pending au CHOCH (Pine lignes 484-487) ---
        if choch_haussier {
            self.mss_h_pending = false;
        }
        if choch_baissier {
            self.mss_b_pending = false;
        }

        // --- Réinitialisation tendance + niveaux au MSS (Pine lignes 504-515) ---
        let mut mss_level = None;
        let mut mss_bar = None;
        let mut mss_dir = None;
        if mss_haussier {
            structure.reset_counts();
            mss_level = pivots.sh1();
            mss_bar = Some(cur_idx);
            mss_dir = Some(MssDir::Haussier);
            self.dernier_mss_level = pivots.sh1();
            self.dernier_mss_bar = Some(cur_idx);
        }
        if mss_baissier {
            structure.reset_counts();
            mss_level = pivots.sl1();
            mss_bar = Some(cur_idx);
            mss_dir = Some(MssDir::Baissier);
            self.dernier_mss_level = pivots.sl1();
            self.dernier_mss_bar = Some(cur_idx);
        }

        // --- Niveaux/dir au CHOCH (Pine lignes 516-523) ---
        let mut choch_level = None;
        let mut choch_bar = None;
        let mut choch_dir = None;
        if choch_haussier {
            choch_level = pivots.sh1();
            choch_bar = Some(cur_idx);
            choch_dir = Some(MssDir::Haussier);
            self.dernier_choch_level = pivots.sh1();
            self.dernier_choch_bar = Some(cur_idx);
            self.dernier_choch_dir = Some(MssDir::Haussier);
        }
        if choch_baissier {
            choch_level = pivots.sl1();
            choch_bar = Some(cur_idx);
            choch_dir = Some(MssDir::Baissier);
            self.dernier_choch_level = pivots.sl1();
            self.dernier_choch_bar = Some(cur_idx);
            self.dernier_choch_dir = Some(MssDir::Baissier);
        }

        let ev = MssEvent {
            mss_haussier,
            mss_baissier,
            choch_haussier,
            choch_baissier,
            mss_level,
            mss_bar,
            mss_dir,
            choch_level,
            choch_bar,
            choch_dir,
            mss_h_pending: self.mss_h_pending,
            mss_b_pending: self.mss_b_pending,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> MssEvent {
        self.last_event.clone()
    }
    pub fn mss_h_pending(&self) -> bool {
        self.mss_h_pending
    }
    pub fn mss_b_pending(&self) -> bool {
        self.mss_b_pending
    }
    pub fn dernier_mss_level(&self) -> Option<f64> {
        self.dernier_mss_level
    }
    pub fn dernier_choch_level(&self) -> Option<f64> {
        self.dernier_choch_level
    }
}

impl Default for MssDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: usize) -> BarInput {
        BarInput {
            timestamp: i as i64,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 0.0,
        }
    }

    /// Construit un StructureEvent avec tendance donnée (pré-reset).
    fn struct_event(haussiere: bool, baissiere: bool, is_hl: bool, is_lh: bool) -> StructureEvent {
        StructureEvent {
            is_hl,
            is_lh,
            is_hh: false,
            is_ll: false,
            bull_count: if haussiere { 2 } else { 0 },
            bear_count: if baissiere { 2 } else { 0 },
            tendance_haussiere: haussiere,
            tendance_baissiere: baissiere,
        }
    }

    /// Stubs pivots : sh1/sl1 connus via setters externes impossibles → on teste
    /// la logique MSS/CHOCH via un PivotDetector réel minimal (sl=3) pour avoir sh1.
    fn build_pivots_with_sh1() -> (PivotDetector, usize) {
        // sh1=110 à l'index 3 (sl=3, 7 bars). bsh1 = 3.
        let mut piv = PivotDetector::new(3);
        for i in 0..7usize {
            let h = if i == 3 { 110.0 } else { 100.0 };
            let b = BarInput {
                timestamp: i as i64,
                open: 100.0,
                high: h,
                low: 90.0,
                close: 100.0,
                volume: 0.0,
            };
            piv.update(&b);
        }
        (piv, 3)
    }

    #[test]
    fn mss_haussier_bos_contre_tendance_baissiere_reset_counts() {
        let (piv, _) = build_pivots_with_sh1();
        assert_eq!(piv.sh1(), Some(110.0));
        let mut st = StructureDetector::new();
        let mut mss = MssDetector::new();

        // BOS haussier en tendance baissière ⇒ MSS haussier.
        let ev = mss.update(
            &bar(0),
            &piv,
            &struct_event(false, true, false, false),
            &BosEvent {
                bullish: true,
                bearish: false,
                level: Some(110.0),
                bar_index: Some(0),
            },
            &mut st,
        );
        assert!(ev.mss_haussier, "BOS haussier contre tendance baissière ⇒ MSS");
        assert!(ev.mss_h_pending, "flag pending armé");
        assert_eq!(ev.mss_level, Some(110.0));
        let (b, r) = st.counts();
        assert_eq!((b, r), (0, 0), "MSS réinitialise les compteurs à 0");
    }

    #[test]
    fn mss_ne_déclenche_pas_si_bos_dans_le_sens_de_la_tendance() {
        let (piv, _) = build_pivots_with_sh1();
        let mut st = StructureDetector::new();
        let mut mss = MssDetector::new();
        let ev = mss.update(
            &bar(0),
            &piv,
            &struct_event(true, false, false, false), // tendance haussière
            &BosEvent {
                bullish: true,
                ..Default::default()
            },
            &mut st,
        );
        assert!(!ev.mss_haussier, "BOS dans le sens de la tendance ⇒ pas de MSS");
    }

    #[test]
    fn choch_haussier_confirmé_par_hl_apres_mss() {
        let (piv, _) = build_pivots_with_sh1();
        let mut st = StructureDetector::new();
        let mut mss = MssDetector::new();

        // Bar 0 : MSS haussier (BOS haussier contre tendance baissière).
        let _ = mss.update(
            &bar(0),
            &piv,
            &struct_event(false, true, false, false),
            &BosEvent {
                bullish: true,
                level: Some(110.0),
                bar_index: Some(0),
                ..Default::default()
            },
            &mut st,
        );
        assert!(mss.mss_h_pending());

        // Bar 1 : HL apparaît (isHL=true), pas de MSS cette bar ⇒ CHOCH haussier.
        let ev = mss.update(
            &bar(1),
            &piv,
            &struct_event(false, false, true, false),
            &BosEvent::default(),
            &mut st,
        );
        assert!(ev.choch_haussier, "HL après MSS pending ⇒ CHOCH haussier");
        assert!(!mss.mss_h_pending(), "CHOCH acquitte le flag pending");
        assert_eq!(ev.choch_level, Some(110.0));
    }

    #[test]
    fn choch_baissier_ne_déclenche_pas_simultanément_avec_mss() {
        let (piv, _) = build_pivots_with_sh1();
        let mut st = StructureDetector::new();
        let mut mss = MssDetector::new();
        // Bar 0 : MSS baissier + LH présent même bar ⇒ CHOCH bloqué (not mssBaissier).
        let ev = mss.update(
            &bar(0),
            &piv,
            &struct_event(true, false, false, true), // LH true
            &BosEvent {
                bearish: true,
                level: Some(90.0),
                bar_index: Some(0),
                ..Default::default()
            },
            &mut st,
        );
        assert!(ev.mss_baissier);
        assert!(
            !ev.choch_baissier,
            "CHOCH bloqué sur la même bar que le MSS (not mssBaissier)"
        );
    }

    #[test]
    fn nouveau_mss_inverse_rearme_le_bon_flag() {
        let (piv, _) = build_pivots_with_sh1();
        let mut st = StructureDetector::new();
        let mut mss = MssDetector::new();
        // MSS haussier.
        let _ = mss.update(
            &bar(0),
            &piv,
            &struct_event(false, true, false, false),
            &BosEvent {
                bullish: true,
                ..Default::default()
            },
            &mut st,
        );
        assert!(mss.mss_h_pending() && !mss.mss_b_pending());
        // MSS baissier ensuite ⇒ bascule du flag.
        let ev = mss.update(
            &bar(1),
            &piv,
            &struct_event(true, false, false, false),
            &BosEvent {
                bearish: true,
                ..Default::default()
            },
            &mut st,
        );
        assert!(ev.mss_baissier);
        assert!(mss.mss_b_pending() && !mss.mss_h_pending());
    }

    #[test]
    fn pas_de_mss_sans_bos() {
        let (piv, _) = build_pivots_with_sh1();
        let mut st = StructureDetector::new();
        let mut mss = MssDetector::new();
        let ev = mss.update(
            &bar(0),
            &piv,
            &struct_event(false, true, false, false),
            &BosEvent::default(),
            &mut st,
        );
        assert!(!ev.mss_haussier && !ev.mss_baissier && !ev.choch_haussier);
    }
}
