//! R pondéré SMC — la couche comptable des ventes partielles (unipanel).
//!
//! Le moteur reste Pine-fidèle (R réalisé unitaire) ; cette fonction pure
//! retraduit chaque verdict en R réellement encaissé quand le lot est coupé
//! en trois (f1 vendue à TP1, f2 à TP2, f3 = solde). Cantonnement acté :
//! le pondéré alimente UNIQUEMENT la simulation de capital — Σ R de
//! référence, WR et historique restent au moteur.

/// Fractions du lot vendues à chaque palier (0..=1, Σ = 1).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Fractions {
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: f64,
}

impl Default for Fractions {
    fn default() -> Self {
        Self { tp1: 0.5, tp2: 0.3, tp3: 0.2 }
    }
}

/// R pondéré d'une clôture.
///
/// * `verdict` — chaîne canonique du moteur (TP3 / TS / TP2+BE / TP1+BE / SL / BE / Expire).
/// * `r_realise` — R unitaire du moteur (distance réelle de la sortie).
/// * `r_tp1`, `r_tp2` — distances réelles des paliers en R (TP1/TP2 réglables).
///
/// Sémantique (tableau de la page Caractéristiques) :
/// - TP3 → f1·tp1 + f2·tp2 + f3·r3 (tout est vendu, le solde à la cible) ;
/// - TS  → f1·tp1 + f2·tp2 + f3·r_ts (le solde sort au stop suivi — réel) ;
/// - TP2+BE → f1·tp1 + f2·tp2 + f3·0 (le solde sort à l'entrée) ;
/// - TP1+BE → f1·tp1 (le reste sort à l'entrée) ;
/// - SL → −1R (lot entier), BE forcé → 0, Expire → 0 (aucun palier touché :
///   une expiration après TP1 porte le verdict TP1, pas Expire).
pub fn r_pondere(verdict: &str, r_realise: f64, r_tp1: f64, r_tp2: f64, f: Fractions) -> f64 {
    let v = verdict.to_lowercase();
    match v.as_str() {
        "tp3" => f.tp1 * r_tp1 + f.tp2 * r_tp2 + f.tp3 * r_realise,
        "ts" => f.tp1 * r_tp1 + f.tp2 * r_tp2 + f.tp3 * r_realise,
        "tp2" | "tp2+be" => f.tp1 * r_tp1 + f.tp2 * r_tp2,
        "tp1" | "tp1+be" => f.tp1 * r_tp1,
        "sl" | "sl+be" => -1.0,
        // BE forcé (aucun palier touché) ou expiration pure.
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: Fractions = Fractions { tp1: 0.5, tp2: 0.3, tp3: 0.2 };

    #[test]
    fn tp3_aux_defauts() {
        // 0.5×0.6 + 0.3×2 + 0.2×3 = 1.50R (corrigé : le tableau documenté
        // disait 1.80R par erreur d'arithmétique).
        assert!((r_pondere("TP3", 3.0, 0.6, 2.0, F) - 1.50).abs() < 1e-9);
    }

    #[test]
    fn tp2_be_aux_defauts() {
        // 0.5×0.6 + 0.3×2 = 0.90R.
        assert!((r_pondere("TP2+BE", 2.0, 0.6, 2.0, F) - 0.90).abs() < 1e-9);
    }

    #[test]
    fn tp1_be_aux_defauts() {
        assert!((r_pondere("TP1+BE", 0.6, 0.6, 2.0, F) - 0.30).abs() < 1e-9);
    }

    #[test]
    fn ts_solde_au_reel() {
        // 0.5×0.6 + 0.3×2 + 0.2×3.25 = 1.55R (exemple du propriétaire).
        assert!((r_pondere("TS", 3.25, 0.6, 2.0, F) - 1.55).abs() < 1e-9);
    }

    #[test]
    fn sl_be_expire() {
        assert_eq!(r_pondere("SL", -1.0, 0.6, 2.0, F), -1.0);
        assert_eq!(r_pondere("BE", 0.0, 0.6, 2.0, F), 0.0);
        assert_eq!(r_pondere("Expire", 0.0, 0.6, 2.0, F), 0.0);
    }
}
