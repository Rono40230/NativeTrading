use common::Candle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NiveauxFibonacci {
    pub swing_haut: f64,
    pub swing_bas: f64,
    /// Unix secondes — timestamp du pivot haut (bord gauche des segments)
    pub timestamp_haut: i64,
    /// Unix secondes — timestamp du pivot bas (bord gauche des segments)
    pub timestamp_bas: i64,
    pub niveau_500: f64,
    pub niveau_618: f64,
    pub niveau_786: f64,
}

/// Identifie le dernier pivot haut local dans le slice (entouré de N bougies plus basses).
fn dernier_pivot_haut(slice: &[Candle], rayon: usize) -> Option<&Candle> {
    let n = slice.len();
    // Parcours de droite à gauche pour trouver le plus récent
    for i in (rayon..n.saturating_sub(rayon)).rev() {
        let haut = slice[i].high;
        let est_pivot = slice[i.saturating_sub(rayon)..i].iter().all(|b| b.high < haut)
            && slice[i + 1..=(i + rayon).min(n - 1)].iter().all(|b| b.high < haut);
        if est_pivot {
            return Some(&slice[i]);
        }
    }
    None
}

/// Identifie le dernier pivot bas local dans le slice.
fn dernier_pivot_bas(slice: &[Candle], rayon: usize) -> Option<&Candle> {
    let n = slice.len();
    for i in (rayon..n.saturating_sub(rayon)).rev() {
        let bas = slice[i].low;
        let est_pivot = slice[i.saturating_sub(rayon)..i].iter().all(|b| b.low > bas)
            && slice[i + 1..=(i + rayon).min(n - 1)].iter().all(|b| b.low > bas);
        if est_pivot {
            return Some(&slice[i]);
        }
    }
    None
}

/// Calcule les niveaux de retracement Fibonacci (0%, 50%, 61.8%, 78.6%, 100%)
/// à partir du dernier pivot haut et du dernier pivot bas des N dernières bougies.
pub fn calculer(bougies: &[Candle]) -> Option<NiveauxFibonacci> {
    if bougies.len() < 20 {
        return None;
    }

    let lookback = bougies.len().min(100);
    let slice = &bougies[bougies.len() - lookback..];

    // Rayon 3 : pivot entouré de 3 bougies de chaque côté
    let pivot_haut = dernier_pivot_haut(slice, 3).unwrap_or_else(|| {
        // Fallback : bougie avec le plus haut HIGH
        slice.iter().max_by(|a, b| a.high.partial_cmp(&b.high).unwrap_or(std::cmp::Ordering::Equal)).unwrap()
    });
    let pivot_bas = dernier_pivot_bas(slice, 3).unwrap_or_else(|| {
        slice.iter().min_by(|a, b| a.low.partial_cmp(&b.low).unwrap_or(std::cmp::Ordering::Equal)).unwrap()
    });

    let swing_haut = pivot_haut.high;
    let swing_bas  = pivot_bas.low;
    let range = swing_haut - swing_bas;
    if range < 1e-10 {
        return None;
    }

    let ts_haut = pivot_haut.timestamp.timestamp();
    let ts_bas  = pivot_bas.timestamp.timestamp();

    Some(NiveauxFibonacci {
        swing_haut,
        swing_bas,
        timestamp_haut: ts_haut,
        timestamp_bas:  ts_bas,
        niveau_500: swing_haut - range * 0.500,
        niveau_618: swing_haut - range * 0.618,
        niveau_786: swing_haut - range * 0.786,
    })
}

/// Vérifie si le prix est proche d'un niveau clé (50%, 61.8%, 78.6%)
/// avec une tolérance en % du prix.
pub fn prix_sur_niveau(prix: f64, niveaux: &NiveauxFibonacci, tolerance_pct: f64) -> Option<f64> {
    [niveaux.niveau_500, niveaux.niveau_618, niveaux.niveau_786]
        .iter()
        .copied()
        .find(|&niveau| (prix - niveau).abs() / prix.max(1e-10) <= tolerance_pct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie_ts(high: f64, low: f64, offset_s: i64) -> common::Candle {
        let mid = (high + low) / 2.0;
        let ts = Utc::now() + chrono::Duration::seconds(offset_s);
        common::Candle { timestamp: ts, open: mid, high, low, close: mid, volume: 1000.0 }
    }

    #[test]
    fn niveaux_calcules_correctement() {
        // Créer un pivot haut isolé puis un pivot bas isolé clairement détectables
        let mut bougies: Vec<common::Candle> = (0..25)
            .map(|i| bougie_ts(50.0, 40.0, i * 60))
            .collect();
        // Pivot haut à l'index 10
        bougies[10] = bougie_ts(100.0, 40.0, 10 * 60);
        // Pivot bas à l'index 20
        bougies[20] = bougie_ts(50.0, 10.0, 20 * 60);

        let niveaux_opt = calculer(&bougies);
        assert!(niveaux_opt.is_some(), "calculer doit retourner Some pour 25 bougies");
        if let Some(niveaux) = niveaux_opt {
            assert!(niveaux.swing_haut > niveaux.swing_bas);
            assert!(niveaux.niveau_618 < niveaux.niveau_500);
            assert!(niveaux.niveau_500 < niveaux.swing_haut);
            assert!(niveaux.niveau_786 > niveaux.swing_bas);
        }
    }
}
