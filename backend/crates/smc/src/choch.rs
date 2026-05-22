use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

/// Résultat d'une détection de Change of Character.
///
/// Le CHoCH est le **premier** BOS contre la tendance en cours.
/// Il signale un potentiel retournement de structure (contrairement au BOS
/// qui confirme la continuité).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatChoch {
    /// Direction du CHoCH : Long = premier retournement haussier, Short = premier retournement baissier
    pub direction: Direction,
    /// Niveau qui a été cassé (dernier swing dans la direction opposée)
    pub niveau_casse: f64,
    /// Prix de clôture qui a effectué le CHoCH
    pub prix_cassure: f64,
}

const LOOKBACK: usize = 3;

/// Nombre de bougies récentes à scanner pour trouver le CHoCH le plus récent
const MAX_SCAN: usize = 50;

/// Détecte le Change of Character (CHoCH) le plus récent dans les dernières bougies.
///
/// CHoCH Long  = tendance baissière (LH+LL) puis cassure haussière d'un swing high récent
/// CHoCH Short = tendance haussière (HH+HL) puis cassure baissière d'un swing low récent
///
/// Différence avec BOS : le CHoCH va à l'encontre de la tendance structurelle,
/// c'est donc la **première** rupture de la structure en place.
///
/// Scanne les `MAX_SCAN` dernières bougies pour retourner l'événement le plus récent.
pub fn detecter_choch(bougies: &[Candle]) -> Option<ResultatChoch> {
    let n = bougies.len();
    let min_requis = 2 * LOOKBACK + 6;
    if n < min_requis {
        return None;
    }

    // Scan du plus récent au plus ancien
    // min_idx = minimum idx pour que l'historique soit suffisant (tendance + swings)
    let min_idx = 2 * LOOKBACK + 5;
    let debut_scan = n.saturating_sub(MAX_SCAN).max(min_idx);

    for idx in (debut_scan..n).rev() {
        let historique = &bougies[..idx];
        let close = bougies[idx].close;

        let tendance = match tendance_recente(historique, LOOKBACK) {
            Some(t) => t,
            None => continue,
        };

        match tendance {
            Direction::Short => {
                // Tendance baissière → CHoCH Long si on casse un swing high récent
                if let Some(swing_high) = dernier_swing_high(historique, LOOKBACK) {
                    if close > swing_high {
                        return Some(ResultatChoch {
                            direction: Direction::Long,
                            niveau_casse: swing_high,
                            prix_cassure: close,
                        });
                    }
                }
            }
            Direction::Long => {
                // Tendance haussière → CHoCH Short si on casse un swing low récent
                if let Some(swing_low) = dernier_swing_low(historique, LOOKBACK) {
                    if close < swing_low {
                        return Some(ResultatChoch {
                            direction: Direction::Short,
                            niveau_casse: swing_low,
                            prix_cassure: close,
                        });
                    }
                }
            }
            Direction::Both => {}
        }
    }

    None
}

/// Détermine la tendance récente à partir des 2 derniers pivots.
fn tendance_recente(bougies: &[Candle], n: usize) -> Option<Direction> {
    let sommets = pivots_hauts(bougies, n);
    let creux = pivots_bas(bougies, n);

    if sommets.len() < 2 || creux.len() < 2 {
        return None;
    }

    let lh = sommets[sommets.len() - 1] < sommets[sommets.len() - 2]; // Lower High
    let ll = creux[creux.len() - 1] < creux[creux.len() - 2]; // Lower Low
    let hh = sommets[sommets.len() - 1] > sommets[sommets.len() - 2]; // Higher High
    let hl = creux[creux.len() - 1] > creux[creux.len() - 2]; // Higher Low

    // Tendance confirmée par les deux composantes (HH+HL ou LH+LL)
    match (hh && hl, lh && ll) {
        (true, false) => Some(Direction::Long),
        (false, true) => Some(Direction::Short),
        // Tendance partielle : LH seul ou LL seul suffit
        (false, false) if lh => Some(Direction::Short),
        (false, false) if ll => Some(Direction::Short),
        (false, false) if hh => Some(Direction::Long),
        (false, false) if hl => Some(Direction::Long),
        _ => None,
    }
}

fn pivots_hauts(bougies: &[Candle], n: usize) -> Vec<f64> {
    let mut out = Vec::new();
    let len = bougies.len();
    for i in n..len.saturating_sub(n) {
        let pivot = bougies[i].high;
        let gauche = bougies[i - n..i].iter().all(|b| b.high <= pivot);
        let droite = bougies[i + 1..=i + n].iter().all(|b| b.high <= pivot);
        if gauche && droite {
            out.push(pivot);
        }
    }
    out
}

fn pivots_bas(bougies: &[Candle], n: usize) -> Vec<f64> {
    let mut out = Vec::new();
    let len = bougies.len();
    for i in n..len.saturating_sub(n) {
        let pivot = bougies[i].low;
        let gauche = bougies[i - n..i].iter().all(|b| b.low >= pivot);
        let droite = bougies[i + 1..=i + n].iter().all(|b| b.low >= pivot);
        if gauche && droite {
            out.push(pivot);
        }
    }
    out
}

fn dernier_swing_high(bougies: &[Candle], n: usize) -> Option<f64> {
    pivots_hauts(bougies, n).into_iter().last()
}

fn dernier_swing_low(bougies: &[Candle], n: usize) -> Option<f64> {
    pivots_bas(bougies, n).into_iter().last()
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

    /// Tendance baissière (LH+LL) puis close dépasse un swing high → CHoCH Long
    #[test]
    fn choch_long_sur_tendance_baissiere() {
        // Série : 2 pivots hauts (LH) + 2 pivots bas (LL) clairement séparés (LOOKBACK=3)
        let mut b: Vec<Candle> = Vec::new();

        // Bougies avant pivot haut 1
        for _ in 0..3 { b.push(bougie(100.0, 102.0, 99.0, 101.0)); }
        // Pivot haut 1 : high=115
        b.push(bougie(101.0, 115.0, 100.0, 101.0));
        // 3 bougies avant pivot bas 1
        for _ in 0..3 { b.push(bougie(101.0, 103.0, 98.0, 100.0)); }
        // Pivot bas 1 : low=94
        b.push(bougie(100.0, 101.0, 94.0, 100.0));
        // 3 bougies avant pivot haut 2
        for _ in 0..3 { b.push(bougie(100.0, 102.0, 99.0, 101.0)); }
        // Pivot haut 2 : LH=108 < 115
        b.push(bougie(101.0, 108.0, 100.0, 101.0));
        // 3 bougies avant pivot bas 2
        for _ in 0..3 { b.push(bougie(101.0, 103.0, 97.0, 99.0)); }
        // Pivot bas 2 : LL=90 < 94
        b.push(bougie(99.0, 100.0, 90.0, 98.0));
        // 3 bougies après le pivot bas 2 (nécessaire pour valider le pivot)
        for _ in 0..3 { b.push(bougie(98.0, 100.0, 91.0, 99.0)); }
        // Bougie finale : casse le dernier swing high (108) → CHoCH Long
        b.push(bougie(99.0, 120.0, 98.0, 119.0));

        let res = detecter_choch(&b).expect("CHoCH Long attendu");
        assert_eq!(res.direction, Direction::Long);
    }

    /// Tendance haussière (HH+HL) puis close casse un swing low → CHoCH Short
    #[test]
    fn choch_short_sur_tendance_haussiere() {
        let mut bougies: Vec<Candle> = Vec::new();
        // 3 bougies avant le 1er pivot bas
        for _ in 0..3 {
            bougies.push(bougie(100.0, 102.0, 98.0, 101.0));
        }
        // Pivot bas 1 (valeur basse = 95)
        bougies.push(bougie(101.0, 102.0, 95.0, 101.0));
        // 3 bougies entre les pivots
        for _ in 0..3 {
            bougies.push(bougie(101.0, 104.0, 99.0, 103.0));
        }
        // Pivot bas 2 (HL = 97 > 95)
        bougies.push(bougie(103.0, 104.0, 97.0, 103.0));
        // 3 bougies entre les pivots
        for _ in 0..3 {
            bougies.push(bougie(103.0, 107.0, 102.0, 106.0));
        }
        // Pivot haut 1 (valeur haute = 108)
        bougies.push(bougie(106.0, 108.0, 105.0, 106.0));
        // 3 bougies entre les pivots
        for _ in 0..3 {
            bougies.push(bougie(106.0, 112.0, 105.0, 111.0));
        }
        // Pivot haut 2 (HH = 115 > 108)
        bougies.push(bougie(111.0, 115.0, 110.0, 111.0));
        // 3 bougies après ce pivot
        for _ in 0..3 {
            bougies.push(bougie(111.0, 113.0, 109.0, 112.0));
        }
        // Bougie finale qui casse le dernier swing low (97) → CHoCH Short
        bougies.push(bougie(112.0, 113.0, 88.0, 89.0));

        let res = detecter_choch(&bougies).expect("CHoCH Short attendu");
        assert_eq!(res.direction, Direction::Short);
    }
}
