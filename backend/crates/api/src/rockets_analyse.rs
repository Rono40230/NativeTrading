//! Analyse technique d'un symbol Binance pour le scan Rockets.
//! Séparé de rockets_scan.rs pour respecter la limite de 300 lignes.
use db::rockets::RocketsConfig;
use strategies::rockets_filtres::{calc_atr50, calc_contraction_qualite, calc_swing_amplitudes, calc_volume_seche};
use strategies::rockets_indicateurs::{
    calc_atr, calc_ema, calc_nb_compression, calc_rsi, calculer_phase,
    ScanResultat, KLINES_N, LOOKBACK,
};
use strategies::rockets_niveaux::{
    calculer_entree_limite, calculer_entree_stop,
    calculer_niveau_invalidation, recommander_type_entree,
};
use strategies::rockets_position::calculer_trailing_coeff;

/// Construit un Vec<Candle> depuis les vecs OHLCV extraits de Binance klines.
/// Utilisé pour appeler `ml::features::extraire_features` sans re-requête.
fn bougies_depuis_vecs(
    opens: &[f64],
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
    volumes: &[f64],
) -> Vec<common::Candle> {
    let n = opens.len().min(highs.len()).min(lows.len()).min(closes.len()).min(volumes.len());
    (0..n)
        .map(|i| common::Candle {
            timestamp: chrono::Utc::now(), // non utilisé par extraire_features
            open: opens[i],
            high: highs[i],
            low: lows[i],
            close: closes[i],
            volume: volumes[i],
        })
        .collect()
}

pub async fn analyser_symbol(
    client: &reqwest::Client,
    ticker: &str,
    cfg: &RocketsConfig,
) -> Option<ScanResultat> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}USDT&interval=1h&limit={}",
        ticker, KLINES_N
    );

    // Binance response: Vec<Vec<serde_json::Value>>
    // [openTime(ms), open, high, low, close, volume, closeTime, ...]
    // Ordre: chronologique (plus ancien → plus récent)
    let raw: Vec<Vec<serde_json::Value>> = client.get(&url).send().await.ok()?.json().await.ok()?;

    if raw.len() < 20 {
        return None;
    }

    let parse = |row: &Vec<serde_json::Value>, idx: usize| -> f64 {
        row.get(idx)
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| v.as_f64())
            })
            .unwrap_or(0.0)
    };
    // Binance: [openTime(ms), open, high, low, close, volume, closeTime, ...]
    let opens: Vec<f64> = raw.iter().map(|k| parse(k, 1)).collect();
    let highs: Vec<f64> = raw.iter().map(|k| parse(k, 2)).collect();
    let lows: Vec<f64> = raw.iter().map(|k| parse(k, 3)).collect();
    let closes: Vec<f64> = raw.iter().map(|k| parse(k, 4)).collect();
    let volumes: Vec<f64> = raw.iter().map(|k| parse(k, 5)).collect();
    let prix = *closes.last()?;

    // Ratio corps/amplitude de la dernière bougie (qualité du breakout)
    let ratio_corps = {
        let open = *opens.last()?;
        let high = *highs.last()?;
        let low = *lows.last()?;
        let amplitude = high - low;
        if amplitude > 0.0 {
            (prix - open).abs() / amplitude
        } else {
            0.0
        }
    };

    let (atr14, atr5) = calc_atr(&highs, &lows, &closes);
    let atr_ratio = if atr14 > 0.0 { atr5 / atr14 } else { 1.0 };

    let prev_end = volumes.len().saturating_sub(1);
    let prev_start = prev_end.saturating_sub(LOOKBACK);
    let avg_vol = if prev_end > prev_start {
        volumes[prev_start..prev_end].iter().sum::<f64>() / (prev_end - prev_start) as f64
    } else {
        1.0
    };
    let ratio_volume = if avg_vol > 0.0 {
        volumes.last().copied().unwrap_or(0.0) / avg_vol
    } else {
        1.0
    };

    let high_end = highs.len().saturating_sub(1);
    let high_start = high_end.saturating_sub(LOOKBACK);
    let max_recent = highs[high_start..high_end]
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    // Breakout strict : prix dépasse le max récent ET corps suffisant ET volatilité en expansion
    let breakout = prix > max_recent && max_recent > 0.0 && ratio_corps >= 0.40 && atr_ratio >= 0.85;

    let low_start = lows.len().saturating_sub(LOOKBACK);
    let support = lows[low_start..]
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let rsi = calc_rsi(&closes);
    let change1h = if closes.len() >= 2 {
        let prev = closes[closes.len() - 2];
        if prev > 0.0 {
            (prix - prev) / prev * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Calculés avant calculer_phase car ils influencent le score (compression/prelancement)
    let tendance_haussiere = calc_ema(&closes, 20) > calc_ema(&closes, 50);
    let nb_bougies_compression = calc_nb_compression(&highs, &lows, atr14);

    // ── Filtres professionnels VCP ────────────────────────────────────────────
    let atr50 = calc_atr50(&highs, &lows, &closes);
    let volume_seche = calc_volume_seche(&volumes, nb_bougies_compression, LOOKBACK);
    let contraction_qualite = calc_contraction_qualite(&highs, &lows, nb_bougies_compression);
    let swing_amplitudes = calc_swing_amplitudes(&highs, &lows, nb_bougies_compression);

    let ctx_phase = strategies::rockets_indicateurs::ContextePhase {
        breakout,
        ratio_volume,
        rsi,
        atr_ratio,
        change1h,
        nb_bougies_compression,
        tendance_haussiere,
        volume_seche,
        contraction_qualite,
        atr50,
        atr14,
        ratio_corps,
    };
    let (phase, score) = calculer_phase(&ctx_phase, cfg)?;
    let trailing_coeff = calculer_trailing_coeff(score, atr_ratio, cfg);
    let hauteur_base = (max_recent - support).max(0.0);
    let closes_spark: Vec<f64> = closes[closes.len().saturating_sub(24)..].to_vec();
    let sl = prix - atr14 * cfg.sl_mult;
    let tp1 = prix + atr14 * cfg.tp1_mult();
    let tp2 = prix + atr14 * cfg.tp2_mult();
    let tp3_trigger = prix + atr14 * cfg.trailing_trigger_mult();
    let entree_limite = calculer_entree_limite(prix, max_recent, support, &phase);
    let entree_stop = calculer_entree_stop(prix, max_recent, &phase);
    let niveau_invalidation = calculer_niveau_invalidation(support, atr14);
    let type_entree_rec = recommander_type_entree(atr_ratio, ratio_corps, change1h).to_string();

    // ── Features ML snapshot ──────────────────────────────────────────────────
    // Reconstruit les Candles depuis les vecs pour appeler extraire_features.
    // Stockées dans ScanResultat.features_ml → persistées en DB à la sauvegarde du signal.
    let bougies_ml = bougies_depuis_vecs(&opens, &highs, &lows, &closes, &volumes);
    let features_ml = ml::features::extraire_features(&bougies_ml);

    Some(ScanResultat {
        symbol: format!("{}USDT", ticker),
        ticker: ticker.to_string(),
        prix,
        change1h,
        phase,
        score,
        ratio_volume,
        atr_ratio,
        atr14,
        rsi,
        support,
        target20: max_recent,
        closes: closes_spark,
        ratio_corps,
        tendance_haussiere,
        nb_bougies_compression,
        hauteur_base,
        trailing_coeff,
        sl,
        tp1,
        tp2,
        tp3_trigger,
        entree_limite,
        entree_stop,
        niveau_invalidation,
        type_entree_rec,
        volume_seche,
        contraction_qualite,
        atr50,
        swing_amplitudes,
        features_ml,
    })
}
