//! Noms des 52 features — dans le même ordre que `features::extraire_features()`.
//! Utilisés pour l'affichage des feature importances (P4).

use crate::features::NB_FEATURES;

pub const FEATURE_NOMS: [&str; NB_FEATURES] = [
    // 1-5 : OHLCV normalisés
    "open_rel",
    "high_rel",
    "low_rel",
    "rendement_1",
    "volume_rel",
    // 6-10 : Spreads OHLC
    "range_rel",
    "corps_rel",
    "meche_haute",
    "meche_basse",
    "rendement_5",
    // 11-16 : EMA
    "ema9_rel",
    "ema21_rel",
    "ema50_rel",
    "spread_ema9_21",
    "spread_ema21_50",
    "pente_ema21",
    // 17-19 : RSI
    "rsi14",
    "rsi_surachat",
    "rsi_survente",
    // 20-23 : ATR
    "atr14_rel",
    "atr_vs_moyenne",
    "atr_extreme_150pct",
    "atr_moyen_rel",
    // 24-28 : MACD
    "macd_ligne",
    "macd_signal",
    "macd_histogramme",
    "macd_croise_haut",
    "macd_croise_bas",
    // 29-33 : Bollinger
    "bb_largeur",
    "bb_position",
    "bb_au_dessus_sup",
    "bb_en_dessous_inf",
    "bb_pct_b",
    // 34-42 : Momentum bougies précédentes
    "rdt_1",
    "rdt_2",
    "rdt_3",
    "rdt_4",
    "rdt_5",
    "momentum_10",
    "momentum_20",
    "momentum_30",
    "momentum_50",
    // 43-47 : Volume
    "vol_1",
    "vol_2",
    "vol_3",
    "vol_4",
    "vol_5",
    // 48-52 : Patterns
    "ratio_corps_range",
    "trois_haussiers",
    "trois_baissiers",
    "englobante_haussiere",
    "momentum_ema9_50",
];
