//! Calculs de gestion de position pour la stratégie Rockets.
//! Séparé de rockets_indicateurs.rs pour respecter la limite des 300 lignes.

use db::rockets_config::RocketsConfig;

/// Calcule le coefficient de trailing stop dynamique selon le momentum du signal.
///
/// `score`     : score du signal (0–100)
/// `atr_ratio` : ratio ATR5/ATR14 (>1.0 = volatilité en expansion)
///
/// Retourne le multiplicateur ATR appliqué au peak pour poser le trailing stop :
/// `trailing_stop = peak - ATR14 × coefficient`
///
/// Plus le coefficient est élevé, plus le stop est lâche (laisse courir le prix).
pub fn calculer_trailing_coeff(score: i64, atr_ratio: f64, cfg: &RocketsConfig) -> f64 {
    let base: f64 = 2.5;

    // Facteur momentum : signal explosif → stop large pour capter le move
    let score_factor = if score > 80 {
        1.5 // Explosif
    } else if score > 60 {
        1.2 // Fort
    } else if score > 40 {
        1.0 // Modéré
    } else {
        0.8 // Faible → stop serré
    };

    // Bonus volatilité : crypto en accélération forte
    let vol_factor = if atr_ratio > 1.5 { 1.2 } else { 1.0 };

    (base * score_factor * vol_factor).clamp(cfg.trailing_coeff_min, cfg.trailing_coeff_max)
}

/// Calcule les pourcentages de vente partielle selon le score du signal.
///
/// Retourne `(pct_tp1, pct_tp2, pct_trailing)` — les trois somment à 1.0.
///
/// | Score        | TP1  | TP2  | Trailing |
/// |--------------|------|------|----------|
/// | < faible     | 40%  | 35%  | 25%      |
/// | faible–fort  | 25%  | 25%  | 50%      |
/// | ≥ fort       | 15%  | 20%  | 65%      |
///
/// Logique : plus le signal est fort, plus on laisse en trailing pour capter le move explosif.
pub fn calculer_split_vente(score: i64, cfg: &RocketsConfig) -> (f64, f64, f64) {
    if score < cfg.seuil_score_faible {
        (0.40, 0.35, 0.25)
    } else if score < cfg.seuil_score_fort {
        (0.25, 0.25, 0.50)
    } else {
        (0.15, 0.20, 0.65)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::rockets_config::RocketsConfig;

    fn cfg() -> RocketsConfig {
        RocketsConfig::default()
    }

    #[test]
    fn trailing_coeff_explosif() {
        let c = calculer_trailing_coeff(85, 1.6, &cfg());
        // base 2.5 × 1.5 × 1.2 = 4.5, clamp(1.5, 5.0) = 4.5
        assert!((c - 4.5).abs() < 0.01);
    }

    #[test]
    fn trailing_coeff_faible() {
        let c = calculer_trailing_coeff(30, 0.9, &cfg());
        // base 2.5 × 0.8 × 1.0 = 2.0, clamp(1.5, 5.0) = 2.0
        assert!((c - 2.0).abs() < 0.01);
    }

    #[test]
    fn split_score_fort() {
        let (tp1, tp2, trailing) = calculer_split_vente(70, &cfg());
        assert!((tp1 - 0.25).abs() < 0.001);
        assert!((tp2 - 0.25).abs() < 0.001);
        assert!((trailing - 0.50).abs() < 0.001);
    }

    #[test]
    fn split_score_explosif() {
        let (tp1, tp2, trailing) = calculer_split_vente(90, &cfg());
        assert!((tp1 - 0.15).abs() < 0.001);
        assert!((tp2 - 0.20).abs() < 0.001);
        assert!((trailing - 0.65).abs() < 0.001);
    }
}
