use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

/// Nombre de bougies de chaque côté pour valider un pivot
const LOOKBACK_PIVOT: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatTendance {
    pub direction: Direction,
    pub dernier_sommet: f64,
    pub dernier_creux: f64,
    /// Force de la tendance 0–2 : 0=indécis, 1=partiel, 2=confirmé (HH+HL ou LH+LL)
    pub force: f64,
}

/// Trouve les pivots hauts locaux (n bougies de chaque côté)
fn sommets(bougies: &[Candle], n: usize) -> Vec<f64> {
    let mut out = Vec::new();
    for i in n..bougies.len().saturating_sub(n) {
        let pivot = bougies[i].high;
        let gauche = bougies[i - n..i].iter().all(|b| b.high <= pivot);
        let droite = bougies[i + 1..=i + n].iter().all(|b| b.high <= pivot);
        if gauche && droite {
            out.push(pivot);
        }
    }
    out
}

/// Trouve les pivots bas locaux
fn creux(bougies: &[Candle], n: usize) -> Vec<f64> {
    let mut out = Vec::new();
    for i in n..bougies.len().saturating_sub(n) {
        let pivot = bougies[i].low;
        let gauche = bougies[i - n..i].iter().all(|b| b.low >= pivot);
        let droite = bougies[i + 1..=i + n].iter().all(|b| b.low >= pivot);
        if gauche && droite {
            out.push(pivot);
        }
    }
    out
}

/// Analyse la structure de marché (HH/HL = haussière, LH/LL = baissère).
pub fn analyser(bougies: &[Candle]) -> Option<ResultatTendance> {
    if bougies.len() < 20 {
        return None;
    }
    let n = LOOKBACK_PIVOT;
    // Prendre les 2 derniers pivots de chaque type (ordre décroissant = plus récent en premier)
    let mut s = sommets(bougies, n);
    let mut c = creux(bougies, n);
    s.reverse();
    c.reverse();

    if s.len() < 2 || c.len() < 2 {
        return None;
    }

    let hh = s[0] > s[1]; // Higher High
    let hl = c[0] > c[1]; // Higher Low
    let lh = s[0] < s[1]; // Lower High
    let ll = c[0] < c[1]; // Lower Low

    let (direction, force) = match (hh, hl, lh, ll) {
        (true, true, _, _) => (Direction::Long, 2.0),
        (_, _, true, true) => (Direction::Short, 2.0),
        (true, false, _, _) | (false, true, _, _) => (Direction::Long, 1.0),
        (_, _, true, false) | (_, _, false, true) => (Direction::Short, 1.0),
        _ => (Direction::Both, 0.0),
    };

    Some(ResultatTendance {
        direction,
        dernier_sommet: s[0],
        dernier_creux: c[0],
        force,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie(high: f64, low: f64) -> Candle {
        let mid = (high + low) / 2.0;
        Candle {
            timestamp: Utc::now(),
            open: mid,
            high,
            low,
            close: mid,
            volume: 1000.0,
        }
    }

    #[test]
    fn tendance_haussiere_detectee() {
        // Structure HH/HL avec 2 pivots hauts + 2 pivots bas clairement séparés (LOOKBACK=3)
        // Creux 1 à i=3 (l=80), Sommet 1 à i=7 (h=120)
        // Creux 2 à i=11 (l=90, HL), Sommet 2 à i=15 (h=130, HH)
        let bougies: Vec<Candle> = vec![
            bougie(103.0, 83.0),  // i=0
            bougie(102.0, 82.0),  // i=1
            bougie(101.0, 81.0),  // i=2
            bougie(100.0, 80.0),  // i=3 — creux 1 (l=80)
            bougie(105.0, 84.0),  // i=4
            bougie(110.0, 88.0),  // i=5
            bougie(115.0, 92.0),  // i=6
            bougie(120.0, 95.0),  // i=7 — sommet 1 (h=120)
            bougie(118.0, 93.0),  // i=8
            bougie(116.0, 92.0),  // i=9
            bougie(112.0, 91.0),  // i=10
            bougie(115.0, 90.0),  // i=11 — creux 2 (l=90, HL > 80)
            bougie(118.0, 93.0),  // i=12
            bougie(122.0, 96.0),  // i=13
            bougie(126.0, 100.0), // i=14
            bougie(130.0, 104.0), // i=15 — sommet 2 (h=130, HH > 120)
            bougie(128.0, 103.0), // i=16
            bougie(125.0, 101.0), // i=17
            bougie(122.0, 98.0),  // i=18
            bougie(120.0, 96.0),  // i=19
            bougie(118.0, 94.0),  // i=20
            bougie(116.0, 92.0),  // i=21
        ];
        let res = analyser(&bougies);
        assert!(
            res.is_some(),
            "analyser doit retourner Some pour des bougies valides"
        );
        if let Some(r) = res {
            assert_eq!(r.direction, Direction::Long);
            assert_eq!(r.force, 2.0); // HH + HL confirmé
        }
    }
}
