//! Adaptateur SMC du hook structurel du lifecycle commun (`gestion_trades`).
//! Extrait de mod.rs — limite de 600 lignes par fichier (pre-commit).

/// BOS/MSS opposé lus sur la sortie courante du moteur, un-signal via le
/// scoring (règle de l'un-signal — SMC seule : le straddle passe HookVide).
pub struct HookSmc<'a, 'b, 'c> {
    pub out: &'a crate::v12::types::SmcOutput,
    pub scoring: &'b mut crate::v12::scoring_v11::ScoringV11,
    pub ob_bull: &'c [crate::v12::types::ObZone],
    pub ob_bear: &'c [crate::v12::types::ObZone],
}

impl gestion_trades::HookStructure for HookSmc<'_, '_, '_> {
    fn bos_oppose(&self, is_buy: bool) -> bool {
        if is_buy { self.out.bos_raw.bearish } else { self.out.bos_raw.bullish }
    }
    fn mss_oppose(&self, is_buy: bool) -> bool {
        if is_buy { self.out.mss.mss_baissier } else { self.out.mss.mss_haussier }
    }
    fn sur_be_force(&mut self, is_buy: bool) {
        let zones = if is_buy { self.ob_bull } else { self.ob_bear };
        self.scoring.unmark_premier_signale(is_buy, zones);
    }
}
