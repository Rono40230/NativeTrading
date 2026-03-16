use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

/// Détecte les croisements prix/EMA : golden cross (haussier) et death cross (baissier)
pub fn detecter_signaux_ema(
    timestamps: &[i64],
    closes: &[f64],
    ema: &[f64],
) -> Vec<SignalIndicateur> {
    let n = timestamps.len().min(closes.len()).min(ema.len());
    let mut signaux = Vec::new();

    for i in 1..n {
        let ema_prev = ema[i - 1];
        let ema_curr = ema[i];
        if ema_prev.is_nan() || ema_curr.is_nan() {
            continue;
        }
        let prix_passe_dessus = closes[i - 1] <= ema_prev && closes[i] > ema_curr;
        let prix_passe_dessous = closes[i - 1] >= ema_prev && closes[i] < ema_curr;

        if prix_passe_dessus {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "EMA".into(),
                type_signal: "golden_cross".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Moyen,
                description: format!("Prix croise l'EMA à la hausse ({:.5})", ema_curr),
                valeur: ema_curr,
                prix_entree: 0.0,
            });
        } else if prix_passe_dessous {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "EMA".into(),
                type_signal: "death_cross".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Moyen,
                description: format!("Prix croise l'EMA à la baisse ({:.5})", ema_curr),
                valeur: ema_curr,
                prix_entree: 0.0,
            });
        }
    }
    signaux
}
