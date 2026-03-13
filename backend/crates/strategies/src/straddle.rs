use super::{Signal, Strategy};
use common::{Candle, Direction, Result};
use indicators::calculer_atr;
use ml::PipelineML;

/// Seuil ATR : >150% de sa moyenne sur 14 périodes = volatilité extrême
const SEUIL_ATR_RATIO: f64 = 1.5;
/// Multiplicateur ATR pour TP et SL
const MULTIPLICATEUR_TP: f64 = 2.0;
const MULTIPLICATEUR_SL: f64 = 0.5;

/// Stratégie Straddle — volatilité extrême + IA indécise
///
/// Déclencheur : ATR > 150% de sa moyenne ET confiance ML < 60%
/// Exécution : positions opposées simultanées (LONG + SHORT)
/// TP : ATR × 2 | SL : ATR × 0.5
/// Risk : 1% par direction (2% total)
pub struct StraddleStrategy {
    pub pipeline_ml: Option<PipelineML>,
}

impl StraddleStrategy {
    pub fn new() -> Self {
        Self { pipeline_ml: None }
    }

    pub fn avec_ml(pipeline: PipelineML) -> Self {
        Self {
            pipeline_ml: Some(pipeline),
        }
    }
}

impl Default for StraddleStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for StraddleStrategy {
    fn analyze(&self, bougies: &[Candle]) -> Result<Option<Signal>> {
        if bougies.len() < 30 {
            return Ok(None);
        }

        let atr = calculer_atr(bougies, 14);
        let n = bougies.len();
        let atr_courant = atr[n - 1];

        if atr_courant.is_nan() {
            return Ok(None);
        }

        // Moyenne ATR sur les 14 dernières valeurs non-NaN
        let atr_valides: Vec<f64> = atr[n.saturating_sub(14)..n]
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();

        if atr_valides.is_empty() {
            return Ok(None);
        }
        let atr_moyen = atr_valides.iter().sum::<f64>() / atr_valides.len() as f64;
        let ratio_atr = atr_courant / atr_moyen.max(1e-10);

        // Condition 1 : volatilité extrême (ATR > 150% de sa moyenne)
        if ratio_atr <= SEUIL_ATR_RATIO {
            return Ok(None);
        }

        // Condition 2 : IA indécise (si modèle disponible)
        let ia_indecise = match &self.pipeline_ml {
            Some(pipeline) if pipeline.est_pret() => {
                match pipeline.predire(bougies) {
                    Ok(pred) => !pred.est_confiant, // confiance < 60% = indécis
                    Err(e) => {
                        tracing::warn!("Straddle: erreur ML (considéré indécis): {}", e);
                        true
                    }
                }
            }
            _ => true, // Pas de modèle = considéré indécis par défaut
        };

        if !ia_indecise {
            return Ok(None);
        }

        let prix_entree = bougies[n - 1].close;
        let tp = prix_entree + atr_courant * MULTIPLICATEUR_TP;
        let sl = prix_entree - atr_courant * MULTIPLICATEUR_SL;

        tracing::info!(
            "Signal STRADDLE: prix={:.2} ATR={:.4} ratio={:.2}x TP={:.2} SL={:.2}",
            prix_entree,
            atr_courant,
            ratio_atr,
            tp,
            sl
        );

        Ok(Some(Signal {
            direction: Direction::Both,
            confiance: ratio_atr.min(3.0) / 3.0, // normalisé 0-1
            prix_entree,
            stop_loss: sl,
            take_profit: tp,
            take_profit_2: None,
            take_profit_3: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

    fn bougie_plate(c: f64) -> Candle {
        Candle { timestamp: Utc::now(), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: 1000.0 }
    }

    fn bougie_volatile(c: f64, range: f64) -> Candle {
        Candle { timestamp: Utc::now(), open: c, high: c + range, low: c - range, close: c, volume: 5000.0 }
    }

    #[test]
    fn analyse_retourne_none_si_moins_de_30_bougies() {
        let strat = StraddleStrategy::new();
        let bougies: Vec<Candle> = (1..=25).map(|i| bougie_plate(i as f64 * 100.0)).collect();
        assert!(strat.analyze(&bougies).unwrap().is_none());
    }

    #[test]
    fn analyse_retourne_none_si_atr_plat() {
        let strat = StraddleStrategy::new();
        // Toutes les bougies identiques → ATR ratio = 1.0 ≤ seuil 1.5
        let bougies: Vec<Candle> = (0..35).map(|_| bougie_plate(100.0)).collect();
        assert!(strat.analyze(&bougies).unwrap().is_none());
    }

    #[test]
    fn analyse_retourne_signal_straddle_si_volatilite_extreme() {
        let strat = StraddleStrategy::new();
        // 34 bougies plates, dernière avec range gigantesque → ratio ATR >> 1.5
        let mut bougies: Vec<Candle> = (0..34).map(|_| bougie_plate(100.0)).collect();
        bougies.push(bougie_volatile(100.0, 150.0));
        let signal = strat.analyze(&bougies).unwrap();
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert_eq!(s.direction, Direction::Both);
        assert!(s.prix_entree > 0.0);
        assert!(s.take_profit > s.prix_entree);
    }
}
