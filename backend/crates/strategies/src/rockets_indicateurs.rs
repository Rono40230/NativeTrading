use db::rockets::RocketsConfig;
use serde::Serialize;

// ── Constantes ───────────────────────────────────────────────────────────────

pub const STABLECOINS: &[&str] = &[
    "BUSD", "USDC", "TUSD", "DAI", "USDP", "FDUSD", "USDS", "EUR", "GBP", "PAX", "SUSD",
];
pub const KLINES_N: usize = 50;
pub const LOOKBACK: usize = 20;
pub const ATR_P: usize = 14;
pub const BATCH_SIZE: usize = 20;
pub const SCAN_SECS: u64 = 5 * 60;
pub const MAX_DISPLAY: usize = 30;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultat {
    pub symbol: String,
    pub ticker: String,
    pub prix: f64,
    pub change1h: f64,
    pub phase: String,
    pub score: i64,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub atr14: f64,
    pub rsi: f64,
    pub support: f64,
    pub target20: f64,
    pub closes: Vec<f64>,
    /// Ratio corps/amplitude totale de la dernière bougie (0.0–1.0)
    /// 1.0 = bougie pleine sans mèche | <0.3 = mèche dominante (rejet possible)
    pub ratio_corps: f64,
    /// EMA20 > EMA50 = tendance haussière préalable confirmée
    pub tendance_haussiere: bool,
    /// Bougies consécutives en compression (range < 90% ATR14) avant la dernière bougie
    pub nb_bougies_compression: usize,
    /// Hauteur de la base = max_recent − support (measured move pour TP1)
    pub hauteur_base: f64,
    /// Coefficient trailing stop calculé dynamiquement (score + volatilité)
    pub trailing_coeff: f64,
    /// TP1 précalculé = prix + ATR × cfg.tp1_mult() — niveau affiché en modale
    pub tp1: f64,
    /// TP2 précalculé = prix + ATR × cfg.tp2_mult()
    pub tp2: f64,
    /// Trigger trailing = prix + ATR × cfg.trailing_trigger_mult()
    pub tp3_trigger: f64,
    /// SL précalculé = prix - ATR × cfg.sl_mult
    pub sl: f64,
    /// Niveau d'entrée limite : pullback vers l'ancienne résistance (devenue support)
    pub entree_limite: f64,
    /// Niveau d'entrée stop : confirmation de momentum au-dessus de la zone
    pub entree_stop: f64,
    /// Niveau d'invalidation : setup annulé si le prix atteint ce niveau avant l'entrée
    pub niveau_invalidation: f64,
    /// Type d'entrée recommandé algorithmiquement : "limite" ou "stop"
    pub type_entree_rec: String,
    /// Ratio volume compression vs volume baseline (VCP) : <0.75 = assèchement valide
    pub volume_seche: f64,
    /// Score 0.0–1.0 de progressivité des contractions (VCP Minervini) : >0.7 = valide
    pub contraction_qualite: f64,
    /// ATR Wilder 50 périodes — référence long terme pour détecter vraie expansion
    pub atr50: f64,
    /// Série ordonnée des amplitudes (high−low) des bougies de compression.
    /// Décroissante = VCP authentique. Envoyée au LLM pour analyse structurelle.
    pub swing_amplitudes: Vec<f64>,
}

#[derive(serde::Deserialize)]
pub struct Ticker24h {
    pub symbol: String,
    #[serde(rename = "quoteVolume")]
    pub quote_volume: String,
}

// ── Indicateurs techniques ───────────────────────────────────────────────────

pub fn calc_atr(highs: &[f64], lows: &[f64], closes: &[f64]) -> (f64, f64) {
    let n = highs.len().min(lows.len()).min(closes.len());
    if n < 2 {
        return (0.0, 0.0);
    }
    let trs: Vec<f64> = (1..n)
        .map(|i| {
            let p = closes[i - 1];
            [
                highs[i] - lows[i],
                (highs[i] - p).abs(),
                (lows[i] - p).abs(),
            ]
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max)
        })
        .collect();

    // Wilder EMA (période 14) : même algorithme que indicators::calculer_atr
    let atr14 = if trs.len() >= ATR_P {
        let mut val = trs[..ATR_P].iter().sum::<f64>() / ATR_P as f64; // SMA initiale
        for &tr in &trs[ATR_P..] {
            val = (val * (ATR_P as f64 - 1.0) + tr) / ATR_P as f64;
        }
        val
    } else if !trs.is_empty() {
        trs.iter().sum::<f64>() / trs.len() as f64
    } else {
        0.0
    };

    // Wilder EMA (période 5)
    let atr5 = if trs.len() >= 5 {
        let mut val = trs[..5].iter().sum::<f64>() / 5.0;
        for &tr in &trs[5..] {
            val = (val * 4.0 + tr) / 5.0;
        }
        val
    } else {
        atr14
    };
    (atr14, atr5)
}

pub fn calc_rsi(closes: &[f64]) -> f64 {
    if closes.len() < 15 {
        return 50.0;
    }
    let slice = &closes[closes.len() - 15..];
    let (gains, losses) = slice.windows(2).fold((0.0f64, 0.0f64), |(g, l), w| {
        let d = w[1] - w[0];
        if d > 0.0 {
            (g + d, l)
        } else {
            (g, l - d)
        }
    });
    if losses == 0.0 {
        return 100.0;
    }
    100.0 - 100.0 / (1.0 + gains / losses)
}

pub fn calc_ema(closes: &[f64], period: usize) -> f64 {
    if closes.is_empty() || period == 0 {
        return 0.0;
    }
    if closes.len() < period {
        return closes.iter().sum::<f64>() / closes.len() as f64;
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut ema = closes[..period].iter().sum::<f64>() / period as f64;
    for &c in &closes[period..] {
        ema = c * k + ema * (1.0 - k);
    }
    ema
}

/// Bougies consécutives dont le range (high−low) < 90% ATR14, depuis la fin (hors dernière bougie).
pub fn calc_nb_compression(highs: &[f64], lows: &[f64], atr14: f64) -> usize {
    let seuil = atr14 * 0.90;
    let n = highs.len().saturating_sub(1);
    let mut count = 0usize;
    for i in (0..n).rev() {
        if highs[i] - lows[i] < seuil {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Données brutes passées à `calculer_phase` pour éviter une liste d'arguments trop longue.
pub struct ContextePhase {
    pub breakout: bool,
    pub ratio_volume: f64,
    pub rsi: f64,
    pub atr_ratio: f64,
    pub change1h: f64,
    pub nb_bougies_compression: usize,
    pub tendance_haussiere: bool,
    /// Ratio volume compression/baseline (VCP) : <0.75 = bon assèchement
    pub volume_seche: f64,
    /// Score 0.0–1.0 progressivité contractions : >0.70 = VCP authentique
    pub contraction_qualite: f64,
    /// ATR 50 périodes pour distinguer expansion réelle vs. bruit
    pub atr50: f64,
    /// ATR 14 périodes absolu (nécessaire pour comparaison long terme)
    pub atr14: f64,
    /// Ratio corps dernière bougie (qualité bougie de breakout)
    pub ratio_corps: f64,
}

pub fn calculer_phase(ctx: &ContextePhase, cfg: &RocketsConfig) -> Option<(String, i64)> {
    let ContextePhase {
        breakout, ratio_volume, rsi, atr_ratio, change1h,
        nb_bougies_compression, tendance_haussiere,
        volume_seche, contraction_qualite, atr50, atr14, ratio_corps,
    } = ctx;
    let (breakout, ratio_volume, rsi, atr_ratio, change1h, nb_bougies_compression, tendance_haussiere) =
        (*breakout, *ratio_volume, *rsi, *atr_ratio, *change1h, *nb_bougies_compression, *tendance_haussiere);
    let (volume_seche, contraction_qualite, atr50, atr14, ratio_corps) =
        (*volume_seche, *contraction_qualite, *atr50, *atr14, *ratio_corps);

    if breakout && ratio_volume >= cfg.ratio_volume_min {
        let mut s = 40i64;
        if ratio_volume >= 2.0 {
            s += 20;
        }
        if rsi > 60.0 && rsi <= cfg.rsi_max {
            s += 20;
        }
        if atr_ratio > 1.0 {
            s += 10;
        }
        if change1h > 1.0 {
            s += 10;
        }
        // Qualité de la bougie de breakout (VCP : clôture forte = setup solide)
        if ratio_corps >= 0.70 {
            s += 10; // Bougie de breakout pleine = conviction institutionnelle
        } else if ratio_corps < 0.50 {
            s -= 10; // Mèche dominante = rejet potentiel / bull trap
        }
        // Expansion réelle vs. long terme : ATR14 > ATR50 × 1.2 = vraie volatilité
        if atr50 > 0.0 && atr14 > atr50 * 1.2 {
            s += 10;
        }
        Some(("breakout".to_string(), s.min(100)))
    } else if atr_ratio < 0.80 {
        let phase = if atr_ratio < 0.65 {
            "prelancement"
        } else {
            "compression"
        };
        let mut s = ((1.0 - atr_ratio) * 55.0).round() as i64;
        if ratio_volume >= 1.3 {
            s += 15;
        }
        if rsi > 50.0 && rsi < 70.0 {
            s += 10;
        }
        // Bonus ressort comprimé : plus de bougies en compression = meilleure opportunité
        if nb_bougies_compression >= 4 {
            s += 15;
        }
        // Bonus tendance haussière confirmée (EMA20 > EMA50 1h) = continuation probable
        if tendance_haussiere {
            s += 10;
        }
        // ── Filtres professionnels VCP ────────────────────────────────────────
        // Assèchement du volume (signal clé VCP Minervini) : distribution absente
        if volume_seche < 0.75 {
            s += 15; // Volume qui sèche = smart money accumule silencieusement
        } else if volume_seche < 0.55 {
            s += 5; // Bonus supplémentaire pour assèchement fort (déjà compté ci-dessus)
        }
        // Progressivité des contractions (authenticité du pattern VCP)
        if contraction_qualite > 0.70 {
            s += 10; // Contractions progressivement décroissantes = VCP classique
        }
        if s < 15 {
            return None;
        }
        Some((phase.to_string(), s.min(100)))
    } else {
        None
    }
}

pub fn est_eligible(symbol: &str, quote_volume: f64, vol_min: f64) -> bool {
    if !symbol.ends_with("USDT") {
        return false;
    }
    if symbol.ends_with("UPUSDT") || symbol.ends_with("DOWNUSDT") {
        return false;
    }
    if symbol.ends_with("BULLUSDT") || symbol.ends_with("BEARUSDT") {
        return false;
    }
    let ticker = &symbol[..symbol.len() - 4];
    !STABLECOINS.contains(&ticker) && quote_volume >= vol_min
}

pub fn phase_priorite(phase: &str) -> u8 {
    match phase {
        "breakout" => 2,
        "prelancement" => 1,
        _ => 0,
    }
}
