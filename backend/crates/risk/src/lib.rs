use common::{Asset, Result, Signal, TradingError};
use std::collections::HashMap;

/// Limites de risque non-négociables (voir .clinerules règle 1)
const MAX_RISK_PAR_TRADE: f64 = 0.02; // 2% capital max
const MAX_POSITIONS_SIMULTANEES: usize = 3;
const MAX_EXPOSITION_ACTIF: f64 = 0.25; // 25% capital par asset
const MAX_DRAWDOWN: f64 = 20.0; // % — arrêt automatique

pub struct GestionnaireRisque {
    capital: f64,
    drawdown_courant: f64,
    positions_ouvertes: usize,
    /// Valeur notionnelle engagée par asset (en unités monétaires)
    exposition_par_actif: HashMap<Asset, f64>,
}

impl GestionnaireRisque {
    pub fn new(capital: f64) -> Self {
        Self {
            capital,
            drawdown_courant: 0.0,
            positions_ouvertes: 0,
            exposition_par_actif: HashMap::new(),
        }
    }

    /// Valide un signal avant ouverture de position
    /// Renvoie Ok(true) si le trade est autorisé, Ok(false) si refusé par une règle de risque
    pub fn valider_signal(&self, signal: &Signal) -> Result<bool> {
        // 1. Drawdown maximum atteint → arrêt trading
        if self.drawdown_courant >= MAX_DRAWDOWN {
            tracing::warn!(
                "Signal refusé: drawdown {}% ≥ seuil {}%",
                self.drawdown_courant,
                MAX_DRAWDOWN
            );
            return Ok(false);
        }

        // 2. Exposition max par actif — limite à 25% du capital
        let taille = self.calculer_taille_position(signal.prix_entree, signal.stop_loss);
        let valeur_notionnelle = taille * signal.prix_entree;
        let expo_existante = self
            .exposition_par_actif
            .get(&signal.asset)
            .copied()
            .unwrap_or(0.0);
        let expo_totale_actif = expo_existante + valeur_notionnelle;
        if expo_totale_actif > self.capital * MAX_EXPOSITION_ACTIF {
            tracing::warn!(
                "Signal refusé: exposition {:?} {:.2} > max {:.2} ({:.0}% capital)",
                signal.asset,
                expo_totale_actif,
                self.capital * MAX_EXPOSITION_ACTIF,
                MAX_EXPOSITION_ACTIF * 100.0
            );
            return Ok(false);
        }

        // 3. Nombre max de positions simultanées
        if self.positions_ouvertes >= MAX_POSITIONS_SIMULTANEES {
            tracing::warn!(
                "Signal refusé: {} positions ouvertes (max {})",
                self.positions_ouvertes,
                MAX_POSITIONS_SIMULTANEES
            );
            return Ok(false);
        }

        // 4. Taille de position ≤ 2% capital
        let distance_stop = (signal.prix_entree - signal.stop_loss).abs();
        if distance_stop <= 0.0 {
            return Err(TradingError::Risk("Distance stop invalide (≤ 0)".into()));
        }
        let risque_pct = (taille * distance_stop) / self.capital;
        if risque_pct > MAX_RISK_PAR_TRADE {
            tracing::warn!(
                "Signal refusé: risque {:.2}% > max {:.0}%",
                risque_pct * 100.0,
                MAX_RISK_PAR_TRADE * 100.0
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Calcule la taille de position en unités d'asset (risque fixé à 1% capital)
    pub fn calculer_taille_position(&self, prix_entree: f64, stop_loss: f64) -> f64 {
        let distance_stop = (prix_entree - stop_loss).abs();
        if distance_stop == 0.0 || prix_entree == 0.0 {
            return 0.0;
        }
        let risque_capital = self.capital * 0.01; // 1% par défaut
        risque_capital / distance_stop
    }

    // ── Formule pips-aware (Phase 2) ─────────────────────────────────────────────

    /// Calcule la taille en lots à partir des paramètres de l'asset.
    /// Formule : Lot = (Capital × risque_pct/100) / (sl_pips × valeur_pips)
    /// Le résultat est clampé entre lot_min et lot_max.
    pub fn calculer_lot_asset(
        capital: f64,
        risque_pct: f64,
        sl_pips: f64,
        valeur_pips: f64,
        lot_min: f64,
        lot_max: f64,
    ) -> Result<f64> {
        if sl_pips <= 0.0 || valeur_pips <= 0.0 {
            return Err(TradingError::Risk(
                "sl_pips et valeur_pips doivent être > 0".into(),
            ));
        }
        if risque_pct <= 0.0 || risque_pct > 100.0 {
            return Err(TradingError::Risk(
                "risque_pct doit être entre 0 et 100".into(),
            ));
        }
        let investi = capital * (risque_pct / 100.0);
        let lot = investi / (sl_pips * valeur_pips);
        Ok(lot.clamp(lot_min, lot_max))
    }

    pub fn mettre_a_jour_drawdown(&mut self, drawdown_pct: f64) {
        self.drawdown_courant = drawdown_pct;
    }

    pub fn incrementer_positions(&mut self) {
        self.positions_ouvertes += 1;
    }

    pub fn decrementer_positions(&mut self) {
        self.positions_ouvertes = self.positions_ouvertes.saturating_sub(1);
    }

    /// Enregistre l'ouverture d'une position (met à jour l'exposition par actif)
    pub fn ouvrir_position(&mut self, asset: &Asset, valeur_notionnelle: f64) {
        let expo = self
            .exposition_par_actif
            .entry(asset.clone())
            .or_insert(0.0);
        *expo += valeur_notionnelle;
        self.positions_ouvertes += 1;
    }

    /// Enregistre la fermeture d'une position (réduit l'exposition par actif)
    pub fn fermer_position(&mut self, asset: &Asset, valeur_notionnelle: f64) {
        if let Some(expo) = self.exposition_par_actif.get_mut(asset) {
            *expo = (*expo - valeur_notionnelle).max(0.0);
        }
        self.positions_ouvertes = self.positions_ouvertes.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::{Asset, Direction, Timeframe};
    use uuid::Uuid;

    fn signal_test(prix: f64, sl: f64, tp: Vec<f64>) -> Signal {
        Signal {
            id: Uuid::new_v4(),
            asset: Asset::BTC,
            timeframe: Timeframe::M15,
            direction: Direction::Long,
            score: 80.0,
            prix_entree: prix,
            stop_loss: sl,
            take_profit: tp,
            strategie: "test".into(),
            cree_le: Utc::now(),
        }
    }

    #[test]
    fn signal_valide_accepte() {
        let risque = GestionnaireRisque::new(10_000.0);
        // prix=1000, sl=900 → distance=100, taille=100/100=1 unité, notionnel=1*1000=1000€ < 25%*10000=2500
        let signal = signal_test(1_000.0, 900.0, vec![1_100.0]);
        assert!(risque.valider_signal(&signal).unwrap());
    }

    #[test]
    fn drawdown_max_refuse() {
        let mut risque = GestionnaireRisque::new(10_000.0);
        risque.mettre_a_jour_drawdown(20.0);
        let signal = signal_test(50_000.0, 49_000.0, vec![52_000.0]);
        assert!(!risque.valider_signal(&signal).unwrap());
    }

    #[test]
    fn positions_max_refuse() {
        let mut risque = GestionnaireRisque::new(10_000.0);
        // Prix bas pour que la valeur notionnelle reste sous le seuil d'exposition
        risque.ouvrir_position(&Asset::BTC, 50.0);
        risque.ouvrir_position(&Asset::BTC, 50.0);
        risque.ouvrir_position(&Asset::BTC, 50.0);
        let signal = signal_test(50_000.0, 49_000.0, vec![52_000.0]);
        assert!(!risque.valider_signal(&signal).unwrap());
    }

    #[test]
    fn exposition_actif_max_refuse() {
        let capital = 10_000.0;
        let mut risque = GestionnaireRisque::new(capital);
        // On enregistre 2 500 € d'exposition BTC (= 25% capital)
        risque.ouvrir_position(&Asset::BTC, capital * MAX_EXPOSITION_ACTIF);
        // Le signal suivant dépasse la limite
        let signal = signal_test(50_000.0, 49_000.0, vec![52_000.0]);
        assert!(!risque.valider_signal(&signal).unwrap());
    }

    #[test]
    fn taille_position_correcte() {
        let risque = GestionnaireRisque::new(10_000.0);
        // 1% de 10 000 = 100€ risqués, distance = 1000 → taille = 0.1 unité
        let taille = risque.calculer_taille_position(50_000.0, 49_000.0);
        assert!((taille - 0.1).abs() < 1e-10);
    }
}
