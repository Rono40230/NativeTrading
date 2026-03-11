use common::Candle;
use indicators::{calculer_atr, calculer_bollinger, calculer_ema, calculer_macd, calculer_rsi};

/// Vecteur de features normalisées prêt pour XGBoost/RandomForest
/// Dimensions : 52 features par bougie (OHLCV normalisés + indicateurs)
pub const NB_FEATURES: usize = 52;

/// Construit le vecteur de features pour la bougie courante (dernière du slice)
/// Nécessite au minimum 60 bougies pour tous les indicateurs
pub fn extraire_features(bougies: &[Candle]) -> Option<Vec<f64>> {
    if bougies.len() < 60 {
        return None;
    }
    let n = bougies.len();
    let courant = &bougies[n - 1];

    // ─── Prix de référence pour normalisation ─────────────────────────────────
    let prix_ref = courant.close;
    if prix_ref == 0.0 {
        return None;
    }

    // ─── Indicateurs ──────────────────────────────────────────────────────────
    let ema9 = calculer_ema(bougies, 9);
    let ema21 = calculer_ema(bougies, 21);
    let ema50 = calculer_ema(bougies, 50);
    let rsi14 = calculer_rsi(bougies, 14);
    let atr14 = calculer_atr(bougies, 14);
    let macd = calculer_macd(bougies, 12, 26, 9);
    let boll = calculer_bollinger(bougies, 20, 2.0);

    let i = n - 1;

    // Sécurité : certains indicateurs peuvent être NaN sur les premières valeurs
    let safe = |v: f64| {
        if v.is_nan() || v.is_infinite() {
            0.0
        } else {
            v
        }
    };
    let norm = |v: f64| safe(v / prix_ref - 1.0); // retour relatif au close

    let mut f = Vec::with_capacity(NB_FEATURES);

    // ─── 1-5 : OHLCV normalisés ───────────────────────────────────────────────
    f.push(norm(courant.open));
    f.push(norm(courant.high));
    f.push(norm(courant.low));
    f.push(safe(
        courant.close / bougies[i.saturating_sub(1)].close - 1.0,
    )); // rendement
    let vol_moy: f64 = bougies[i.saturating_sub(20)..=i]
        .iter()
        .map(|b| b.volume)
        .sum::<f64>()
        / 21.0;
    f.push(safe(courant.volume / vol_moy.max(1.0))); // volume relatif

    // ─── 6-10 : Spreads OHLC ─────────────────────────────────────────────────
    f.push(safe((courant.high - courant.low) / prix_ref)); // range relatif
    f.push(safe((courant.close - courant.open) / prix_ref)); // corps bougie
    f.push(safe(
        (courant.high - courant.close.max(courant.open)) / prix_ref,
    )); // mèche haute
    f.push(safe(
        (courant.close.min(courant.open) - courant.low) / prix_ref,
    )); // mèche basse

    // Rendement sur 5 bougies
    let close_5 = bougies[i.saturating_sub(5)].close;
    f.push(safe(prix_ref / close_5.max(1e-10) - 1.0));

    // ─── 11-16 : EMA ─────────────────────────────────────────────────────────
    f.push(norm(safe(ema9[i])));
    f.push(norm(safe(ema21[i])));
    f.push(norm(safe(ema50[i])));
    f.push(safe(ema9[i] / ema21[i].max(1e-10) - 1.0)); // spread EMA 9/21
    f.push(safe(ema21[i] / ema50[i].max(1e-10) - 1.0)); // spread EMA 21/50
                                                        // Pente EMA21 sur 3 périodes
    let pente_ema21 = if i >= 3 {
        safe(ema21[i] / ema21[i - 3].max(1e-10) - 1.0)
    } else {
        0.0
    };
    f.push(pente_ema21);

    // ─── 17-19 : RSI ─────────────────────────────────────────────────────────
    f.push(safe(rsi14[i] / 100.0)); // RSI normalisé 0-1
    f.push(if rsi14[i] > 70.0 { 1.0 } else { 0.0 }); // zone surachat
    f.push(if rsi14[i] < 30.0 { 1.0 } else { 0.0 }); // zone survente

    // ─── 20-23 : ATR (volatilité) ────────────────────────────────────────────
    f.push(safe(atr14[i] / prix_ref)); // ATR relatif
    let atr_moy: f64 = (1..=14)
        .filter_map(|k| {
            let idx = i.checked_sub(k)?;
            let v = atr14[idx];
            if v.is_nan() {
                None
            } else {
                Some(v)
            }
        })
        .sum::<f64>()
        / 14.0;
    f.push(safe(atr14[i] / atr_moy.max(1e-10))); // ATR relatif à sa moyenne
                                                 // ATR supérieur à 150% de sa moyenne → volatilité extrême (trigger Straddle)
    f.push(if safe(atr14[i] / atr_moy.max(1e-10)) > 1.5 {
        1.0
    } else {
        0.0
    });
    f.push(safe(atr_moy / prix_ref)); // ATR moyen relatif

    // ─── 24-28 : MACD ────────────────────────────────────────────────────────
    f.push(safe(macd.ligne[i] / prix_ref));
    f.push(safe(macd.signal[i] / prix_ref));
    f.push(safe(macd.histogramme[i] / prix_ref));
    // Croisements MACD
    f.push(
        if i > 0 && macd.ligne[i] > macd.signal[i] && macd.ligne[i - 1] <= macd.signal[i - 1] {
            1.0
        } else {
            0.0
        },
    );
    f.push(
        if i > 0 && macd.ligne[i] < macd.signal[i] && macd.ligne[i - 1] >= macd.signal[i - 1] {
            1.0
        } else {
            0.0
        },
    );

    // ─── 29-33 : Bollinger ───────────────────────────────────────────────────
    let bb_milieu = boll.milieu[i];
    let bb_sup = boll.superieure[i];
    let bb_inf = boll.inferieure[i];
    let bb_largeur = safe((bb_sup - bb_inf) / bb_milieu.max(1e-10));
    f.push(bb_largeur); // Largeur des bandes
    f.push(safe((prix_ref - bb_milieu) / bb_milieu.max(1e-10))); // Position dans bandes
    f.push(if prix_ref > bb_sup { 1.0 } else { 0.0 }); // Au-dessus bande sup
    f.push(if prix_ref < bb_inf { 1.0 } else { 0.0 }); // En-dessous bande inf
                                                       // %B : position normalisée dans les bandes (0 = inférieure, 1 = supérieure)
    let bb_range = (bb_sup - bb_inf).max(1e-10);
    f.push(safe((prix_ref - bb_inf) / bb_range));

    // ─── 34-42 : Bougies précédentes (momentum) ──────────────────────────────
    for k in 1..=5 {
        let prev = i.checked_sub(k).map(|idx| &bougies[idx]);
        let rdt = prev
            .map(|b| safe(prix_ref / b.close.max(1e-10) - 1.0))
            .unwrap_or(0.0);
        f.push(rdt);
    }
    // Momentum 10 et 20 périodes
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
    // Volume des 5 dernières bougies normalisé
    for k in 1..=5 {
        let vol_prev = i
            .checked_sub(k)
            .map(|idx| bougies[idx].volume)
            .unwrap_or(0.0);
        f.push(safe(vol_prev / vol_moy.max(1.0)));
    }

    // ─── 48-52 : Patterns simple ─────────────────────────────────────────────
    // Doji (corps très petit)
    let corps = (courant.close - courant.open).abs();
    let range = (courant.high - courant.low).max(1e-10);
    f.push(safe(corps / range)); // Ratio corps/range (0 = doji)

    // 3 bougies haussières consécutives
    let trois_haussiers = (1..=3).all(|k| {
        i.checked_sub(k)
            .map(|idx| bougies[idx].close > bougies[idx].open)
            .unwrap_or(false)
    });
    f.push(if trois_haussiers { 1.0 } else { 0.0 });

    // 3 bougies baissières consécutives
    let trois_baissiers = (1..=3).all(|k| {
        i.checked_sub(k)
            .map(|idx| bougies[idx].close < bougies[idx].open)
            .unwrap_or(false)
    });
    f.push(if trois_baissiers { 1.0 } else { 0.0 });

    // Bougie englobante haussière
    let englobante_h = i >= 1
        && courant.close > courant.open
        && courant.open < bougies[i - 1].close
        && courant.close > bougies[i - 1].open;
    f.push(if englobante_h { 1.0 } else { 0.0 });

    // Score momentum global (EMA court > EMA long = haussier)
    let momentum = safe(ema9[i] / ema50[i].max(1e-10) - 1.0);
    f.push(momentum);

    debug_assert_eq!(f.len(), NB_FEATURES, "Nombre de features incorrect");
    Some(f)
}

/// Label pour entraînement : 1.0 = hausse dans N bougies, 0.0 = baisse/plat
pub fn labelliser(bougies: &[Candle], index: usize, horizon: usize, seuil_pct: f64) -> Option<f64> {
    let futur = index + horizon;
    if futur >= bougies.len() {
        return None;
    }
    let rendement = bougies[futur].close / bougies[index].close - 1.0;
    if rendement > seuil_pct {
        Some(1.0)
    } else if rendement < -seuil_pct {
        Some(0.0)
    } else {
        None // Neutre → exclure du dataset
    }
}
