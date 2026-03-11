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
            confidence: ratio_atr.min(3.0) / 3.0, // normalisé 0-1
            entry_price: prix_entree,
            stop_loss: sl,
            take_profit: tp,
            take_profit_2: None,
            take_profit_3: None,
        }))
    }
}
