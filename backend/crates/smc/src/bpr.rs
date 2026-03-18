use common::Candle;
use serde::{Deserialize, Serialize};

/// Zone FVG unifiée — reproduit le comportement de l'indicateur "IFVG/BPR Kasper Bootcamp".
///
/// `type_zone` :
/// - `"FvgBull"` : FVG haussier seul (gap up sans overlap) → affiché en bleu
/// - `"FvgBear"` : FVG baissier seul (gap down sans overlap) → affiché en rouge
/// - `"Bpr"`     : Overlap entre un FVG bull et un FVG bear → bleu + rouge superposés
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneFvgBpr {
    pub type_zone: String,
    /// Bornes de la zone principale (bull pour FvgBull, bear pour FvgBear, overlap pour Bpr)
    pub haut: f64,
    pub bas: f64,
    /// Bornes du FVG haussier (renseignées uniquement pour Bpr)
    pub bull_haut: f64,
    pub bull_bas: f64,
    /// Bornes du FVG baissier (renseignées uniquement pour Bpr)
    pub bear_haut: f64,
    pub bear_bas: f64,
    pub timestamp: i64,
}

// Redéfini pour compatibilité ascendante — garde Bpr comme alias de ZoneFvgBpr
pub type Bpr = ZoneFvgBpr;

struct Fvg {
    haut: f64,
    bas: f64,
    debut_idx: usize,
    timestamp: i64,
    /// true si ce FVG fait partie d'un BPR détecté
    en_bpr: bool,
}

/// Détecte les zones FVG unifiées (FVG bull, FVG bear, BPR).
///
/// - `show_last`       : nombre max de zones par type à retourner
/// - `atr_mult`        : taille minimum d'un FVG = ATR14 × atr_mult (défaut 0.5)
/// - `fenetre`         : distance max (bougies) entre bull et bear pour former un BPR (défaut 30)
/// - `mitigation_close`: `true` = mitigé sur clôture, `false` = mitigé dès qu'une mèche entre dans la zone
pub fn detecter(bougies: &[Candle], show_last: usize, atr_mult: f64, fenetre: usize, mitigation_close: bool) -> Vec<ZoneFvgBpr> {
    let n = bougies.len();
    if n < 10 {
        return vec![];
    }

    // ATR 14
    let n_atr = n.min(14);
    let atr14 = bougies[n - n_atr..]
        .iter()
        .map(|b| b.high - b.low)
        .sum::<f64>()
        / n_atr as f64;
    let seuil_min = (atr14 * atr_mult).max(1e-10);

    let debut = n.saturating_sub(200);
    let prix_ref = bougies.last().map(|b| b.close).unwrap_or(1.0);
    let tolerance = prix_ref * 0.001;

    // ── Collecte FVGs bull et bear ───────────────────────────────────────────
    let mut bulls: Vec<Fvg> = Vec::new();
    let mut bears: Vec<Fvg> = Vec::new();

    for i in debut..n.saturating_sub(2) {
        let gauche = &bougies[i];
        let droite = &bougies[i + 2];

        if droite.low > gauche.high && (droite.low - gauche.high) >= seuil_min {
            // Bull FVG : zone [gauche.high (bas) .. droite.low (haut)]
            // Mitigation : une bougie ultérieure dont le range CHEVAUCHE la zone
            //   (garde : si b.high < gauche.high, la bougie est entièrement sous la zone → pas de test)
            let fvg_bas = gauche.high;
            let fvg_haut = droite.low;
            let mitige = bougies[i + 3..].iter().any(|b| {
                if b.high < fvg_bas { return false; } // candle entièrement sous la zone
                if mitigation_close {
                    b.close <= fvg_haut // clôture entre dans la zone par le haut
                } else {
                    b.low <= fvg_haut // mèche basse touche le haut de la zone
                }
            });
            if !mitige {
                bulls.push(Fvg {
                    haut: fvg_haut,
                    bas: fvg_bas,
                    debut_idx: i,
                    timestamp: gauche.timestamp.timestamp(),
                    en_bpr: false,
                });
            }
        }

        if gauche.low > droite.high && (gauche.low - droite.high) >= seuil_min {
            // Bear FVG : zone [droite.high (bas) .. gauche.low (haut)]
            // Mitigation : une bougie ultérieure dont le range CHEVAUCHE la zone
            //   (garde : si b.low > gauche.low, la bougie est entièrement au-dessus → pas de test)
            let fvg_bas = droite.high;
            let fvg_haut = gauche.low;
            let mitige = bougies[i + 3..].iter().any(|b| {
                if b.low > fvg_haut { return false; } // candle entièrement au-dessus de la zone
                if mitigation_close {
                    b.close >= fvg_bas // clôture entre dans la zone par le bas
                } else {
                    b.high >= fvg_bas // mèche haute touche le bas de la zone
                }
            });
            if !mitige {
                bears.push(Fvg {
                    haut: fvg_haut,
                    bas: fvg_bas,
                    debut_idx: i,
                    timestamp: gauche.timestamp.timestamp(),
                    en_bpr: false,
                });
            }
        }
    }

    // ── Détecte les BPRs (overlaps) ──────────────────────────────────────────
    let mut bprs: Vec<ZoneFvgBpr> = Vec::new();

    for bull in bulls.iter_mut() {
        for bear in bears.iter_mut() {
            let dist = bull.debut_idx.abs_diff(bear.debut_idx);
            if dist > fenetre {
                continue;
            }

            let overlap_haut = bull.haut.min(bear.haut);
            let overlap_bas = bull.bas.max(bear.bas);
            if overlap_haut <= overlap_bas {
                continue;
            }

            // Déduplique — même overlap déjà enregistré ?
            let doublon = bprs.iter().any(|z| {
                (z.haut - overlap_haut).abs() < tolerance
                    && (z.bas - overlap_bas).abs() < tolerance
            });
            if !doublon {
                bprs.push(ZoneFvgBpr {
                    type_zone: "Bpr".into(),
                    haut: overlap_haut,
                    bas: overlap_bas,
                    bull_haut: bull.haut,
                    bull_bas: bull.bas,
                    bear_haut: bear.haut,
                    bear_bas: bear.bas,
                    timestamp: bull.timestamp.min(bear.timestamp),
                });
            }

            bull.en_bpr = true;
            bear.en_bpr = true;
        }
    }

    // ── Assemble et trie par proximité au prix actuel ─────────────────────────
    let prix_actuel = bougies.last().map(|b| b.close).unwrap_or(0.0);
    let milieu = |z: &ZoneFvgBpr| -> f64 { (z.haut + z.bas) / 2.0 };
    let dist = |z: &ZoneFvgBpr| -> f64 { (milieu(z) - prix_actuel).abs() };

    bprs.sort_by(|a, b| dist(a).partial_cmp(&dist(b)).unwrap_or(std::cmp::Ordering::Equal));
    bprs.truncate(show_last);
    bprs
}
