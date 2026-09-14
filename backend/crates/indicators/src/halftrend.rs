use common::Candle;

use crate::atr::calculer_atr;

/// HalfTrend — transposition exacte de l'étalon KDJ/Halftrend
/// (docs/reference/definition_kdj_halftrend.md §2.3).
/// channelDeviation = 2 et ATR(100)/2 sont codés en dur, comme dans le Pine.
#[derive(Debug, Clone)]
pub struct Halftrend {
    /// 0 = haussier, 1 = baissier (convention Pine de l'étalon).
    pub trend: Vec<i32>,
    /// Ligne HalfTrend (up en trend 0, down en trend 1). NaN avant initialisation.
    pub ht: Vec<f64>,
    pub up: Vec<f64>,
    pub down: Vec<f64>,
    /// Flèche acheteuse à CETTE barre : retournement 1→0 avec ATR prêt.
    pub arrow_up: Vec<bool>,
    /// Flèche vendeuse à CETTE barre : retournement 0→1 avec ATR prêt.
    pub arrow_down: Vec<bool>,
}

/// machine d'état + lignes, barre par barre, sur valeurs clôturées (piège 2 :
/// aucune évaluation intrabar — décision à la clôture seule).
pub fn calculer_halftrend(bougies: &[Candle], amplitude: usize) -> Halftrend {
    let n = bougies.len();
    let mut out = Halftrend {
        trend: vec![0; n],
        ht: vec![f64::NAN; n],
        up: vec![f64::NAN; n],
        down: vec![f64::NAN; n],
        arrow_up: vec![false; n],
        arrow_down: vec![false; n],
    };
    if n == 0 {
        return out;
    }

    // atr2 = atr(100)/2 — Wilder, NaN avant 100 bougies (convention TV).
    let atr: Vec<f64> = calculer_atr(bougies, 100)
        .iter()
        .map(|v| v / 2.0)
        .collect();

    // État persistant (var Pine) : maxLowPrice/minHighPrice seedés à la barre 0
    // (nz(low[1], low) → low de la barre courante).
    let mut next_trend = 0i32;
    let mut max_low_price = bougies[0].low;
    let mut min_high_price = bougies[0].high;
    let mut up = f64::NAN;
    let mut down = f64::NAN;

    for i in 0..n {
        // highPrice/lowPrice = extrêmes des `amplitude` dernières barres
        // (= high[highestbars(amplitude)] : la VALEUR, l'offset ne compte pas).
        let (high_price, low_price) = extremum(bougies, i, amplitude);
        let highma = moyenne(bougies, i, amplitude, |b| b.high);
        let lowma = moyenne(bougies, i, amplitude, |b| b.low);

        // trend garde sa valeur précédente (var) ; barre 0 → init 0.
        out.trend[i] = if i == 0 { 0 } else { out.trend[i - 1] };
        // À la barre 0, nz(low[1], low) = low[0] : close < low[0] est faux —
        // aucune bascule possible à la première barre.
        if next_trend == 1 {
            max_low_price = max_low_price.max(low_price);
            if i >= 1
                && highma.is_finite()
                && highma < max_low_price
                && bougies[i].close < bougies[i - 1].low
            {
                out.trend[i] = 1;
                next_trend = 0;
                min_high_price = high_price;
            }
        } else {
            min_high_price = min_high_price.min(high_price);
            if i >= 1
                && lowma.is_finite()
                && lowma > min_high_price
                && bougies[i].close > bougies[i - 1].high
            {
                out.trend[i] = 0;
                next_trend = 1;
                max_low_price = low_price;
            }
        }

        // Lignes up/down — la flèche n'existe qu'à la barre de retournement
        // ET si atr2 est prêt (sinon arrowUp = na dans l'étalon → pas de flèche).
        if out.trend[i] == 0 {
            if i >= 1 && out.trend[i - 1] != 0 {
                // na(down[1]) ? down : down[1] — la var vaut déjà down[1].
                up = down;
                out.arrow_up[i] = atr[i].is_finite() && up.is_finite();
            } else {
                up = if i == 0 || out.up[i - 1].is_nan() {
                    max_low_price
                } else {
                    max_low_price.max(out.up[i - 1])
                };
            }
            out.ht[i] = up;
        } else {
            if i >= 1 && out.trend[i - 1] != 1 {
                down = up;
                out.arrow_down[i] = atr[i].is_finite() && down.is_finite();
            } else {
                down = if i == 0 || out.down[i - 1].is_nan() {
                    min_high_price
                } else {
                    min_high_price.min(out.down[i - 1])
                };
            }
            out.ht[i] = down;
        }
        out.up[i] = up;
        out.down[i] = down;
    }
    out
}

/// Extremum des `amplitude` dernières bougies (borné aux bougies existantes ;
/// `amplitude` ≤ 1 → bougie courante via saturating).
fn extremum(bougies: &[Candle], i: usize, amplitude: usize) -> (f64, f64) {
    let debut = i.saturating_sub(amplitude.saturating_sub(1));
    let fen = &bougies[debut..=i];
    let hh = fen.iter().fold(f64::NEG_INFINITY, |a, b| a.max(b.high));
    let ll = fen.iter().fold(f64::INFINITY, |a, b| a.min(b.low));
    (hh, ll)
}

/// SMA sur `amplitude` barres du champ extrait — NaN si fenêtre incomplète
/// (Pine : sma(high, 2) = na à la barre 0).
fn moyenne(bougies: &[Candle], i: usize, amplitude: usize, champ: fn(&Candle) -> f64) -> f64 {
    if amplitude == 0 || i + 1 < amplitude {
        return f64::NAN;
    }
    let fen = &bougies[i + 1 - amplitude..=i];
    fen.iter().map(champ).sum::<f64>() / amplitude as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie_ts(ts: i64, o: f64, h: f64, l: f64, c: f64) -> Candle {
        Candle {
            timestamp: Utc::now() + chrono::Duration::seconds(ts),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 1000.0,
        }
    }

    /// Rampes linéaires : montée/descente régulières, mèches ±0.4.
    fn rampe(depart: f64, pas: f64, nb: usize, ts_debut: i64) -> Vec<Candle> {
        (0..nb)
            .map(|k| {
                let p = depart + pas * k as f64;
                bougie_ts(ts_debut + k as i64, p, p + 0.4, p - 0.4, p)
            })
            .collect()
    }

    #[test]
    fn montee_pure_jamais_de_flèche() {
        // trend démarre 0 et ne peut pas basculer 1→0 : zéro flèche acheteuse,
        // zéro flèche vendeuse (pas de retournement baissier).
        let b = rampe(100.0, 1.0, 300, 0);
        let ht = calculer_halftrend(&b, 2);
        assert!(ht.trend.iter().all(|&t| t == 0));
        assert!(ht.arrow_up.iter().all(|&a| !a));
        assert!(ht.arrow_down.iter().all(|&a| !a));
    }

    #[test]
    fn retournement_v_date_la_flèche_vendeuse() {
        // Montée longue, puis chute franche : la machine doit passer trend=1
        // et dater UNE flèche vendeuse à la bascule.
        let mut b = rampe(100.0, 1.0, 220, 0);
        let haut = 100.0 + 220.0;
        b.extend(rampe(haut, -2.0, 120, 220));
        let ht = calculer_halftrend(&b, 2);
        assert_eq!(ht.trend[ht.trend.len() - 1], 1, "doit finir baissier");
        let fleches: Vec<usize> = ht
            .arrow_down
            .iter()
            .enumerate()
            .filter(|(_, &a)| a)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(fleches.len(), 1, "exactement une flèche down : {:?}", fleches);
        assert_eq!(ht.trend[fleches[0]], 1);
        assert_eq!(ht.trend[fleches[0] - 1], 0);
    }

    #[test]
    fn v_then_remonte_flèche_acheteuse() {
        // Montée, chute (trend=1), remontée (trend=0) : une flèche up datée,
        // exactement au retournement.
        let mut b = rampe(100.0, 1.0, 200, 0);
        b.extend(rampe(300.0, -2.0, 130, 200));
        let bas = 300.0 - 2.0 * 130.0;
        b.extend(rampe(bas, 2.0, 150, 330));
        let ht = calculer_halftrend(&b, 2);
        let fleches: Vec<usize> = ht
            .arrow_up
            .iter()
            .enumerate()
            .filter(|(_, &a)| a)
            .map(|(i, _)| i)
            .collect();
        assert!(!fleches.is_empty(), "au moins une flèche up");
        for i in fleches {
            assert_eq!(ht.trend[i], 0);
            assert_eq!(ht.trend[i - 1], 1);
        }
        assert_eq!(ht.trend.last(), Some(&0));
    }

    #[test]
    fn flèche_impossible_avant_atr_pret() {
        // Retournement avant la barre 100 : pas de flèche (arrowUp = na).
        let mut b = rampe(100.0, 1.0, 60, 0);
        b.extend(rampe(160.0, -2.0, 80, 60));
        let ht = calculer_halftrend(&b, 2);
        assert!(ht.arrow_down.iter().all(|&a| !a));
    }
}
