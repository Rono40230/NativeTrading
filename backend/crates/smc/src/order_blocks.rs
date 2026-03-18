use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

/// Nombre maximum d'OBs affichés (Pine : max_boxes_count=20 au total)
const MAX_OBS: usize = 20;
/// Distance minimale entre deux signaux ROC (Pine : cross_index - cross_index[1] > 5)
const MIN_SIGNAL_DISTANCE: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBlock {
    pub prix_haut: f64,
    pub prix_bas: f64,
    pub direction: Direction,
    pub force: f64,
    pub timestamp: i64,
}

/// Détecte les Order Blocks en simulant exactement Pine Script Sonarlab barre par barre.
///
/// - `sensibilite` : 1–100 (défaut 28), divisée par 100 → seuil ROC
/// - `mitigation_close` : true = close[1], false = wick (low/high courant)
pub fn detecter(bougies: &[Candle], sensibilite: f64, mitigation_close: bool) -> Vec<OrderBlock> {
    let n = bougies.len();
    if n < 20 {
        return vec![];
    }

    let seuil = sensibilite / 100.0;
    let vol_moy: f64 = bougies.iter().map(|b| b.volume).sum::<f64>() / n as f64;

    // Simulation des deux tableaux de boîtes Pine (longBoxes / shortBoxes)
    let mut obs_long: Vec<OrderBlock> = Vec::new();
    let mut obs_short: Vec<OrderBlock> = Vec::new();
    // Index de barre du dernier cross (bull ou bear) — Pine : cross_index partagé
    let mut dernier_cross_idx: Option<usize> = None;

    for i in 5..n {
        // --- Mitigation progressive (Pine l'exécute à chaque barre avant création) ---
        // Bull mitigation : close[1] < bot (Close) ou low < bot (Wick)
        let mitigation_bull = if mitigation_close {
            bougies[i - 1].close
        } else {
            bougies[i].low
        };
        // Bear mitigation : close[1] > top (Close) ou high > top (Wick)
        let mitigation_bear = if mitigation_close {
            bougies[i - 1].close
        } else {
            bougies[i].high
        };
        obs_long.retain(|ob| mitigation_bull >= ob.prix_bas);
        obs_short.retain(|ob| mitigation_bear <= ob.prix_haut);

        // --- Calcul ROC (Pine : pc = (open - open[4]) / open[4] * 100) ---
        let roc_curr = (bougies[i].open - bougies[i - 4].open)
            / bougies[i - 4].open.max(1e-10)
            * 100.0;
        let roc_prev = (bougies[i - 1].open - bougies[i - 5].open)
            / bougies[i - 5].open.max(1e-10)
            * 100.0;

        // Anti-spam : cross_index - cross_index[1] > 5 (Pine)
        let anti_spam_ok = dernier_cross_idx.is_none_or(|last| i - last > MIN_SIGNAL_DISTANCE);

        // Bearish crossunder → OB Short (1re bougie verte parmi les 4–15 précédentes)
        if roc_prev >= -seuil && roc_curr < -seuil && anti_spam_ok {
            dernier_cross_idx = Some(i);
            for offset in 4..=15_usize {
                if i < offset { break; }
                let b = &bougies[i - offset];
                if b.close > b.open {
                    let force = ((b.volume / vol_moy.max(1e-10)) * 50.0).min(100.0);
                    obs_short.push(OrderBlock {
                        prix_haut: b.high,
                        prix_bas: b.low,
                        direction: Direction::Short,
                        force,
                        timestamp: b.timestamp.timestamp(),
                    });
                    break;
                }
            }
        }

        // Bullish crossover → OB Long (1re bougie rouge parmi les 4–15 précédentes)
        if roc_prev <= seuil && roc_curr > seuil && anti_spam_ok {
            dernier_cross_idx = Some(i);
            for offset in 4..=15_usize {
                if i < offset { break; }
                let b = &bougies[i - offset];
                if b.close < b.open {
                    let force = ((b.volume / vol_moy.max(1e-10)) * 50.0).min(100.0);
                    obs_long.push(OrderBlock {
                        prix_haut: b.high,
                        prix_bas: b.low,
                        direction: Direction::Long,
                        force,
                        timestamp: b.timestamp.timestamp(),
                    });
                    break;
                }
            }
        }
    }

    // Combine, plus récents en premier, limite 20 au total (Pine : max_boxes_count=20)
    let mut result: Vec<OrderBlock> = obs_long
        .into_iter()
        .chain(obs_short)
        .collect();
    result.reverse();
    result.truncate(MAX_OBS);
    result
}

/// Score (0–100) de l'OB le plus fort aligné avec la direction donnée.
pub fn score_pour_direction(obs: &[OrderBlock], direction: Direction) -> f64 {
    obs.iter()
        .filter(|ob| ob.direction == direction)
        .map(|ob| ob.force)
        .fold(0.0f64, f64::max)
}
