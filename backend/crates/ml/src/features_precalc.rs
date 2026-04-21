//! Pré-calcul des séries d'indicateurs pour les boucles d'entraînement ML.
//!
//! Au lieu de recalculer EMA/RSI/ATR/MACD/Bollinger depuis zéro à chaque
//! itération (coût O(N²)), on les calcule **une seule fois** en O(N) puis on
//! lit les valeurs par index. Réservé aux boucles d'entraînement et d'évaluation.
//!
//! Pour l'inférence temps réel (une seule bougie), utiliser `features::extraire_features`.

use common::Candle;
use indicators::{calculer_atr, calculer_bollinger, calculer_ema, calculer_macd, calculer_rsi};

use crate::features::NB_FEATURES;

/// Séries d'indicateurs pré-calculées sur un historique complet.
pub struct SeriesIndicateurs {
    pub ema9: Vec<f64>,
    pub ema21: Vec<f64>,
    pub ema50: Vec<f64>,
    pub rsi14: Vec<f64>,
    pub atr14: Vec<f64>,
    pub macd_ligne: Vec<f64>,
    pub macd_signal: Vec<f64>,
    pub macd_histo: Vec<f64>,
    pub boll_milieu: Vec<f64>,
    pub boll_sup: Vec<f64>,
    pub boll_inf: Vec<f64>,
}

/// Calcule toutes les séries d'indicateurs en **un seul passage** sur `bougies`.
///
/// Complexité : O(N) — à appeler une fois avant la boucle d'entraînement.
pub fn precalculer(bougies: &[Candle]) -> SeriesIndicateurs {
    let ema9 = calculer_ema(bougies, 9);
    let ema21 = calculer_ema(bougies, 21);
    let ema50 = calculer_ema(bougies, 50);
    let rsi14 = calculer_rsi(bougies, 14);
    let atr14 = calculer_atr(bougies, 14);
    let macd = calculer_macd(bougies, 12, 26, 9);
    let boll = calculer_bollinger(bougies, 20, 2.0);
    SeriesIndicateurs {
        ema9,
        ema21,
        ema50,
        rsi14,
        atr14,
        macd_ligne: macd.ligne,
        macd_signal: macd.signal,
        macd_histo: macd.histogramme,
        boll_milieu: boll.milieu,
        boll_sup: boll.superieure,
        boll_inf: boll.inferieure,
    }
}

/// Construit le vecteur de features pour la bougie à l'index `i` en lisant
/// les séries pré-calculées. Identique à `extraire_features` mais sans recomputation.
///
/// Requiert `i >= 59` (minimum 60 bougies de contexte) et `i < bougies.len()`.
pub fn extraire_depuis_series(
    s: &SeriesIndicateurs,
    bougies: &[Candle],
    i: usize,
) -> Option<Vec<f64>> {
    if i < 59 || i >= bougies.len() {
        return None;
    }
    // Guard : séries cohérentes avec l'index demandé
    if i >= s.ema9.len() || i >= s.atr14.len() || i >= s.macd_ligne.len() {
        return None;
    }

    let courant = &bougies[i];
    let prix_ref = courant.close;
    if prix_ref == 0.0 {
        return None;
    }

    let safe = |v: f64| {
        if v.is_nan() || v.is_infinite() {
            0.0
        } else {
            v
        }
    };
    let norm = |v: f64| safe(v / prix_ref - 1.0);

    let mut f = Vec::with_capacity(NB_FEATURES);

    // ─── 1-5 : OHLCV normalisés ───────────────────────────────────────────────
    f.push(norm(courant.open));
    f.push(norm(courant.high));
    f.push(norm(courant.low));
    f.push(safe(
        courant.close / bougies[i.saturating_sub(1)].close - 1.0,
    ));
    let vol_moy: f64 = bougies[i.saturating_sub(20)..=i]
        .iter()
        .map(|b| b.volume)
        .sum::<f64>()
        / 21.0;
    f.push(safe(courant.volume / vol_moy.max(1.0)));

    // ─── 6-10 : Spreads OHLC ─────────────────────────────────────────────────
    f.push(safe((courant.high - courant.low) / prix_ref));
    f.push(safe((courant.close - courant.open) / prix_ref));
    f.push(safe(
        (courant.high - courant.close.max(courant.open)) / prix_ref,
    ));
    f.push(safe(
        (courant.close.min(courant.open) - courant.low) / prix_ref,
    ));
    let close_5 = bougies[i.saturating_sub(5)].close;
    f.push(safe(prix_ref / close_5.max(1e-10) - 1.0));

    // ─── 11-16 : EMA ─────────────────────────────────────────────────────────
    f.push(norm(safe(s.ema9[i])));
    f.push(norm(safe(s.ema21[i])));
    f.push(norm(safe(s.ema50[i])));
    f.push(safe(s.ema9[i] / s.ema21[i].max(1e-10) - 1.0));
    f.push(safe(s.ema21[i] / s.ema50[i].max(1e-10) - 1.0));
    let pente_ema21 = if i >= 3 {
        safe(s.ema21[i] / s.ema21[i - 3].max(1e-10) - 1.0)
    } else {
        0.0
    };
    f.push(pente_ema21);

    // ─── 17-19 : RSI ─────────────────────────────────────────────────────────
    f.push(safe(s.rsi14[i] / 100.0));
    f.push(if s.rsi14[i] > 70.0 { 1.0 } else { 0.0 });
    f.push(if s.rsi14[i] < 30.0 { 1.0 } else { 0.0 });

    // ─── 20-23 : ATR ─────────────────────────────────────────────────────────
    f.push(safe(s.atr14[i] / prix_ref));
    let atr_moy: f64 = (1..=14)
        .filter_map(|k| {
            let idx = i.checked_sub(k)?;
            let v = s.atr14[idx];
            if v.is_nan() {
                None
            } else {
                Some(v)
            }
        })
        .sum::<f64>()
        / 14.0;
    f.push(safe(s.atr14[i] / atr_moy.max(1e-10)));
    f.push(if safe(s.atr14[i] / atr_moy.max(1e-10)) > 1.5 {
        1.0
    } else {
        0.0
    });
    f.push(safe(atr_moy / prix_ref));

    // ─── 24-28 : MACD ────────────────────────────────────────────────────────
    f.push(safe(s.macd_ligne[i] / prix_ref));
    f.push(safe(s.macd_signal[i] / prix_ref));
    f.push(safe(s.macd_histo[i] / prix_ref));
    f.push(
        if i > 0
            && s.macd_ligne[i] > s.macd_signal[i]
            && s.macd_ligne[i - 1] <= s.macd_signal[i - 1]
        {
            1.0
        } else {
            0.0
        },
    );
    f.push(
        if i > 0
            && s.macd_ligne[i] < s.macd_signal[i]
            && s.macd_ligne[i - 1] >= s.macd_signal[i - 1]
        {
            1.0
        } else {
            0.0
        },
    );

    // ─── 29-33 : Bollinger ───────────────────────────────────────────────────
    let bb_milieu = s.boll_milieu[i];
    let bb_sup = s.boll_sup[i];
    let bb_inf = s.boll_inf[i];
    let bb_largeur = safe((bb_sup - bb_inf) / bb_milieu.max(1e-10));
    f.push(bb_largeur);
    f.push(safe((prix_ref - bb_milieu) / bb_milieu.max(1e-10)));
    f.push(if prix_ref > bb_sup { 1.0 } else { 0.0 });
    f.push(if prix_ref < bb_inf { 1.0 } else { 0.0 });
    let bb_range = (bb_sup - bb_inf).max(1e-10);
    f.push(safe((prix_ref - bb_inf) / bb_range));

    // ─── 34-42 : Momentum ────────────────────────────────────────────────────
    for k in 1..=5 {
        let prev = i.checked_sub(k).map(|idx| &bougies[idx]);
        let rdt = prev
            .map(|b| safe(prix_ref / b.close.max(1e-10) - 1.0))
            .unwrap_or(0.0);
        f.push(rdt);
    }
    f.push(if i >= 10 {
        safe(prix_ref / bougies[i - 10].close.max(1e-10) - 1.0)
    } else {
        0.0
    });
    f.push(if i >= 20 {
        safe(prix_ref / bougies[i - 20].close.max(1e-10) - 1.0)
    } else {
        0.0
    });
    f.push(if i >= 30 {
        safe(prix_ref / bougies[i - 30].close.max(1e-10) - 1.0)
    } else {
        0.0
    });
    f.push(if i >= 50 {
        safe(prix_ref / bougies[i - 50].close.max(1e-10) - 1.0)
    } else {
        0.0
    });

    // ─── 43-47 : Volume ──────────────────────────────────────────────────────
    for k in 1..=5 {
        let vol_prev = i
            .checked_sub(k)
            .map(|idx| bougies[idx].volume)
            .unwrap_or(0.0);
        f.push(safe(vol_prev / vol_moy.max(1.0)));
    }

    // ─── 48-52 : Patterns ────────────────────────────────────────────────────
    let corps = (courant.close - courant.open).abs();
    let range = (courant.high - courant.low).max(1e-10);
    f.push(safe(corps / range));

    let trois_haussiers = (1..=3).all(|k| {
        i.checked_sub(k)
            .map(|idx| bougies[idx].close > bougies[idx].open)
            .unwrap_or(false)
    });
    f.push(if trois_haussiers { 1.0 } else { 0.0 });

    let trois_baissiers = (1..=3).all(|k| {
        i.checked_sub(k)
            .map(|idx| bougies[idx].close < bougies[idx].open)
            .unwrap_or(false)
    });
    f.push(if trois_baissiers { 1.0 } else { 0.0 });

    let englobante_h = i >= 1
        && courant.close > courant.open
        && courant.open < bougies[i - 1].close
        && courant.close > bougies[i - 1].open;
    f.push(if englobante_h { 1.0 } else { 0.0 });

    let momentum = safe(s.ema9[i] / s.ema50[i].max(1e-10) - 1.0);
    f.push(momentum);

    debug_assert_eq!(f.len(), NB_FEATURES, "Nombre de features incorrect");
    Some(f)
}
