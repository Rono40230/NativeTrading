use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

// Seuils de détection par rapport à la moyenne mobile de l'ATR
const SEUIL_SPIKE: f64 = 1.2; // ATR > 120% de sa moyenne → volatilité anormale
const SEUIL_COMPRESSION: f64 = 0.75; // ATR < 75% de sa moyenne → breakout imminent

/// Détecte les signaux ATR : spike (volatilité excessive) et compression (breakout imminent)
pub fn detecter_signaux_atr(
    timestamps: &[i64],
    closes: &[f64],
    atr: &[f64],
    periode_moyenne: usize,
) -> Vec<SignalIndicateur> {
    let n = timestamps.len().min(closes.len()).min(atr.len());
    if n <= periode_moyenne {
        return vec![];
    }
    let mut signaux = Vec::new();

    for i in periode_moyenne..n {
        if !atr[i].is_finite() || !closes[i].is_finite() {
            continue;
        }
        let debut = i.saturating_sub(periode_moyenne);
        let vals: Vec<f64> = atr[debut..i]
            .iter()
            .filter(|v| v.is_finite())
            .copied()
            .collect();
        if vals.is_empty() {
            continue;
        }
        let moyenne = vals.iter().sum::<f64>() / vals.len() as f64;
        if moyenne <= 0.0 {
            continue;
        }
        let ratio = atr[i] / moyenne;

        if ratio >= SEUIL_SPIKE {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "ATR".into(),
                type_signal: "atr_spike".into(),
                direction: DirectionSignal::Neutre,
                force: NiveauForce::Faible,
                description: format!(
                    "Volatilité ×{:.1} vs moyenne — news/manipulation possible",
                    ratio
                ),
                valeur: atr[i],
                prix_entree: closes[i],
            });
        } else if ratio <= SEUIL_COMPRESSION {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "ATR".into(),
                type_signal: "atr_compression".into(),
                direction: DirectionSignal::Neutre,
                force: NiveauForce::Moyen,
                description: format!(
                    "Compression ATR ({:.1}× moy) — breakout directionnel imminent",
                    ratio
                ),
                valeur: atr[i],
                prix_entree: closes[i],
            });
        }
    }
    signaux
}
