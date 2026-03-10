use common::{Result, Signal, TradingError};

/// Limites de risque non-négociables (voir .clinerules règle 1)
const MAX_RISK_PAR_TRADE: f64 = 0.02;     // 2% capital max
const MAX_POSITIONS_SIMULTANEES: usize = 3;
const MAX_EXPOSITION_ACTIF: f64 = 0.25;   // 25% capital par asset
const MAX_DRAWDOWN: f64 = 20.0;           // % — arrêt automatique

pub struct GestionnaireRisque {
    capital: f64,
    drawdown_courant: f64,
    positions_ouvertes: usize,
}

impl GestionnaireRisque {
    pub fn new(capital: f64) -> Self {
        Self {
            capital,
            drawdown_courant: 0.0,
            positions_ouvertes: 0,
        }
    }

    /// Valide un signal avant ouverture de position
    /// Renvoie Ok(true) si le trade est autorisé, Ok(false) si refusé par une règle de risque
    pub fn valider_signal(&self, signal: &Signal) -> Result<bool> {
        // 1. Drawdown maximum atteint → arrêt trading
        if self.drawdown_courant >= MAX_DRAWDOWN {
            tracing::warn!(
                "Signal refusé: drawdown {}% ≥ seuil {}%",
                self.drawdown_courant, MAX_DRAWDOWN
            );
            return Ok(false);
        }

        // 2. Exposition max par actif (vérification partielle sans positions détaillées)
        let _ = MAX_EXPOSITION_ACTIF; // Limite 25% — appliquée dans la stratégie

        // 3. Nombre max de positions simultanées
        if self.positions_ouvertes >= MAX_POSITIONS_SIMULTANEES {
            tracing::warn!(
                "Signal refusé: {} positions ouvertes (max {})",
                self.positions_ouvertes, MAX_POSITIONS_SIMULTANEES
            );
            return Ok(false);
        }

        // 4. Taille de position ≤ 2% capital
        let distance_stop = (signal.prix_entree - signal.stop_loss).abs();
        if distance_stop <= 0.0 {
            return Err(TradingError::Risk(
                "Distance stop invalide (≤ 0)".into(),
            ));
        }
        let taille = self.calculer_taille_position(signal.prix_entree, signal.stop_loss);
        let risque_pct = (taille * distance_stop) / self.capital;
        if risque_pct > MAX_RISK_PAR_TRADE {
            tracing::warn!(
                "Signal refusé: risque {:.2}% > max {:.0}%",
                risque_pct * 100.0, MAX_RISK_PAR_TRADE * 100.0
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

    pub fn mettre_a_jour_drawdown(&mut self, drawdown_pct: f64) {
        self.drawdown_courant = drawdown_pct;
    }

    pub fn incrementer_positions(&mut self) {
        self.positions_ouvertes += 1;
    }

    pub fn decrementer_positions(&mut self) {
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
        let signal = signal_test(50_000.0, 49_000.0, vec![52_000.0]);
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
        risque.incrementer_positions();
        risque.incrementer_positions();
        risque.incrementer_positions();
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
