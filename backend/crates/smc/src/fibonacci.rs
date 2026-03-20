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

/// Collecte tous les pivots hauts locaux (entourés de `rayon` bougies plus basses).
fn pivots_hauts(slice: &[Candle], rayon: usize) -> Vec<usize> {
    let n = slice.len();
    let mut res = Vec::new();
    for i in rayon..n.saturating_sub(rayon) {
        let h = slice[i].high;
        if slice[i.saturating_sub(rayon)..i].iter().all(|b| b.high < h)
            && slice[i + 1..=(i + rayon).min(n - 1)].iter().all(|b| b.high < h)
        {
            res.push(i);
        }
    }
    res
}

/// Collecte tous les pivots bas locaux.
fn pivots_bas(slice: &[Candle], rayon: usize) -> Vec<usize> {
    let n = slice.len();
    let mut res = Vec::new();
    for i in rayon..n.saturating_sub(rayon) {
        let l = slice[i].low;
        if slice[i.saturating_sub(rayon)..i].iter().all(|b| b.low > l)
            && slice[i + 1..=(i + rayon).min(n - 1)].iter().all(|b| b.low > l)
        {
            res.push(i);
        }
    }
    res
}

/// Détecte le dernier swing impulsif complet sur le slice.
///
/// Principe (Option 2) :
/// - Trouver le dernier pivot (haut ou bas) — c'est la fin de l'impulsion.
/// - Chercher le pivot opposé le plus récent qui le **précède** — c'est le début.
/// - Le swing retenu est celui dont le range est le plus grand parmi les candidats.
///
/// Retourne `(index_debut, index_fin, est_haussier)` :
/// - haussier  : debut = pivot bas, fin = pivot haut  (retracement vers le bas attendu)
/// - baissier  : debut = pivot haut, fin = pivot bas   (retracement vers le haut attendu)
fn dernier_swing(slice: &[Candle], rayon: usize) -> Option<(usize, usize, bool)> {
    let hauts = pivots_hauts(slice, rayon);
    let bas   = pivots_bas(slice, rayon);

    if hauts.is_empty() || bas.is_empty() {
        return None;
    }

    let dernier_h = *hauts.last().unwrap();
    let dernier_l = *bas.last().unwrap();

    // Cas 1 : dernière impulsion haussière (bas → haut, haut est plus récent)
    if dernier_h > dernier_l {
        // Chercher le pivot bas le plus récent AVANT dernier_h
        let debut_l = bas.iter().rev().find(|&&i| i < dernier_h).copied()?;
        Some((debut_l, dernier_h, true))
    } else {
        // Cas 2 : dernière impulsion baissière (haut → bas)
        let debut_h = hauts.iter().rev().find(|&&i| i < dernier_l).copied()?;
        Some((debut_h, dernier_l, false))
    }
}

/// Calcule les niveaux de retracement Fibonacci (0%, 50%, 61.8%, 78.6%, 100%)
/// à partir du dernier swing impulsif complet (Option 2 — pivot bas→haut ou haut→bas).
pub fn calculer(bougies: &[Candle]) -> Option<NiveauxFibonacci> {
    if bougies.len() < 20 {
        return None;
    }

    let lookback = bougies.len().min(100);
    let slice = &bougies[bougies.len() - lookback..];

    let (idx_debut, idx_fin, haussier) = dernier_swing(slice, 3).or_else(|| {
        // Fallback : max/min absolu si aucun pivot détecté
        let idx_h = slice.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.high.partial_cmp(&b.high).unwrap_or(std::cmp::Ordering::Equal))?.0;
        let idx_l = slice.iter().enumerate()
            .min_by(|(_, a), (_, b)| a.low.partial_cmp(&b.low).unwrap_or(std::cmp::Ordering::Equal))?.0;
        if idx_l < idx_h { Some((idx_l, idx_h, true)) } else { Some((idx_h, idx_l, false)) }
    })?;

    let (swing_haut, ts_haut, swing_bas, ts_bas) = if haussier {
        (slice[idx_fin].high,  slice[idx_fin].timestamp.timestamp(),
         slice[idx_debut].low, slice[idx_debut].timestamp.timestamp())
    } else {
        (slice[idx_debut].high, slice[idx_debut].timestamp.timestamp(),
         slice[idx_fin].low,   slice[idx_fin].timestamp.timestamp())
    };

    let range = swing_haut - swing_bas;
    if range < 1e-10 {
        return None;
    }

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
