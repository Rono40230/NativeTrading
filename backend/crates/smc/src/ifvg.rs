use common::{Candle, Direction};

#[derive(Debug, Clone)]
pub struct Ifvg {
    pub prix_haut: f64,
    pub prix_bas: f64,
    /// Direction après inversion (BOS confirmé)
    pub direction: Direction,
}

/// Détecte les IFVG (Inversion Fair Value Gap).
///
/// Processus :
/// 1. Un FVG est formé (gap 3 bougies).
/// 2. Le prix entre dans la zone (mitigation).
/// 3. Un BOS (Break of Structure) dans la direction opposée confirme l'inversion.
pub fn detecter(bougies: &[Candle]) -> Vec<Ifvg> {
    if bougies.len() < 10 {
        return vec![];
    }

    let prix_ref = bougies.last().map(|b| b.close).unwrap_or(1.0);
    let seuil_min = prix_ref * 0.0003;
    let mut ifvgs: Vec<Ifvg> = Vec::new();
    let debut = bougies.len().saturating_sub(80);

    for i in debut..bougies.len().saturating_sub(5) {
        let gauche = &bougies[i];
        let droite = &bougies[i + 2];
        let suite = &bougies[i + 2..];

        // FVG Bullish → devient IFVG baissier si prix entre dans le gap puis BOS baissier
        if droite.low > gauche.high && (droite.low - gauche.high) >= seuil_min {
            let fvg_bas = gauche.high;
            let fvg_haut = droite.low;
            let mitigation = suite.iter().any(|b| b.low <= fvg_haut && b.high >= fvg_bas);
            if mitigation {
                let bos = suite.iter().any(|b| b.close < fvg_bas);
                if bos {
                    ifvgs.push(Ifvg {
                        prix_haut: fvg_haut,
                        prix_bas: fvg_bas,
                        direction: Direction::Short,
                    });
                }
            }
        }

        // FVG Bearish → devient IFVG haussier si prix entre dans le gap puis BOS haussier
        if gauche.low > droite.high && (gauche.low - droite.high) >= seuil_min {
            let fvg_bas = droite.high;
            let fvg_haut = gauche.low;
            let mitigation = suite.iter().any(|b| b.high >= fvg_bas && b.low <= fvg_haut);
            if mitigation {
                let bos = suite.iter().any(|b| b.close > fvg_haut);
                if bos {
                    ifvgs.push(Ifvg {
                        prix_haut: fvg_haut,
                        prix_bas: fvg_bas,
                        direction: Direction::Long,
                    });
                }
            }
        }
    }

    ifvgs.reverse();
    ifvgs.truncate(3);
    ifvgs
}

/// Score (0 ou 15) en fonction de la présence d'un IFVG aligné avec la direction.
pub fn score_pour_direction(ifvgs: &[Ifvg], direction: Direction) -> f64 {
    if ifvgs.iter().any(|fg| fg.direction == direction) {
        15.0
    } else {
        0.0
    }
}
