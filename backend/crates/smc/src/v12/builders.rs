//! Builders d'étude & réglages de `SmcV12Engine` — extraits de `mod.rs`
//! (limite 600 lignes) : les méthodes `avec_*` de configuration du scoring
//! et des niveaux. Le cœur (update, primer MTF, lifecycle) reste dans mod.rs.

use super::signals;
use super::SmcV12Engine;

impl SmcV12Engine {
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

    /// R2 (étude étape 3) : porte P/D directionnel (« jamais acheter en
    /// premium, vendre en discount ») en qualification v11.
    pub fn avec_pd_requis(mut self, actif: bool) -> Self {
        self.signals.definir_pd_requis(actif);
        self
    }

    /// R4 (étude étape 3) : confluences MTF évaluées sur HTF CLÔTURÉ seul
    /// (défaut = live, parité Pine — mesure du coût du repaint).
    pub fn avec_mtf_cloture(mut self, actif: bool) -> Self {
        self.mtf = self.mtf.clone().avec_cloture_seulement(actif);
        self
    }

    /// R5 (étude étape 3) : confluences MTF à containment DIRECTIONNEL
    /// (close dans une zone du sens du trade, vs flag agnostique + existence).
    pub fn avec_mtf_directionnel(mut self, actif: bool) -> Self {
        self.mtf_directionnel = actif;
        self
    }

    /// Étude étape 4 — multiplicateurs de niveaux : SL (offset ATR),
    /// TP1 (×r), TP2 (×r). Défauts = production (1.0 / 1.0 / 2.0).
    pub fn avec_multiplicateurs(mut self, sl: f64, tp1: f64, tp2: f64) -> Self {
        self.signals.definir_multiplicateurs(sl, tp1, tp2);
        self
    }

    /// TP1 réglable (Paramètres › SMC) — SL et TP2 inchangés.
    pub fn avec_tp1(mut self, tp1: f64) -> Self {
        self.signals.definir_tp1(tp1);
        self
    }

    /// TP2 réglable (Paramètres › SMC, défaut 2.0) — SL et TP1 inchangés.
    pub fn avec_tp2(mut self, tp2: f64) -> Self {
        self.signals.definir_tp2(tp2);
        self
    }

    /// TP3 réglable propriétaire (liquidité lointaine / R fixe + repli).
    pub fn avec_tp3_reglage(mut self, reglage: signals::Tp3Reglage) -> Self {
        self.signals.definir_tp3_reglage(reglage);
        self
    }

    /// Trailing stop après TP2 : Some(k) = actif à k×R de l'extrême post-TP2.
    /// Surcharge SL max PAR ASSET (0121) : multiple d'ATR remplaçant la
    /// classe — None = étalon Pine.
    pub fn avec_surcharge_sl_max(mut self, mult: Option<f64>) -> Self {
        self.calibration.sl_max_surcharge = mult;
        self
    }

    pub fn avec_trailing_tp2(mut self, k: Option<f64>) -> Self {
        self.lifecycle.definir_trailing_tp2(k);
        self
    }

    /// Étude étape 4 — BE automatique à seuil de MFE (None = production).
    pub fn avec_be_auto(mut self, seuil: Option<f64>) -> Self {
        self.lifecycle.definir_be_auto(seuil);
        self
    }
}
