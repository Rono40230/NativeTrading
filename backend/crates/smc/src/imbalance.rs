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

/// Conservé pour compatibilité avec le scorer SMC (score 0–20 sur FVG alignés)
pub fn score_pour_direction_legacy(bougies: &[Candle], direction: common::Direction) -> f64 {
    let zones = detecter(bougies, 5, true, false, true);
    let aligne = match direction {
        common::Direction::Long => zones.iter().any(|z| z.type_zone == "FvgBull"),
        common::Direction::Short => zones.iter().any(|z| z.type_zone == "FvgBear"),
        common::Direction::Both => false,
    };
    if aligne {
        20.0
    } else {
        0.0
    }
}
