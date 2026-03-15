use common::Candle;
use serde::{Deserialize, Serialize};

/// Tolérance en % pour détecter les equal highs/lows (0.1% = 10 pips sur XAUUSD)
const TOLERANCE_PCT: f64 = 0.001;
/// Nombre de bougies de chaque côté pour valider un pivot
const LOOKBACK: usize = 3;

/// Zone de liquidité (BSL ou SSL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NiveauLiquidite {
    /// Prix du niveau (borne haute pour BSL, borne basse pour SSL)
    pub prix: f64,
    /// "BSL" (Buy Side) ou "SSL" (Sell Side)
    pub cote: String,
    /// True = equal highs/lows (piège retail visible), False = swing isolé
    pub equal: bool,
    /// True = le niveau a déjà été sweepé (liquidité prise)
    pub sweepé: bool,
}

/// Détecte les niveaux de liquidité BSL et SSL sur les N dernières bougies.
///
/// BSL (Buy Side Liquidity) = highs récents où se concentrent les stops vendeurs
///   → equal highs et swing highs locaux
/// SSL (Sell Side Liquidity) = lows récents où se concentrent les stops acheteurs
///   → equal lows et swing lows locaux
pub fn detecter(bougies: &[Candle]) -> Vec<NiveauLiquidite> {
    if bougies.len() < LOOKBACK * 2 + 1 {
        return Vec::new();
    }

    let prix_actuel = bougies.last().map(|b| b.close).unwrap_or(0.0);
    let mut niveaux: Vec<NiveauLiquidite> = Vec::new();

    for i in LOOKBACK..bougies.len().saturating_sub(LOOKBACK) {
        let b = &bougies[i];

        // ── BSL : swing high local ──────────────────────────────────────────
        let est_swing_high = bougies[i - LOOKBACK..i].iter().all(|x| x.high <= b.high)
            && bougies[i + 1..=i + LOOKBACK].iter().all(|x| x.high <= b.high);

        if est_swing_high {
            // Chercher un equal high dans la fenêtre précédente
            let equal = bougies[i.saturating_sub(20)..i].iter().any(|x| {
                (x.high - b.high).abs() / b.high.max(f64::EPSILON) <= TOLERANCE_PCT
            });
            // Sweepé si le prix a cassé au-dessus après
            let sweepé = bougies[i + 1..].iter().any(|x| x.high > b.high * (1.0 + TOLERANCE_PCT));
            niveaux.push(NiveauLiquidite {
                prix: b.high,
                cote: "BSL".to_string(),
                equal,
                sweepé,
            });
        }

        // ── SSL : swing low local ───────────────────────────────────────────
        let est_swing_low = bougies[i - LOOKBACK..i].iter().all(|x| x.low >= b.low)
            && bougies[i + 1..=i + LOOKBACK].iter().all(|x| x.low >= b.low);

        if est_swing_low {
            let equal = bougies[i.saturating_sub(20)..i].iter().any(|x| {
                (x.low - b.low).abs() / b.low.max(f64::EPSILON) <= TOLERANCE_PCT
            });
            let sweepé = bougies[i + 1..].iter().any(|x| x.low < b.low * (1.0 - TOLERANCE_PCT));
            niveaux.push(NiveauLiquidite {
                prix: b.low,
                cote: "SSL".to_string(),
                equal,
                sweepé,
            });
        }
    }

    // Garder les 10 niveaux les plus proches du prix actuel (non sweepés en priorité)
    niveaux.sort_by(|a, b| {
        let da = (a.prix - prix_actuel).abs();
        let db = (b.prix - prix_actuel).abs();
        let score_a = if a.sweepé { da * 2.0 } else { da };
        let score_b = if b.sweepé { db * 2.0 } else { db };
        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    niveaux.dedup_by(|a, b| {
        (a.prix - b.prix).abs() / b.prix.max(f64::EPSILON) < TOLERANCE_PCT * 2.0
    });
    niveaux.truncate(10);
    niveaux
}
