use crate::calculer_atr;
use common::Candle;

/// Résultat du calcul Supertrend pour une série de bougies.
pub struct ResultatSupertrend {
    /// 1 = haussier (prix > bande basse), -1 = baissier (prix < bande haute), 0 = non calculé
    pub direction: Vec<i8>,
    /// Valeur de la ligne Supertrend active (bande basse si haussier, bande haute si baissier)
    pub valeur: Vec<f64>,
}

/// Calcule le Supertrend (algorithme identique à `ta.supertrend` de Pine Script v5).
///
/// - `atr_periode` : longueur de l'ATR de Wilder (défaut TradingView : 10)
/// - `facteur`     : multiplicateur ATR (défaut TradingView : 3.0)
///
/// Logique :
///   HL2 = (high + low) / 2
///   upper_basic = HL2 + factor * ATR
///   lower_basic = HL2 - factor * ATR
///   Bandes ajustées : la bande ne revient pas en arrière tant que la clôture ne la franchit pas.
///   Direction : croisement de la clôture au-delà des bandes de la bougie précédente.
pub fn calculer_supertrend(
    bougies: &[Candle],
    atr_periode: usize,
    facteur: f64,
) -> ResultatSupertrend {
    let n = bougies.len();
    let mut direction = vec![0i8; n];
    let mut valeur = vec![f64::NAN; n];

    if n <= atr_periode || atr_periode == 0 {
        return ResultatSupertrend { direction, valeur };
    }

    let atr = calculer_atr(bougies, atr_periode);

    let mut upper = vec![f64::NAN; n];
    let mut lower = vec![f64::NAN; n];

    for i in 0..n {
        if atr[i].is_nan() {
            continue;
        }

        let hl2 = (bougies[i].high + bougies[i].low) / 2.0;
        let upper_basic = hl2 + facteur * atr[i];
        let lower_basic = hl2 - facteur * atr[i];

        // Ajustement bande haute : ne monte que si la clôture précédente était au-dessus
        upper[i] = if i == 0 || upper[i - 1].is_nan() {
            upper_basic
        } else {
            let prev_close = bougies[i - 1].close;
            if upper_basic < upper[i - 1] || prev_close > upper[i - 1] {
                upper_basic
            } else {
                upper[i - 1]
            }
        };

        // Ajustement bande basse : ne descend que si la clôture précédente était en dessous
        lower[i] = if i == 0 || lower[i - 1].is_nan() {
            lower_basic
        } else {
            let prev_close = bougies[i - 1].close;
            if lower_basic > lower[i - 1] || prev_close < lower[i - 1] {
                lower_basic
            } else {
                lower[i - 1]
            }
        };

        // Direction : comparaison de la clôture courante avec les bandes de la bougie précédente
        direction[i] = if i == 0 || upper[i - 1].is_nan() {
            // Première bougie valide → haussier par défaut (comme Pine Script nz(trend[1], 1))
            1
        } else {
            let prev_dir = direction[i - 1];
            let close = bougies[i].close;
            if prev_dir == -1 && close > upper[i - 1] {
                1 // bascule haussier
            } else if prev_dir == 1 && close < lower[i - 1] {
                -1 // bascule baissier
            } else if prev_dir == 0 {
                if close > upper[i] {
                    1
                } else {
                    -1
                }
            } else {
                prev_dir
            }
        };

        valeur[i] = if direction[i] == 1 {
            lower[i]
        } else {
            upper[i]
        };
    }

    ResultatSupertrend { direction, valeur }
}
