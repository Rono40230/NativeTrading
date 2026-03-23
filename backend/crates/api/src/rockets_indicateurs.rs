use db::rockets::{RocketSignal, RocketsConfig};
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
    let atr14 = if trs.len() >= ATR_P {
        trs[trs.len() - ATR_P..].iter().sum::<f64>() / ATR_P as f64
    } else if !trs.is_empty() {
        trs.iter().sum::<f64>() / trs.len() as f64
    } else {
        0.0
    };
    let atr5 = if trs.len() >= 5 {
        trs[trs.len() - 5..].iter().sum::<f64>() / 5.0
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

pub fn calculer_phase(
    breakout: bool,
    ratio_volume: f64,
    rsi: f64,
    atr_ratio: f64,
    change1h: f64,
    cfg: &RocketsConfig,
) -> Option<(String, i64)> {
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

// ── Logique de progression de position ──────────────────────────────────────

pub fn calculer_verdict_rocket(
    s: &RocketSignal,
    prix: f64,
    peak: f64,
) -> Option<&'static str> {
    let atr14 = s.atr14.unwrap_or(s.prix_entree * 0.01);
    let trailing_stop = peak - atr14 * 1.5;

    // SL effectif progressif selon le niveau TP atteint (break-even)
    let sl_effectif = match (s.target2, s.target3) {
        (Some(_tp2), Some(tp3)) if peak >= tp3 => {
            // TP3 en route : trailing stop
            return if prix <= trailing_stop {
                Some("TP3")
            } else {
                None
            };
        }
        (Some(tp2), _) if peak >= tp2 => s.target, // BE = TP1
        _ if peak >= s.target => s.prix_entree,    // BE = entrée
        _ => s.stop_loss,                          // SL original
    };

    if prix <= sl_effectif {
        return Some("invalide");
    }
    // TP2 : fermeture immédiate si prix >= TP2 et pas encore en zone TP3
    if let Some(tp2) = s.target2 {
        if prix >= tp2 {
            return Some("TP2");
        }
    }
    // TP1 : fermeture uniquement si pas de TP2 (sinon on attend TP2, SL=BE)
    if prix >= s.target && s.target2.is_none() {
        return Some("TP1");
    }
    None
}
