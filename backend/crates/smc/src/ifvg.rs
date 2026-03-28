use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ifvg {
    pub prix_haut: f64,
    pub prix_bas: f64,
    /// Direction de l'IFVG APRÈS inversion
    /// Short = ex-bull FVG inversé | Long = ex-bear FVG inversé
    pub direction: Direction,
    /// Timestamp de la bougie milieu du FVG (bord GAUCHE de la zone)
    pub timestamp: i64,
    /// Timestamp de la bougie où le corps a causé l'inversion (séparation gauche/droite)
    pub timestamp_inversion: i64,
}

// ─── Structures internes ───────────────────────────────────────────────────────

struct FvgEntry {
    top: f64,
    bot: f64,
    left_ts: i64, // timestamp de la bougie milieu (time[1] en Pine)
}

struct IfvgEntry {
    top: f64,
    bot: f64,
    left_ts: i64,
    inv_ts: i64,
    state: u8, // 0 = premier tick après inversion, 1+ = actif
}

/// Détecte les IFVG selon l'algorithme exact LuxAlgo (Pine Script).
///
/// Logique :
/// 1. FVG détecté : `low[0] > high[2] && close[1] > high[2]` (bull) ou inverse (bear)
/// 2. Inversion quand le **corps** (`max/min(open,close)`) croise la zone
/// 3. Mitigation finale si le corps dépasse entièrement la zone
/// 4. Affichage : 2 rectangles par IFVG (avant/après inversion)
pub fn detecter(
    bougies: &[Candle],
    show_last: usize,
    _signal_pref_close: bool,
    atr_mult: f64,
) -> Vec<Ifvg> {
    let n = bougies.len();
    if n < 5 {
        return vec![];
    }

    // ATR 200 (comme Pine : `ta.atr(200)`)
    let n_atr = n.min(200);
    let atr = bougies[n - n_atr..]
        .iter()
        .map(|b| b.high - b.low)
        .sum::<f64>()
        / n_atr as f64;
    let atr_seuil = (atr * atr_mult).max(1e-10);

    let mut bull_fvg: Vec<FvgEntry> = Vec::new();
    let mut bear_fvg: Vec<FvgEntry> = Vec::new();
    let mut bull_inv: Vec<IfvgEntry> = Vec::new(); // ex-bull FVGs inversés → dir final Short
    let mut bear_inv: Vec<IfvgEntry> = Vec::new(); // ex-bear FVGs inversés → dir final Long

    // Parcours chronologique — simule l'exécution barre à barre de Pine
    for i in 2..n {
        let b0 = &bougies[i]; // bar[0] en Pine
        let b1 = &bougies[i - 1]; // bar[1]
        let b2 = &bougies[i - 2]; // bar[2]

        let ts = b0.timestamp.timestamp();
        let c_top = b0.open.max(b0.close); // haut du corps (sans mèche)
        let c_bot = b0.open.min(b0.close); // bas du corps (sans mèche)

        // ── 1. Détection FVG (Pine: push avant fvg_manage) ───────────────────
        // Bull FVG : low[0] > high[2] && close[1] > high[2]
        if b0.low > b2.high && b1.close > b2.high && (b0.low - b2.high) > atr_seuil {
            if bull_fvg.len() >= 100 {
                bull_fvg.remove(0);
            }
            bull_fvg.push(FvgEntry {
                top: b0.low,
                bot: b2.high,
                left_ts: b1.timestamp.timestamp(),
            });
        }
        // Bear FVG : high[0] < low[2] && close[1] < low[2]
        if b0.high < b2.low && b1.close < b2.low && (b2.low - b0.high) > atr_seuil {
            if bear_fvg.len() >= 100 {
                bear_fvg.remove(0);
            }
            bear_fvg.push(FvgEntry {
                top: b2.low,
                bot: b0.high,
                left_ts: b1.timestamp.timestamp(),
            });
        }

        // ── 2. fvg_manage → inversion des FVGs en attente ────────────────────
        // Bull FVG : inversé si le corps descend sous le bas (c_bot < bot)
        let mut keep: Vec<FvgEntry> = Vec::new();
        for fvg in bull_fvg.drain(..) {
            if c_bot < fvg.bot {
                if bull_inv.len() >= 100 {
                    bull_inv.remove(0);
                }
                bull_inv.push(IfvgEntry {
                    top: fvg.top,
                    bot: fvg.bot,
                    left_ts: fvg.left_ts,
                    inv_ts: ts,
                    state: 0,
                });
            } else {
                keep.push(fvg);
            }
        }
        bull_fvg = keep;

        // Bear FVG : inversé si le corps monte au-dessus du haut (c_top > top)
        let mut keep: Vec<FvgEntry> = Vec::new();
        for fvg in bear_fvg.drain(..) {
            if c_top > fvg.top {
                if bear_inv.len() >= 100 {
                    bear_inv.remove(0);
                }
                bear_inv.push(IfvgEntry {
                    top: fvg.top,
                    bot: fvg.bot,
                    left_ts: fvg.left_ts,
                    inv_ts: ts,
                    state: 0,
                });
            } else {
                keep.push(fvg);
            }
        }
        bear_fvg = keep;

        // ── 3. inv_manage → changement de state + mitigation ─────────────────
        // bull_inv : état 0 → dir final Short ; mitigé si c_top > top
        bull_inv.retain_mut(|inv| {
            if inv.state == 0 {
                inv.state = 1;
            }
            // dir final Short (ex-bull) : mitigé si c_top > top
            c_top <= inv.top
        });

        // bear_inv : état 0 → dir final Long ; mitigé si c_bot < bot
        bear_inv.retain_mut(|inv| {
            if inv.state == 0 {
                inv.state = 1;
            }
            // dir final Long (ex-bear) : mitigé si c_bot < bot
            c_bot >= inv.bot
        });
    }

    // ── Assemblage final — show_last derniers de chaque catégorie ─────────────
    let mut result: Vec<Ifvg> = Vec::new();

    let take_bull = show_last.min(bull_inv.len());
    for inv in &bull_inv[bull_inv.len() - take_bull..] {
        result.push(Ifvg {
            prix_haut: inv.top,
            prix_bas: inv.bot,
            direction: Direction::Short,
            timestamp: inv.left_ts,
            timestamp_inversion: inv.inv_ts,
        });
    }

    let take_bear = show_last.min(bear_inv.len());
    for inv in &bear_inv[bear_inv.len() - take_bear..] {
        result.push(Ifvg {
            prix_haut: inv.top,
            prix_bas: inv.bot,
            direction: Direction::Long,
            timestamp: inv.left_ts,
            timestamp_inversion: inv.inv_ts,
        });
    }

    result
}

/// Score (0 ou 15) en fonction de la présence d'un IFVG aligné avec la direction.
pub fn score_pour_direction(ifvgs: &[Ifvg], direction: Direction) -> f64 {
    if ifvgs.iter().any(|fg| fg.direction == direction) {
        15.0
    } else {
        0.0
    }
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
    fn detecter_vide_si_moins_de_5_bougies() {
        let bougies: Vec<Candle> = (0..4)
            .map(|i| {
                b(
                    i as f64 + 10.,
                    i as f64 + 11.,
                    i as f64 + 9.,
                    i as f64 + 10.5,
                )
            })
            .collect();
        assert!(
            detecter(&bougies, 5, false, 1.0).is_empty(),
            "Moins de 5 bougies → vide"
        );
    }

    #[test]
    fn score_pour_direction_zero_si_pas_ifvg() {
        let ifvgs: Vec<Ifvg> = vec![];
        assert_eq!(score_pour_direction(&ifvgs, Direction::Long), 0.0);
    }

    #[test]
    fn score_pour_direction_15_si_ifvg_aligne() {
        let ifvg = Ifvg {
            prix_haut: 110.,
            prix_bas: 100.,
            direction: Direction::Long,
            timestamp: 0,
            timestamp_inversion: 0,
        };
        assert_eq!(score_pour_direction(&[ifvg], Direction::Long), 15.0);
    }
}
