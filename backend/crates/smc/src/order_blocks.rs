use common::{Candle, Direction};

#[derive(Debug, Clone)]
pub struct OrderBlock {
    /// Borne haute de la zone OB
    pub prix_haut: f64,
    /// Borne basse de la zone OB
    pub prix_bas: f64,
    /// Long = OB haussier (support), Short = OB baissier (résistance)
    pub direction: Direction,
    /// Force relative (0–100) basée sur le volume
    pub force: f64,
}

/// Détecte les Order Blocks récents sur les 50 dernières bougies.
///
/// Bullish OB : dernière bougie bearish avant une impulsion haussiere forte.
/// Bearish OB : dernière bougie bullish avant une impulsion baissière forte.
pub fn detecter(bougies: &[Candle]) -> Vec<OrderBlock> {
    if bougies.len() < 5 {
        return vec![];
    }

    let vol_moy: f64 = bougies.iter().map(|b| b.volume).sum::<f64>() / bougies.len() as f64;

    let mut obs: Vec<OrderBlock> = Vec::new();
    let debut = bougies.len().saturating_sub(50);

    for i in debut..bougies.len().saturating_sub(2) {
        let actuelle = &bougies[i];
        let apres = &bougies[i + 2];
        let corps = (actuelle.close - actuelle.open).abs();
        let force_vol = ((actuelle.volume / vol_moy.max(1e-10)) * 50.0).min(100.0);

        // BULLISH OB : bougie bearish (close < open)
        // + impulsion haussiere forte au plus tard 2 bougies après
        let impulsion_haussiere =
            apres.close > actuelle.high && (apres.close - apres.open).abs() > corps * 0.6;
        if actuelle.close < actuelle.open && impulsion_haussiere {
            obs.push(OrderBlock {
                prix_haut: actuelle.high,
                prix_bas: actuelle.open.min(actuelle.close),
                direction: Direction::Long,
                force: force_vol,
            });
        }

        // BEARISH OB : bougie bullish (close > open)
        // + impulsion baissière forte au plus tard 2 bougies après
        let impulsion_baissiere =
            apres.close < actuelle.low && (apres.open - apres.close).abs() > corps * 0.6;
        if actuelle.close > actuelle.open && impulsion_baissiere {
            obs.push(OrderBlock {
                prix_haut: actuelle.open.max(actuelle.close),
                prix_bas: actuelle.low,
                direction: Direction::Short,
                force: force_vol,
            });
        }
    }

    // Garder les 3 plus récents (ordre décroissant d'index = plus récent en premier)
    obs.reverse();
    obs.truncate(3);
    obs
}

/// Retourne le score (0–100) de l'OB le plus fort aligné avec la direction donnée.
pub fn score_pour_direction(obs: &[OrderBlock], direction: Direction) -> f64 {
    obs.iter()
        .filter(|ob| ob.direction == direction)
        .map(|ob| ob.force)
        .fold(0.0f64, f64::max)
}
