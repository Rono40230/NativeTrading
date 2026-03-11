use common::Candle;

#[derive(Debug, Clone)]
pub struct NiveauxFibonacci {
    pub swing_haut: f64,
    pub swing_bas: f64,
    pub niveau_236: f64,
    pub niveau_382: f64,
    pub niveau_500: f64,
    pub niveau_618: f64,
    pub niveau_786: f64,
}

/// Calcule les niveaux de retracement de Fibonacci
/// à partir du swing high et low des N dernières bougies.
pub fn calculer(bougies: &[Candle]) -> Option<NiveauxFibonacci> {
    if bougies.len() < 20 {
        return None;
    }

    let lookback = bougies.len().min(60);
    let slice = &bougies[bougies.len() - lookback..];

    let swing_haut = slice
        .iter()
        .map(|b| b.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let swing_bas = slice.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);

    let range = swing_haut - swing_bas;
    if range < 1e-10 {
        return None;
    }

    Some(NiveauxFibonacci {
        swing_haut,
        swing_bas,
        niveau_236: swing_haut - range * 0.236,
        niveau_382: swing_haut - range * 0.382,
        niveau_500: swing_haut - range * 0.500,
        niveau_618: swing_haut - range * 0.618,
        niveau_786: swing_haut - range * 0.786,
    })
}

/// Vérifie si le prix est proche d'un niveau clé (38.2%, 50%, 61.8%)
/// avec une tolérance en % du prix.
pub fn prix_sur_niveau(prix: f64, niveaux: &NiveauxFibonacci, tolerance_pct: f64) -> Option<f64> {
    [niveaux.niveau_382, niveaux.niveau_500, niveaux.niveau_618]
        .iter()
        .copied()
        .find(|&niveau| (prix - niveau).abs() / prix.max(1e-10) <= tolerance_pct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie(high: f64, low: f64) -> common::Candle {
        let mid = (high + low) / 2.0;
        common::Candle {
            timestamp: Utc::now(),
            open: mid,
            high,
            low,
            close: mid,
            volume: 1000.0,
        }
    }

    #[test]
    fn niveaux_calcules_correctement() {
        let bougies: Vec<common::Candle> = (0..25)
            .map(|i| bougie(100.0 + i as f64, 80.0 + i as f64))
            .collect();
        let niveaux = calculer(&bougies).unwrap();
        // Range = (100+24) - 80 = 44
        assert!(niveaux.swing_haut > niveaux.swing_bas);
        assert!(niveaux.niveau_618 < niveaux.niveau_382);
        assert!(niveaux.niveau_382 < niveaux.swing_haut);
        assert!(niveaux.niveau_618 > niveaux.swing_bas);
    }
}
