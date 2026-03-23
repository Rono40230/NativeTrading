pub use crate::rockets_indicateurs::ScanResultat;
use crate::rockets_indicateurs::{
    calc_atr, calc_rsi, calculer_phase, est_eligible, phase_priorite, Ticker24h, BATCH_SIZE,
    KLINES_N, LOOKBACK, MAX_DISPLAY, SCAN_SECS,
};
use db::rockets::{self, NouveauRocket, RocketsConfig};
use futures_util::future::join_all;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;

// ── État partagé (lecture depuis le handler HTTP) ────────────────────────────

static SCAN_RESULTS: OnceLock<Arc<RwLock<Vec<ScanResultat>>>> = OnceLock::new();
static TOTAL_CANDIDATS: OnceLock<Arc<RwLock<usize>>> = OnceLock::new();

pub fn get_scan_results() -> Arc<RwLock<Vec<ScanResultat>>> {
    SCAN_RESULTS
        .get_or_init(|| Arc::new(RwLock::new(vec![])))
        .clone()
}

pub fn get_total_candidats() -> Arc<RwLock<usize>> {
    TOTAL_CANDIDATS
        .get_or_init(|| Arc::new(RwLock::new(0)))
        .clone()
}

async fn analyser_symbol(
    client: &reqwest::Client,
    ticker: &str,
    cfg: &RocketsConfig,
) -> Option<ScanResultat> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}USDT&interval=1h&limit={}",
        ticker, KLINES_N
    );
    let raw: Vec<serde_json::Value> = client.get(&url).send().await.ok()?.json().await.ok()?;
    if raw.len() < 20 {
        return None;
    }

    let parse = |k: &serde_json::Value, idx: usize| -> f64 {
        k[idx].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0)
    };
    let opens: Vec<f64> = raw.iter().map(|k| parse(k, 1)).collect();
    let closes: Vec<f64> = raw.iter().map(|k| parse(k, 4)).collect();
    let highs: Vec<f64> = raw.iter().map(|k| parse(k, 2)).collect();
    let lows: Vec<f64> = raw.iter().map(|k| parse(k, 3)).collect();
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
    let breakout = prix > max_recent && max_recent > 0.0;

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

    let (phase, score) = calculer_phase(breakout, ratio_volume, rsi, atr_ratio, change1h, cfg)?;
    let closes_spark = closes[closes.len().saturating_sub(24)..].to_vec();

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
    })
}

// ── Worker de scan ───────────────────────────────────────────────────────────

pub async fn demarrer_worker_scan(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Worker scan HTTP: {}", e);
            return;
        }
    };

    loop {
        if let Err(e) = executer_scan(&client, &pool).await {
            tracing::warn!("Scan rockets erreur: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(SCAN_SECS)).await;
    }
}

async fn executer_scan(client: &reqwest::Client, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    use anyhow::Context;

    // Lire la config depuis la DB (paramètres ajustables par l'utilisateur)
    let cfg = rockets::lire_config(pool).await;
    tracing::info!(
        "Config scan: score_min={} rsi_max={} ratio_vol_min={} phases={:?}",
        cfg.score_min,
        cfg.rsi_max,
        cfg.ratio_volume_min,
        cfg.phases_actives
    );

    let tickers: Vec<Ticker24h> = client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send()
        .await
        .context("fetch ticker/24hr")?
        .json()
        .await
        .context("parse ticker/24hr")?;

    let vol_min = cfg.vol_marche_min;
    let candidats: Vec<String> = tickers
        .into_iter()
        .filter(|t| {
            let vol = t.quote_volume.parse::<f64>().unwrap_or(0.0);
            est_eligible(&t.symbol, vol, vol_min)
        })
        .map(|t| t.symbol[..t.symbol.len() - 4].to_string())
        .collect();

    tracing::info!("Scan rockets: {} candidats", candidats.len());
    *get_total_candidats().write().await = candidats.len();

    let mut resultats: Vec<ScanResultat> = Vec::new();
    for batch in candidats.chunks(BATCH_SIZE) {
        let futs = batch
            .iter()
            .map(|ticker| analyser_symbol(client, ticker.as_str(), &cfg));
        let res = join_all(futs).await;
        resultats.extend(res.into_iter().flatten());
    }

    resultats.sort_by(|a, b| {
        phase_priorite(&b.phase)
            .cmp(&phase_priorite(&a.phase))
            .then(b.score.cmp(&a.score))
    });
    resultats.truncate(MAX_DISPLAY);

    // Conviction LLM minimale pour sauvegarde (qualité > quantité)
    const CONVICTION_MIN: i64 = 65;

    // Auto-save breakout/pré-lancement avec filtre LLM pré-sauvegarde
    for r in resultats.iter().filter(|r| {
        cfg.phases_actives.contains(&r.phase)
            && r.score >= cfg.score_min
            && r.rsi <= cfg.rsi_max
            && r.rsi >= cfg.rsi_min
            && r.ratio_volume >= cfg.ratio_volume_min  // volume confirmé
            && r.ratio_corps >= 0.35 // pas de doji / mèche de rejet
    }) {
        // SL = entrée - ATR14 | TP1 = entrée + ATR14 | TP2 = entrée + 2×ATR14 | TP3 = entrée + 20×ATR14
        let sl = r.prix - r.atr14;
        let tp1 = r.prix + r.atr14;
        let tp2 = r.prix + 2.0 * r.atr14;
        let tp3 = r.prix + 20.0 * r.atr14;

        // Filtre LLM : interroge l'historique du ticker + évalue le setup
        let historique = rockets::historique_ticker(pool, &r.ticker, 10).await;
        let candidat = crate::ollama::rockets_filtre::SignalCandidat {
            ticker: r.ticker.clone(),
            phase: r.phase.clone(),
            score: r.score,
            prix_entree: r.prix,
            stop_loss: sl,
            tp1,
            atr14: r.atr14,
            atr_ratio: r.atr_ratio,
            ratio_volume: r.ratio_volume,
            rsi: r.rsi,
            change1h: r.change1h,
            ratio_corps: r.ratio_corps,
        };

        let (llm_valide, llm_conviction, llm_raison, llm_sl, llm_tp1) =
            match crate::ollama::rockets_filtre::filtrer_signal(&candidat, &historique).await {
                Ok(rep) => {
                    tracing::info!(
                        "LLM filtre {} {}: valide={} conviction={}",
                        r.ticker,
                        r.phase,
                        rep.valide,
                        rep.conviction
                    );
                    if !rep.valide {
                        tracing::info!("LLM rejette {} {}: {}", r.ticker, r.phase, rep.raison);
                        continue;
                    }
                    if rep.conviction < CONVICTION_MIN {
                        tracing::info!(
                            "LLM conviction insuffisante {} {} ({}/100): {}",
                            r.ticker,
                            r.phase,
                            rep.conviction,
                            rep.raison
                        );
                        continue; // Qualité insuffisante — pas de sauvegarde
                    }
                    let sl_s = rep.ajustements.as_ref().and_then(|a| a.sl_suggere);
                    let tp1_s = rep.ajustements.as_ref().and_then(|a| a.tp1_suggere);
                    (
                        Some(true),
                        Some(rep.conviction),
                        Some(rep.raison),
                        sl_s,
                        tp1_s,
                    )
                }
                Err(e) => {
                    // Fallback : Ollama indisponible → sauvegarder sans filtre
                    tracing::warn!("LLM filtre indisponible pour {}: {}", r.ticker, e);
                    (None, None, None, None, None)
                }
            };

        let nouveau = NouveauRocket {
            ticker: r.ticker.clone(),
            phase: r.phase.clone(),
            score: r.score,
            prix_entree: r.prix,
            stop_loss: llm_sl.unwrap_or(sl),
            target: llm_tp1.unwrap_or(tp1),
            target2: Some(tp2),
            target3: Some(tp3),
            ratio_volume: r.ratio_volume,
            atr_ratio: r.atr_ratio,
            atr14: Some(r.atr14),
            rsi: r.rsi,
            llm_valide,
            llm_conviction,
            llm_raison,
            llm_sl_suggere: llm_sl,
            llm_tp1_suggere: llm_tp1,
        };
        if let Err(e) = rockets::sauvegarder(pool, &nouveau).await {
            tracing::warn!("Auto-save rocket {}: {}", r.ticker, e);
        }
    }

    let n = resultats.len();
    *get_scan_results().write().await = resultats;
    tracing::info!("Scan rockets terminé: {} signaux actifs", n);
    Ok(())
}
