use common::Candle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneImbalance {
    /// Borne haute du gap
    pub haut: f64,
    /// Borne basse du gap
    pub bas: f64,
    /// "FvgBull" | "FvgBear" | "OgBull" | "OgBear"
    pub type_zone: String,
    /// Vrai si la zone a été remplie (prix est passé dedans)
    pub remplie: bool,
    /// Timestamp Unix (secondes) de la bougie de formation
    pub timestamp: i64,
}

/// Détecte les Fair Value Gaps (FVG) et Opening Gaps (OG) LuxAlgo.
///
/// FVG Bull : low[i] > high[i-2] ET close[i-1] > high[i-2] ET pas d'OG
/// FVG Bear : high[i] < low[i-2] ET close[i-1] < low[i-2] ET pas d'OG
/// OG Bull  : low[i] > high[i-1]   (gap complet entre bougies consécutives)
/// OG Bear  : high[i] < low[i-1]
pub fn detecter(
    bougies: &[Candle],
    show_last: usize,
    show_fvg: bool,
    show_og: bool,
    mitigation_close: bool,
) -> Vec<ZoneImbalance> {
    if bougies.len() < 3 {
        return vec![];
    }

    let n = bougies.len();
    let mut zones: Vec<ZoneImbalance> = Vec::new();

    for i in 2..n {
        let b0 = &bougies[i - 2]; // bougie gauche (la plus ancienne)
        let b1 = &bougies[i - 1]; // bougie médiane
        let b2 = &bougies[i]; // bougie droite (la plus récente)

        // ── Opening Gaps (priorité sur FVG selon LuxAlgo) ──────────────────
        // OG = gap entre 2 bougies CONSÉCUTIVES (b1 et b2), pas sur 3 bougies
        let og_bull = show_og && b2.low > b1.high;
        let og_bear = show_og && b2.high < b1.low;

        if og_bull {
            // Zone OG : entre b1.high (bas) et b2.low (haut)
            // Mitigation : prix entre dans la zone par le plafond (b2.low)
            let remplie = remplissage_bull(&bougies[i + 1..], b2.low, mitigation_close);
            zones.push(ZoneImbalance {
                haut: b2.low,
                bas: b1.high,
                type_zone: "OgBull".into(),
                remplie,
                timestamp: b1.timestamp.timestamp(),
            });
        }

        if og_bear {
            // Zone OG : entre b2.high (bas) et b1.low (haut)
            // Mitigation : prix entre dans la zone par le plancher (b2.high)
            let remplie = remplissage_bear(&bougies[i + 1..], b2.high, mitigation_close);
            zones.push(ZoneImbalance {
                haut: b1.low,
                bas: b2.high,
                type_zone: "OgBear".into(),
                remplie,
                timestamp: b1.timestamp.timestamp(),
            });
        }

        // ── Fair Value Gaps (3 bougies, pas d'OG simultané) ─────────────────
        if show_fvg {
            // FVG Haussier : gap entre b0.high et b2.low
            // Mitigation : prix redescend dans la zone par le plafond (b2.low)
            if b2.low > b0.high && b1.close > b0.high && !og_bull {
                let remplie = remplissage_bull(&bougies[i + 1..], b2.low, mitigation_close);
                zones.push(ZoneImbalance {
                    haut: b2.low,
                    bas: b0.high,
                    type_zone: "FvgBull".into(),
                    remplie,
                    timestamp: b0.timestamp.timestamp(),
                });
            }

            // FVG Baissier : gap entre b2.high et b0.low
            // Mitigation : prix remonte dans la zone par le plancher (b2.high)
            if b2.high < b0.low && b1.close < b0.low && !og_bear {
                let remplie = remplissage_bear(&bougies[i + 1..], b2.high, mitigation_close);
                zones.push(ZoneImbalance {
                    haut: b0.low,
                    bas: b2.high,
                    type_zone: "FvgBear".into(),
                    remplie,
                    timestamp: b0.timestamp.timestamp(),
                });
            }
        }
    }

    // Garder uniquement les zones non-remplies, les N plus récentes
    zones.retain(|z| !z.remplie);
    zones.reverse();
    zones.truncate(show_last);
    zones
}

/// Vrai si une bougie ultérieure comble la zone haussière (low descend sous `bas`)
fn remplissage_bull(bougies_suivantes: &[Candle], bas: f64, mitigation_close: bool) -> bool {
    bougies_suivantes.iter().any(|b| {
        if mitigation_close {
            b.close <= bas
        } else {
            b.low <= bas
        }
    })
}

/// Vrai si une bougie ultérieure comble la zone baissière (high monte au-dessus de `haut`)
fn remplissage_bear(bougies_suivantes: &[Candle], haut: f64, mitigation_close: bool) -> bool {
    bougies_suivantes.iter().any(|b| {
        if mitigation_close {
            b.close >= haut
        } else {
            b.high >= haut
        }
    })
}

/// Score Imbalance/FVG gradué (0–15 pts) pour le scorer SMC.
/// 0 zone alignée = 0 pts | 1 zone = 8 pts | 2+ zones = 15 pts
pub fn score_pour_direction(bougies: &[Candle], direction: common::Direction) -> f64 {
    let zones = detecter(bougies, 5, true, false, true);
    let type_cible = match direction {
        common::Direction::Long => "FvgBull",
        common::Direction::Short => "FvgBear",
        common::Direction::Both => return 0.0,
    };
    let nb = zones.iter().filter(|z| z.type_zone == type_cible).count();
    match nb {
        0 => 0.0,
        1 => 8.0,
        _ => 15.0,
    }
}

/// Conservé pour compatibilité — utiliser `score_continu_pour_direction()` à la place.
pub fn score_pour_direction_legacy(bougies: &[Candle], direction: common::Direction) -> f64 {
    score_pour_direction(bougies, direction)
}

/// Score Imbalance/FVG continu basé sur la proximité du prix (0–15 pts).
///
/// Chaque zone FVG alignée contribue proportionnellement à sa proximité avec `prix_actuel` :
/// - Zone au prix : ~7.5 pts
/// - Zone à 5% du prix : ~3.75 pts
/// - Plusieurs zones : contributions sommées, plafonnées à 15 pts
pub fn score_continu_pour_direction(
    bougies: &[Candle],
    direction: common::Direction,
    prix_actuel: f64,
) -> f64 {
    if prix_actuel <= 0.0 {
        return 0.0;
    }
    let zones = detecter(bougies, 5, true, false, true);
    let type_cible = match direction {
        common::Direction::Long => "FvgBull",
        common::Direction::Short => "FvgBear",
        common::Direction::Both => return 0.0,
    };
    let somme: f64 = zones
        .iter()
        .filter(|z| z.type_zone == type_cible)
        .map(|z| {
            let milieu = (z.haut + z.bas) / 2.0;
            let dist = ((prix_actuel - milieu).abs() / prix_actuel).min(1.0);
            // Contribution décroissante avec la distance (7.5 pts max par zone)
            7.5 / (1.0 + 20.0 * dist)
        })
        .sum();
    somme.min(15.0)
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
    fn detecter_vide_si_moins_de_3_bougies() {
        let bougies = vec![b(10., 12., 9., 11.), b(11., 13., 10., 12.)];
        assert!(detecter(&bougies, 5, true, true, false).is_empty());
    }

    #[test]
    fn detecter_fvg_bull() {
        // b0.high=10, b2.low=12 > b0.high=10 → FVG Bull attendu
        let bougies = vec![
            b(9., 10., 8., 9.5),    // b0 (i=0)
            b(10., 11., 9., 10.5),  // b1 (i=1), close > b0.high
            b(11., 13., 12., 12.5), // b2 (i=2), low=12 > b0.high=10
        ];
        let zones = detecter(&bougies, 5, true, false, false);
        assert!(
            zones.iter().any(|z| z.type_zone == "FvgBull"),
            "FVG Bull attendu"
        );
    }

    #[test]
    fn detecter_fvg_bear() {
        // b0.low=10, b2.high=8 < b0.low=10 → FVG Bear attendu
        let bougies = vec![
            b(11., 12., 10., 10.5), // b0
            b(10., 11., 9., 9.5),   // b1, close < b0.low
            b(9., 8., 7., 7.5),     // b2, high=8 < b0.low=10
        ];
        let zones = detecter(&bougies, 5, true, false, false);
        assert!(
            zones.iter().any(|z| z.type_zone == "FvgBear"),
            "FVG Bear attendu"
        );
    }

    #[test]
    fn detecter_og_bull() {
        // b2.low > b1.high → OG Bull
        let bougies = vec![
            b(10., 11., 9., 10.5),
            b(10., 11., 9., 10.),  // b1, high=11
            b(12., 14., 12., 13.), // b2, low=12 > b1.high=11
        ];
        let zones = detecter(&bougies, 5, false, true, false);
        assert!(
            zones.iter().any(|z| z.type_zone == "OgBull"),
            "OG Bull attendu"
        );
    }

    #[test]
    fn score_continu_zero_si_prix_nul() {
        let bougies = vec![
            b(9., 10., 8., 9.5),
            b(10., 11., 9., 10.5),
            b(11., 13., 12., 12.5),
        ];
        assert_eq!(
            score_continu_pour_direction(&bougies, common::Direction::Long, 0.0),
            0.0
        );
    }

    #[test]
    fn score_continu_diminue_avec_distance() {
        // FVG Bull entre 10 et 12 (milieu=11)
        let bougies = vec![
            b(9., 10., 8., 9.5),
            b(10., 11., 9., 10.5),
            b(11., 13., 12., 12.5),
        ];
        // Prix proche du milieu : score élevé
        let score_proche = score_continu_pour_direction(&bougies, common::Direction::Long, 11.0);
        // Prix loin (20% au-dessus) : score inférieur
        let score_loin = score_continu_pour_direction(&bougies, common::Direction::Long, 13.5);
        assert!(
            score_proche > score_loin,
            "score proche ({score_proche:.2}) doit être > score loin ({score_loin:.2})"
        );
    }

    #[test]
    fn score_continu_plafonne_a_15() {
        // Générer un grand nombre de zones bullish — score doit être ≤ 15
        let mut bougies = Vec::new();
        for i in 0..50i64 {
            let base = i as f64 * 5.0;
            bougies.push(b(base, base + 1.0, base - 1.0, base));
            bougies.push(b(base + 1.0, base + 2.0, base, base + 1.5));
            bougies.push(b(base + 1.5, base + 4.0, base + 2.5, base + 3.0));
        }
        let score = score_continu_pour_direction(&bougies, common::Direction::Long, 125.0);
        assert!(score <= 15.0, "Score {score:.2} doit être ≤ 15");
    }
}
