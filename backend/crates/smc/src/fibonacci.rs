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
/// Construit tous les swings complets (paire pivot_debut → pivot_fin) depuis les listes de pivots.
/// Retourne `(idx_debut, idx_fin, est_haussier, range)` triés du plus ancien au plus récent.
fn tous_les_swings(
    slice: &[Candle],
    hauts: &[usize],
    bas: &[usize],
) -> Vec<(usize, usize, bool, f64)> {
    let mut swings = Vec::new();

    // Swings haussiers : chercher chaque pivot haut et le pivot bas qui le précède
    for &ih in hauts {
        if let Some(&il) = bas.iter().rev().find(|&&i| i < ih) {
            let range = slice[ih].high - slice[il].low;
            swings.push((il, ih, true, range));
        }
    }
    // Swings baissiers : chercher chaque pivot bas et le pivot haut qui le précède
    for &il in bas {
        if let Some(&ih) = hauts.iter().rev().find(|&&i| i < il) {
            let range = slice[ih].high - slice[il].low;
            swings.push((ih, il, false, range));
        }
    }

    // Trier par position de fin (du plus ancien au plus récent)
    swings.sort_by_key(|&(_, fin, _, _)| fin);
    swings
}

/// Identifie le meilleur swing parmi les candidats :
/// — le plus récent dont le range est ≥ 30% du range total du slice,
/// — sinon le plus récent tout court.
fn dernier_swing(slice: &[Candle], rayon: usize) -> Option<(usize, usize, bool)> {
    let hauts = pivots_hauts(slice, rayon);
    let bas   = pivots_bas(slice, rayon);

    if hauts.is_empty() || bas.is_empty() {
        return None;
    }

    let swings = tous_les_swings(slice, &hauts, &bas);
    if swings.is_empty() {
        return None;
    }

    // Range total du slice pour le seuil de significance
    let range_total = {
        let h = slice.iter().map(|b| b.high).fold(f64::NEG_INFINITY, f64::max);
        let l = slice.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        h - l
    };
    let seuil = range_total * 0.30;

    // 1er choix : dernier swing avec range significatif
    if let Some(&(d, f, haussier, _)) = swings.iter().rev().find(|&&(_, _, _, r)| r >= seuil) {
        return Some((d, f, haussier));
    }

    // Fallback : dernier swing disponible (micro-marché)
    let &(d, f, haussier, _) = swings.last().unwrap();
    Some((d, f, haussier))
}

/// Calcule les niveaux de retracement Fibonacci (0%, 50%, 61.8%, 78.6%, 100%)
/// à partir du dernier swing impulsif significatif détecté sur les 200 dernières bougies.
pub fn calculer(bougies: &[Candle]) -> Option<NiveauxFibonacci> {
    if bougies.len() < 20 {
        return None;
    }

    // Rayon 5 : pivot entouré de 5 bougies — filtre les micro-pivots sur M1/M5
    let lookback = bougies.len().min(200);
    let slice = &bougies[bougies.len() - lookback..];

    let (idx_debut, idx_fin, haussier) = dernier_swing(slice, 5).or_else(|| {
        // Fallback absolu : max/min sur tout le slice avec leurs timestamps
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

/// Score Fibonacci SMC (0, 8 ou 15 pts) basé sur la zone de retrace.
///
/// - **15 pts** : prix dans la golden zone (entre 50% et 61.8% de retrace),
///   zone privilégiée pour une entrée en confluence.
/// - **8 pts** : prix dans la zone de deep retrace (entre 61.8% et 78.6%),
///   entrée valide mais risque plus élevé.
/// - **0 pt** : prix hors des zones (pas encore rétracé ou cassure sous 78.6%).
pub fn score_fib(prix: f64, niveaux: &NiveauxFibonacci) -> f64 {
    // niveau_500 > niveau_618 > niveau_786 (tous calculés depuis le haut)
    if prix >= niveaux.niveau_618 && prix <= niveaux.niveau_500 {
        // Golden zone : entre 50% et 61.8% de retrace
        15.0
    } else if prix >= niveaux.niveau_786 && prix < niveaux.niveau_618 {
        // Deep retrace : entre 61.8% et 78.6%
        8.0
    } else {
        0.0
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

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
