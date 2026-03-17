use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

const ATR_HAUSSE: f64 = 1.1;   // ATR > 110% de sa moyenne
const LOOKBACK: usize = 14;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn atr_ratio(atr: &[f64], i: usize) -> f64 {
    let debut = i.saturating_sub(LOOKBACK);
    let vals: Vec<f64> = atr[debut..i].iter().filter(|v| v.is_finite()).copied().collect();
    if vals.is_empty() { return 1.0; }
    let moy = vals.iter().sum::<f64>() / vals.len() as f64;
    if moy <= 0.0 { 1.0 } else { atr[i] / moy }
}

fn bandwidth(haute: &[f64], basse: &[f64], milieu: &[f64], i: usize) -> f64 {
    if milieu[i] == 0.0 { return 0.0; }
    (haute[i] - basse[i]) / milieu[i]
}

fn bw_min(haute: &[f64], basse: &[f64], milieu: &[f64], i: usize, lookback: usize) -> f64 {
    (i.saturating_sub(lookback)..i)
        .filter_map(|j| {
            if milieu[j] == 0.0 || haute[j].is_nan() { None }
            else { Some((haute[j] - basse[j]) / milieu[j]) }
        })
        .fold(f64::INFINITY, f64::min)
}

fn signal(ts: i64, type_signal: &str, dir: DirectionSignal, force: NiveauForce,
    desc: String, valeur: f64, prix_entree: f64) -> SignalIndicateur {
    SignalIndicateur {
        timestamp: ts, source: "Combiné".into(),
        type_signal: type_signal.into(), direction: dir, force,
        description: desc, valeur, prix_entree,
    }
}

// ─── Détections ───────────────────────────────────────────────────────────────

/// 9 signaux combinés multi-indicateurs (tous Fort ou Moyen)
#[allow(clippy::too_many_arguments)]
pub fn detecter_signaux_combines(
    timestamps: &[i64],
    closes: &[f64],
    ema: Option<&[f64]>,
    rsi: Option<&[f64]>,
    macd_ligne: Option<&[f64]>,
    macd_sig: Option<&[f64]>,
    boll_haute: Option<&[f64]>,
    boll_milieu: Option<&[f64]>,
    boll_basse: Option<&[f64]>,
    atr: Option<&[f64]>,
) -> Vec<SignalIndicateur> {
    let n = timestamps.len().min(closes.len());
    if n < 2 { return vec![]; }
    let mut out = Vec::new();

    for i in LOOKBACK..n {
        let ts = timestamps[i];
        let c = closes[i];
        let c_prev = closes[i - 1];

        // ── 1 & 2 : Bollinger bande + RSI extrême ──────────────────────────
        if let (Some(rsi), Some(bh), Some(bm), Some(bb)) = (rsi, boll_haute, boll_milieu, boll_basse) {
            if bh[i].is_finite() && bb[i].is_finite() && rsi[i].is_finite() {
                // Bullish : bande basse + oversold
                if c <= bb[i] && rsi[i] < 30.0 {
                    out.push(signal(ts, "boll_rsi_bull", DirectionSignal::Bullish, NiveauForce::Fort,
                        format!("Bande basse + RSI oversold ({:.1}) — buy confirmé", rsi[i]),
                        bb[i], c));
                }
                // Bearish : bande haute + overbought
                if c >= bh[i] && rsi[i] > 70.0 {
                    out.push(signal(ts, "boll_rsi_bear", DirectionSignal::Bearish, NiveauForce::Fort,
                        format!("Bande haute + RSI overbought ({:.1}) — sell confirmé", rsi[i]),
                        bh[i], c));
                }
                // ── 3 & 4 : Squeeze + MACD croisement ──────────────────────
                if let (Some(ml), Some(ms)) = (macd_ligne, macd_sig) {
                    if i >= LOOKBACK && ml[i].is_finite() && ms[i].is_finite()
                        && ml[i - 1].is_finite() && ms[i - 1].is_finite() {
                        let bw = bandwidth(bh, bb, bm, i);
                        let bwmin = bw_min(bh, bb, bm, i, LOOKBACK);
                        let is_squeeze = bwmin.is_finite() && bw <= bwmin * 1.05;
                        if is_squeeze {
                            // Bullish : squeeze + MACD croise à la hausse
                            if ml[i - 1] <= ms[i - 1] && ml[i] > ms[i] {
                                out.push(signal(ts, "squeeze_macd_bull", DirectionSignal::Bullish, NiveauForce::Fort,
                                    format!("Squeeze + MACD haussier — breakout buy imminent (bw={:.4})", bw),
                                    bw, c));
                            }
                            // Bearish : squeeze + MACD croise à la baisse
                            if ml[i - 1] >= ms[i - 1] && ml[i] < ms[i] {
                                out.push(signal(ts, "squeeze_macd_bear", DirectionSignal::Bearish, NiveauForce::Fort,
                                    format!("Squeeze + MACD baissier — breakout sell imminent (bw={:.4})", bw),
                                    bw, c));
                            }
                        }
                    }
                }
            }
        }

        // ── 5 & 6 : ATR hausse + MACD momentum ─────────────────────────────
        if let (Some(atr), Some(ml), Some(ms)) = (atr, macd_ligne, macd_sig) {
            if atr[i].is_finite() && ml[i].is_finite() && ms[i].is_finite() {
                let ratio = atr_ratio(atr, i);
                if ratio >= ATR_HAUSSE {
                    // Bullish : ATR ↑ + MACD au-dessus du signal
                    if ml[i] > ms[i] && ml[i] > ml[i - 1] {
                        out.push(signal(ts, "atr_macd_bull", DirectionSignal::Bullish, NiveauForce::Fort,
                            format!("ATR ×{:.1} + MACD momentum haussier — tendance forte buy", ratio),
                            atr[i], c));
                    }
                    // Bearish : ATR ↑ + MACD en dessous du signal
                    if ml[i] < ms[i] && ml[i] < ml[i - 1] {
                        out.push(signal(ts, "atr_macd_bear", DirectionSignal::Bearish, NiveauForce::Fort,
                            format!("ATR ×{:.1} + MACD momentum baissier — tendance forte sell", ratio),
                            atr[i], c));
                    }
                }
            }
        }

        // ── 7 & 8 : EMA + MACD même direction ──────────────────────────────
        if let (Some(ema), Some(ml), Some(ms)) = (ema, macd_ligne, macd_sig) {
            if ema[i].is_finite() && ml[i].is_finite() && ms[i].is_finite() {
                let macd_bull = ml[i] > ms[i];
                let macd_bear = ml[i] < ms[i];
                // Bullish : prix au-dessus EMA et MACD haussier
                if c > ema[i] && c_prev <= ema[i - 1] && macd_bull {
                    out.push(signal(ts, "ema_macd_bull", DirectionSignal::Bullish, NiveauForce::Moyen,
                        format!("Prix franchit l'EMA à hausse + MACD haussier — buy ({:.5})", ema[i]),
                        ema[i], c));
                }
                // Bearish : prix sous EMA et MACD baissier
                if c < ema[i] && c_prev >= ema[i - 1] && macd_bear {
                    out.push(signal(ts, "ema_macd_bear", DirectionSignal::Bearish, NiveauForce::Moyen,
                        format!("Prix franchit l'EMA à baisse + MACD baissier — sell ({:.5})", ema[i]),
                        ema[i], c));
                }
            }
        }

        // ── 9 : Golden/Death Cross + MACD aligné ────────────────────────────
        if let (Some(ema), Some(ml), Some(ms)) = (ema, macd_ligne, macd_sig) {
            if i >= 1 && ema[i].is_finite() && ema[i - 1].is_finite()
                && ml[i].is_finite() && ms[i].is_finite() {
                let golden = closes[i - 1] <= ema[i - 1] && closes[i] > ema[i];
                let death  = closes[i - 1] >= ema[i - 1] && closes[i] < ema[i];
                if golden && ml[i] > ms[i] {
                    out.push(signal(ts, "cross_macd_bull", DirectionSignal::Bullish, NiveauForce::Fort,
                        format!("Golden Cross EMA + MACD aligné — confluece buy majeure ({:.5})", ema[i]),
                        ema[i], c));
                }
                if death && ml[i] < ms[i] {
                    out.push(signal(ts, "cross_macd_bear", DirectionSignal::Bearish, NiveauForce::Fort,
                        format!("Death Cross EMA + MACD aligné — confluence sell majeure ({:.5})", ema[i]),
                        ema[i], c));
                }
            }
        }
    }
    out
}
