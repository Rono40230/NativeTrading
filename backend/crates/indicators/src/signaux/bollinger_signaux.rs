use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

/// Détecte les signaux Bollinger : touches et cassures de bandes, squeeze
pub fn detecter_signaux_bollinger(
    timestamps: &[i64],
    closes: &[f64],
    haute: &[f64],
    milieu: &[f64],
    basse: &[f64],
    periode_squeeze: usize,
) -> Vec<SignalIndicateur> {
    let n = timestamps
        .len()
        .min(closes.len())
        .min(haute.len())
        .min(milieu.len())
        .min(basse.len());
    let mut signaux = Vec::new();
    let lookback = periode_squeeze.max(5);

    for i in 1..n {
        let h = haute[i];
        let m = milieu[i];
        let b = basse[i];
        let c = closes[i];
        let c_prev = closes[i - 1];
        if h.is_nan() || m.is_nan() || b.is_nan() || m == 0.0 {
            continue;
        }
        let h_prev = haute[i - 1];
        let b_prev = basse[i - 1];
        let prev_inside = !b_prev.is_nan()
            && !h_prev.is_nan()
            && c_prev > b_prev
            && c_prev < h_prev;

        // Touche bande basse — retournement bullish potentiel (Faible)
        if c <= b && (b_prev.is_nan() || c_prev > b_prev) {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "Bollinger".into(),
                type_signal: "touche_bande_basse".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Faible,
                description: format!("Prix touche la bande basse ({:.5})", b),
                valeur: b,
                prix_entree: 0.0,
            });
        }
        // Touche bande haute — retournement bearish potentiel (Faible)
        if c >= h && (h_prev.is_nan() || c_prev < h_prev) {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "Bollinger".into(),
                type_signal: "touche_bande_haute".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Faible,
                description: format!("Prix touche la bande haute ({:.5})", h),
                valeur: h,
                prix_entree: 0.0,
            });
        }
        // Clôture sous la bande basse — breakout baissier fort (Moyen)
        if c < b && prev_inside {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "Bollinger".into(),
                type_signal: "cassure_basse".into(),
                direction: DirectionSignal::Bearish,
                force: NiveauForce::Moyen,
                description: format!("Clôture sous la bande basse — breakout baissier ({:.5})", b),
                valeur: b,
                prix_entree: 0.0,
            });
        }
        // Clôture au-dessus de la bande haute — breakout haussier (Moyen)
        if c > h && prev_inside {
            signaux.push(SignalIndicateur {
                timestamp: timestamps[i],
                source: "Bollinger".into(),
                type_signal: "cassure_haute".into(),
                direction: DirectionSignal::Bullish,
                force: NiveauForce::Moyen,
                description: format!(
                    "Clôture au-dessus de la bande haute — breakout haussier ({:.5})",
                    h
                ),
                valeur: h,
                prix_entree: 0.0,
            });
        }
        // Squeeze : bandwidth au plus bas sur `lookback` bougies (Faible / Neutre)
        if i >= lookback {
            let bw = (h - b) / m;
            let bw_min = (i - lookback..i)
                .filter_map(|j| {
                    let mj = milieu[j];
                    if haute[j].is_nan() || mj == 0.0 {
                        None
                    } else {
                        Some((haute[j] - basse[j]) / mj)
                    }
                })
                .fold(f64::INFINITY, f64::min);
            if bw < bw_min && bw_min.is_finite() {
                signaux.push(SignalIndicateur {
                    timestamp: timestamps[i],
                    source: "Bollinger".into(),
                    type_signal: "squeeze".into(),
                    direction: DirectionSignal::Neutre,
                    force: NiveauForce::Faible,
                    description: format!(
                        "Squeeze — bandwidth au plus bas sur {} bougies ({:.4})",
                        lookback, bw
                    ),
                    valeur: bw,
                prix_entree: 0.0,
                });
            }
        }
    }
    signaux
}
