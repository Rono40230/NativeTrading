use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

/// Détecte les croisements MACD/Signal et passages à zéro de la ligne MACD
pub fn detecter_signaux_macd(
    timestamps: &[i64],
    macd_ligne: &[f64],
    macd_signal: &[f64],
    macd_histo: &[f64],
) -> Vec<SignalIndicateur> {
    let n = timestamps
        .len()
        .min(macd_ligne.len())
        .min(macd_signal.len())
        .min(macd_histo.len());
    let mut signaux = Vec::new();

    for i in 1..n {
        let ml_p = macd_ligne[i - 1];
        let ml_c = macd_ligne[i];
        let ms_p = macd_signal[i - 1];
        let ms_c = macd_signal[i];
        if ml_p.is_nan() || ml_c.is_nan() || ms_p.is_nan() || ms_c.is_nan() {
            continue;
        }
        // Croisement haussier MACD / Signal (Moyen)
        if ml_p <= ms_p && ml_c > ms_c {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "MACD".into(),
                type_signal: "croisement_haussier".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Moyen,
                description: format!(
                    "MACD croise signal à la hausse (histo={:.5})",
                    macd_histo[i]
                ),
                valeur: ml_c,
                prix_entree: 0.0,
            });
        }
        // Croisement baissier MACD / Signal (Moyen)
        if ml_p >= ms_p && ml_c < ms_c {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "MACD".into(),
                type_signal: "croisement_baissier".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Moyen,
                description: format!(
                    "MACD croise signal à la baisse (histo={:.5})",
                    macd_histo[i]
                ),
                valeur: ml_c,
                prix_entree: 0.0,
            });
        }
        // Passage à zéro haussier (Faible)
        if ml_p <= 0.0 && ml_c > 0.0 {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "MACD".into(),
                type_signal: "zero_haussier".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Faible,
                description: format!("MACD passe au-dessus de zéro ({:.5})", ml_c),
                valeur: ml_c,
                prix_entree: 0.0,
            });
        }
        // Passage à zéro baissier (Faible)
        if ml_p >= 0.0 && ml_c < 0.0 {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "MACD".into(),
                type_signal: "zero_baissier".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Faible,
                description: format!("MACD passe en-dessous de zéro ({:.5})", ml_c),
                valeur: ml_c,
                prix_entree: 0.0,
            });
        }
    }
    signaux
}
