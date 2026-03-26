use common::Candle;

/// Résultat d'un sweep de liquidité détecté sur les bougies récentes.
#[derive(Debug, Clone)]
pub struct SweepLiquidite {
    /// Prix du niveau swing sweepé (swing low pour un SSL, swing high pour un BSL).
    pub prix_sweep: f64,
    /// `true` = SSL sweepé → signal Long attendu.
    /// `false` = BSL sweepé → signal Short attendu.
    pub ssl_swepe: bool,
    /// Index de la bougie de sweep dans le slice fourni.
    pub index_bougie: usize,
}

/// Fenêtre de recherche : combien de bougies avant la fin on analyse pour trouver un sweep.
const FENETRE_SWEEP: usize = 5;

/// Profondeur de recherche des swing high/low référence.
const PROFONDEUR_SWING: usize = 20;

/// Nombre de bougies de chaque côté pour valider un swing.
const SWING_VOISINS: usize = 3;

/// Détecte un sweep de liquidité récent dans le slice de bougies.
///
/// Un sweep est validé si :
/// 1. Une bougie (parmi les `FENETRE_SWEEP` dernières) wick au-delà d'un swing high/low récent
/// 2. La clôture de cette bougie reste dans la structure (en-deçà du niveau sweepé)
/// 3. La bougie suivante confirme le retour dans la structure
///
/// Retourne le sweep le plus récent, ou `None`.
pub fn detecter_sweep(bougies: &[Candle]) -> Option<SweepLiquidite> {
    let n = bougies.len();
    if n < PROFONDEUR_SWING + FENETRE_SWEEP + 2 {
        return None;
    }

    for sweep_idx in (n.saturating_sub(FENETRE_SWEEP + 1)..n - 1).rev() {
        let sweep = &bougies[sweep_idx];
        let confirme = &bougies[sweep_idx + 1];

        // --- SSL sweep : wick bas sous un swing low → signal Long ---
        if let Some(swing_low) = trouver_swing_low(bougies, sweep_idx) {
            if sweep.low < swing_low && sweep.close > swing_low && confirme.close > swing_low {
                return Some(SweepLiquidite {
                    prix_sweep: swing_low,
                    ssl_swepe: true,
                    index_bougie: sweep_idx,
                });
            }
        }

        // --- BSL sweep : wick haut au-dessus d'un swing high → signal Short ---
        if let Some(swing_high) = trouver_swing_high(bougies, sweep_idx) {
            if sweep.high > swing_high && sweep.close < swing_high && confirme.close < swing_high {
                return Some(SweepLiquidite {
                    prix_sweep: swing_high,
                    ssl_swepe: false,
                    index_bougie: sweep_idx,
                });
            }
        }
    }

    None
}

fn trouver_swing_low(bougies: &[Candle], avant_idx: usize) -> Option<f64> {
    let debut = avant_idx.saturating_sub(PROFONDEUR_SWING);
    let fin = avant_idx.saturating_sub(SWING_VOISINS + 1);
    if debut >= fin {
        return None;
    }
    let slice = &bougies[debut..fin];
    if slice.len() < SWING_VOISINS * 2 + 1 {
        return None;
    }
    slice
        .windows(SWING_VOISINS * 2 + 1)
        .filter_map(|w| {
            let centre = SWING_VOISINS;
            let low = w[centre].low;
            let est_swing = w[..centre].iter().all(|b| b.low >= low)
                && w[centre + 1..].iter().all(|b| b.low >= low);
            if est_swing {
                Some(low)
            } else {
                None
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn trouver_swing_high(bougies: &[Candle], avant_idx: usize) -> Option<f64> {
    let debut = avant_idx.saturating_sub(PROFONDEUR_SWING);
    let fin = avant_idx.saturating_sub(SWING_VOISINS + 1);
    if debut >= fin {
        return None;
    }
    let slice = &bougies[debut..fin];
    if slice.len() < SWING_VOISINS * 2 + 1 {
        return None;
    }
    slice
        .windows(SWING_VOISINS * 2 + 1)
        .filter_map(|w| {
            let centre = SWING_VOISINS;
            let high = w[centre].high;
            let est_swing = w[..centre].iter().all(|b| b.high <= high)
                && w[centre + 1..].iter().all(|b| b.high <= high);
            if est_swing {
                Some(high)
            } else {
                None
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn b(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    #[test]
    fn sweep_none_si_moins_de_27_bougies() {
        // PROFONDEUR_SWING(20) + FENETRE_SWEEP(5) + 2 = 27 minimum
        let bougies: Vec<Candle> = (0..26).map(|i| b(i as f64 + 10., i as f64 + 11., i as f64 + 9., i as f64 + 10.5)).collect();
        assert!(
            detecter_sweep(&bougies).is_none(),
            "Moins de 27 bougies → None"
        );
    }

    #[test]
    fn sweep_none_sur_prix_monotone_croissant() {
        // Prix strictement croissants → aucun swing low → aucun sweep possible
        let bougies: Vec<Candle> = (0..35)
            .map(|i| b(i as f64 * 10. + 1., i as f64 * 10. + 9., i as f64 * 10. + 1., i as f64 * 10. + 5.))
            .collect();
        assert!(
            detecter_sweep(&bougies).is_none(),
            "Prix monotone → pas de sweep"
        );
    }
}
