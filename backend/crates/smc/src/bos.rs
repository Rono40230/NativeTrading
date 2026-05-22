use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

/// Résultat d'une détection de Break of Structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatBos {
    /// Direction du BOS : Long = cassure swing high, Short = cassure swing low
    pub direction: Direction,
    /// Niveau du swing cassé
    pub niveau_casse: f64,
    /// Prix de clôture qui a cassé le niveau
    pub prix_cassure: f64,
}

/// Lookback pour identifier un swing high/low (bougies de chaque côté)
const LOOKBACK: usize = 3;

/// Nombre de bougies récentes à scanner pour trouver le BOS le plus récent
const MAX_SCAN: usize = 50;

/// Détecte le Break of Structure le plus récent dans les dernières bougies.
///
/// BOS Long  = une bougie a cassé un swing high récent (résistance pivot)
/// BOS Short = une bougie a cassé un swing low récent (support pivot)
///
/// Scanne les `MAX_SCAN` dernières bougies pour retourner l'événement le plus récent.
pub fn detecter_bos(bougies: &[Candle]) -> Option<ResultatBos> {
    let n = bougies.len();
    let min_requis = 2 * LOOKBACK + 2;
    if n < min_requis {
        return None;
    }

    // Scan du plus récent au plus ancien pour trouver le BOS le plus récent
    // min_idx = minimum idx pour que l'historique soit suffisant pour les swings
    let min_idx = 2 * LOOKBACK + 1;
    let debut_scan = n.saturating_sub(MAX_SCAN).max(min_idx);

    for idx in (debut_scan..n).rev() {
        let historique = &bougies[..idx];
        let close = bougies[idx].close;

        let swing_high = dernier_swing_high(historique, LOOKBACK);
        let swing_low = dernier_swing_low(historique, LOOKBACK);

        if let Some(niveau) = swing_high {
            if close > niveau {
                return Some(ResultatBos {
                    direction: Direction::Long,
                    niveau_casse: niveau,
                    prix_cassure: close,
                });
            }
        }

        if let Some(niveau) = swing_low {
            if close < niveau {
                return Some(ResultatBos {
                    direction: Direction::Short,
                    niveau_casse: niveau,
                    prix_cassure: close,
                });
            }
        }
    }

    None
}

/// Retourne le dernier swing high pivot dans la série (le plus récent).
fn dernier_swing_high(bougies: &[Candle], n: usize) -> Option<f64> {
    let len = bougies.len();
    // Parcourir de la droite vers la gauche pour trouver le plus récent
    for i in (n..len.saturating_sub(n)).rev() {
        let pivot = bougies[i].high;
        let gauche = bougies[i - n..i].iter().all(|b| b.high <= pivot);
        let droite = bougies[i + 1..=i + n].iter().all(|b| b.high <= pivot);
        if gauche && droite {
            return Some(pivot);
        }
    }
    None
}

/// Retourne le dernier swing low pivot dans la série (le plus récent).
fn dernier_swing_low(bougies: &[Candle], n: usize) -> Option<f64> {
    let len = bougies.len();
    for i in (n..len.saturating_sub(n)).rev() {
        let pivot = bougies[i].low;
        let gauche = bougies[i - n..i].iter().all(|b| b.low >= pivot);
        let droite = bougies[i + 1..=i + n].iter().all(|b| b.low >= pivot);
        if gauche && droite {
            return Some(pivot);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

    fn bougie(o: f64, h: f64, l: f64, c: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 1000.0,
        }
    }

    /// Série avec un swing high à 105 cassé par close à 108 → BOS Long
    #[test]
    fn bos_long_cassure_swing_high() {
        let mut bougies: Vec<Candle> = Vec::new();
        // Phase montée → swing high
        for _ in 0..3 {
            bougies.push(bougie(98.0, 100.0, 97.0, 99.0));
        }
        // Pivot haut à 105
        bougies.push(bougie(99.0, 105.0, 98.0, 100.0));
        // Retrait après le pivot
        for _ in 0..3 {
            bougies.push(bougie(100.0, 102.0, 98.0, 99.0));
        }
        // Bougie finale qui casse le swing high
        bougies.push(bougie(99.0, 109.0, 98.0, 108.0));

        let res = detecter_bos(&bougies).expect("BOS Long attendu");
        assert_eq!(res.direction, Direction::Long);
        assert!((res.niveau_casse - 105.0).abs() < 0.001);
    }

    /// Série avec un swing low à 95 cassé par close à 92 → BOS Short
    #[test]
    fn bos_short_cassure_swing_low() {
        let mut bougies: Vec<Candle> = Vec::new();
        // Phase descente → swing low
        for _ in 0..3 {
            bougies.push(bougie(100.0, 101.0, 98.0, 99.0));
        }
        // Pivot bas à 95
        bougies.push(bougie(99.0, 100.0, 95.0, 96.0));
        // Rebond après le pivot
        for _ in 0..3 {
            bougies.push(bougie(96.0, 99.0, 95.5, 98.0));
        }
        // Bougie finale qui casse le swing low
        bougies.push(bougie(98.0, 99.0, 91.0, 92.0));

        let res = detecter_bos(&bougies).expect("BOS Short attendu");
        assert_eq!(res.direction, Direction::Short);
        assert!((res.niveau_casse - 95.0).abs() < 0.001);
    }

    /// Série sans cassure → None
    #[test]
    fn bos_aucune_cassure() {
        let bougies: Vec<Candle> = (0..12)
            .map(|_| bougie(99.0, 101.0, 98.0, 100.0))
            .collect();
        assert!(detecter_bos(&bougies).is_none());
    }
}
