use common::{Candle, Direction};

#[derive(Debug, Clone)]
pub struct Imbalance {
    /// Borne haute du gap
    pub prix_haut: f64,
    /// Borne basse du gap
    pub prix_bas: f64,
    pub direction: Direction,
    /// Vrai si le gap a été totalement comblé
    pub comble: bool,
}

/// Détecte les zones d'imbalance (Fair Value Gap, 3-bougie pattern).
///
/// Bullish FVG : bougies[i].high < bougies[i+2].low  (gap haussier)
/// Bearish FVG : bougies[i].low  > bougies[i+2].high (gap baissière)
pub fn detecter(bougies: &[Candle]) -> Vec<Imbalance> {
    if bougies.len() < 3 {
        return vec![];
    }

    let prix_ref = bougies.last().map(|b| b.close).unwrap_or(1.0);
    // Seuil minimum : ~3 pips (0.03% du prix)
    let seuil_min = prix_ref * 0.0003;

    let mut imbalances: Vec<Imbalance> = Vec::new();
    let debut = bougies.len().saturating_sub(100);

    for i in debut..bougies.len().saturating_sub(2) {
        let gauche = &bougies[i];
        let droite = &bougies[i + 2];

        // Bullish FVG
        if droite.low > gauche.high && (droite.low - gauche.high) >= seuil_min {
            let comble = bougies[i + 2..]
                .iter()
                .any(|b| b.low <= gauche.high);
            imbalances.push(Imbalance {
                prix_haut: droite.low,
                prix_bas: gauche.high,
                direction: Direction::Long,
                comble,
            });
        }

        // Bearish FVG
        if gauche.low > droite.high && (gauche.low - droite.high) >= seuil_min {
            let comble = bougies[i + 2..]
                .iter()
                .any(|b| b.high >= gauche.low);
            imbalances.push(Imbalance {
                prix_haut: gauche.low,
                prix_bas: droite.high,
                direction: Direction::Short,
                comble,
            });
        }
    }

    // Conserver uniquement les non comblés, 5 plus récents
    imbalances.retain(|im| !im.comble);
    imbalances.reverse();
    imbalances.truncate(5);
    imbalances
}

/// Score (0 ou 20) en fonction de la présence d'un gap aligné avec la direction.
pub fn score_pour_direction(imbs: &[Imbalance], direction: Direction) -> f64 {
    if imbs.iter().any(|im| im.direction == direction) { 20.0 } else { 0.0 }
}
