use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

/// Détecte les signaux RSI : sorties de zones extrêmes et croisements de la ligne 50
pub fn detecter_signaux_rsi(
    timestamps: &[i64],
    rsi: &[f64],
    surachat: f64,
    survente: f64,
) -> Vec<SignalIndicateur> {
    let n = timestamps.len().min(rsi.len());
    let mut signaux = Vec::new();

    for i in 1..n {
        let prev = rsi[i - 1];
        let curr = rsi[i];
        if prev.is_nan() || curr.is_nan() {
            continue;
        }
        // Sortie de survente → signal bullish (Moyen)
        if prev < survente && curr >= survente {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "RSI".into(),
                type_signal: "survente_sortie".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Moyen,
                description: format!("RSI sort de survente ({:.1} → {:.1})", prev, curr),
                valeur: curr,
                prix_entree: 0.0,
            });
        }
        // Sortie de surachat → signal bearish (Moyen)
        if prev > surachat && curr <= surachat {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "RSI".into(),
                type_signal: "surachat_sortie".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Moyen,
                description: format!("RSI sort de surachat ({:.1} → {:.1})", prev, curr),
                valeur: curr,
                prix_entree: 0.0,
            });
        }
        // Croisement ligne 50 haussier (Faible)
        if prev < 50.0 && curr >= 50.0 {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "RSI".into(),
                type_signal: "mi_ligne_haussiere".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Faible,
                description: format!("RSI croise 50 à la hausse ({:.1})", curr),
                valeur: curr,
                prix_entree: 0.0,
            });
        }
        // Croisement ligne 50 baissier (Faible)
        if prev > 50.0 && curr <= 50.0 {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "RSI".into(),
                type_signal: "mi_ligne_baissiere".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Faible,
                description: format!("RSI croise 50 à la baisse ({:.1})", curr),
                valeur: curr,
                prix_entree: 0.0,
            });
        }
    }
    signaux
}
