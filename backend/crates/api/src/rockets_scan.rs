use db::rockets::{self, NouveauRocket};
use futures_util::future::join_all;
use serde::Serialize;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;

// ── Constantes ───────────────────────────────────────────────────────────────

const STABLECOINS: &[&str] = &[
    "BUSD", "USDC", "TUSD", "DAI", "USDP", "FDUSD",
    "USDS", "EUR", "GBP", "PAX", "SUSD",
];
const VOL_MIN: f64      = 500_000.0;
const KLINES_N: usize   = 50;
const LOOKBACK: usize   = 20;
const ATR_P: usize      = 14;
const BATCH_SIZE: usize = 20;
const SCAN_SECS: u64    = 5 * 60;
const MAX_DISPLAY: usize = 30;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultat {
    pub symbol:       String,
    pub ticker:       String,
    pub prix:         f64,
    pub change1h:     f64,
    pub phase:        String,
    pub score:        i64,
    pub ratio_volume: f64,
    pub atr_ratio:    f64,
    pub rsi:          f64,
    pub support:      f64,
    pub target20:     f64,
    pub closes:       Vec<f64>,
}

#[derive(serde::Deserialize)]
struct Ticker24h {
    symbol:       String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String,
}

// ── État partagé (lecture depuis le handler HTTP) ────────────────────────────

static SCAN_RESULTS: OnceLock<Arc<RwLock<Vec<ScanResultat>>>> = OnceLock::new();
static TOTAL_CANDIDATS: OnceLock<Arc<RwLock<usize>>> = OnceLock::new();

pub fn get_scan_results() -> Arc<RwLock<Vec<ScanResultat>>> {
    SCAN_RESULTS.get_or_init(|| Arc::new(RwLock::new(vec![]))).clone()
}

pub fn get_total_candidats() -> Arc<RwLock<usize>> {
    TOTAL_CANDIDATS.get_or_init(|| Arc::new(RwLock::new(0))).clone()
}

// ── Indicateurs techniques ───────────────────────────────────────────────────

fn calc_atr(highs: &[f64], lows: &[f64], closes: &[f64]) -> (f64, f64) {
    let n = highs.len().min(lows.len()).min(closes.len());
    if n < 2 { return (0.0, 0.0); }
    let trs: Vec<f64> = (1..n).map(|i| {
        let p = closes[i - 1];
        [highs[i] - lows[i], (highs[i] - p).abs(), (lows[i] - p).abs()]
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max)
    }).collect();
    let atr14 = if trs.len() >= ATR_P {
        trs[trs.len() - ATR_P..].iter().sum::<f64>() / ATR_P as f64
    } else if !trs.is_empty() {
        trs.iter().sum::<f64>() / trs.len() as f64
    } else { 0.0 };
    let atr5 = if trs.len() >= 5 {
        trs[trs.len() - 5..].iter().sum::<f64>() / 5.0
    } else { atr14 };
    (atr14, atr5)
}

fn calc_rsi(closes: &[f64]) -> f64 {
    if closes.len() < 15 { return 50.0; }
    let slice = &closes[closes.len() - 15..];
    let (gains, losses) = slice.windows(2).fold((0.0f64, 0.0f64), |(g, l), w| {
        let d = w[1] - w[0];
        if d > 0.0 { (g + d, l) } else { (g, l - d) }
    });
    if losses == 0.0 { return 100.0; }
    100.0 - 100.0 / (1.0 + gains / losses)
}

// ── Analyse d'un symbole ─────────────────────────────────────────────────────

async fn analyser_symbol(client: &reqwest::Client, ticker: &str) -> Option<ScanResultat> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}USDT&interval=1h&limit={}",
        ticker, KLINES_N
    );
    let raw: Vec<serde_json::Value> =
        client.get(&url).send().await.ok()?.json().await.ok()?;
    if raw.len() < 20 { return None; }

    let parse = |k: &serde_json::Value, idx: usize| -> f64 {
        k[idx].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0)
    };
    let closes:  Vec<f64> = raw.iter().map(|k| parse(k, 4)).collect();
    let highs:   Vec<f64> = raw.iter().map(|k| parse(k, 2)).collect();
    let lows:    Vec<f64> = raw.iter().map(|k| parse(k, 3)).collect();
    let volumes: Vec<f64> = raw.iter().map(|k| parse(k, 5)).collect();
    let prix = *closes.last()?;

    let (atr14, atr5) = calc_atr(&highs, &lows, &closes);
    let atr_ratio = if atr14 > 0.0 { atr5 / atr14 } else { 1.0 };

    let prev_end   = volumes.len().saturating_sub(1);
    let prev_start = prev_end.saturating_sub(LOOKBACK);
    let avg_vol    = if prev_end > prev_start {
        volumes[prev_start..prev_end].iter().sum::<f64>() / (prev_end - prev_start) as f64
    } else { 1.0 };
    let ratio_volume = if avg_vol > 0.0 { volumes.last().copied().unwrap_or(0.0) / avg_vol } else { 1.0 };

    let high_end   = highs.len().saturating_sub(1);
    let high_start = high_end.saturating_sub(LOOKBACK);
    let max_recent = highs[high_start..high_end]
        .iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let breakout  = prix > max_recent && max_recent > 0.0;

    let low_start = lows.len().saturating_sub(LOOKBACK);
    let support   = lows[low_start..].iter().cloned().fold(f64::INFINITY, f64::min);
    let rsi       = calc_rsi(&closes);
    let change1h  = if closes.len() >= 2 {
        let prev = closes[closes.len() - 2];
        if prev > 0.0 { (prix - prev) / prev * 100.0 } else { 0.0 }
    } else { 0.0 };

    let (phase, score) = calculer_phase(breakout, ratio_volume, rsi, atr_ratio, change1h)?;
    let closes_spark = closes[closes.len().saturating_sub(24)..].to_vec();

    Some(ScanResultat {
        symbol: format!("{}USDT", ticker),
        ticker: ticker.to_string(),
        prix, change1h, phase, score,
        ratio_volume, atr_ratio, rsi,
        support, target20: max_recent,
        closes: closes_spark,
    })
}

fn calculer_phase(
    breakout: bool, ratio_volume: f64, rsi: f64, atr_ratio: f64, change1h: f64,
) -> Option<(String, i64)> {
    if breakout && ratio_volume >= 1.5 {
        let mut s = 40i64;
        if ratio_volume >= 2.0        { s += 20; }
        if rsi > 60.0 && rsi <= 85.0  { s += 20; }
        if atr_ratio > 1.0            { s += 10; }
        if change1h > 1.0             { s += 10; }
        Some(("breakout".to_string(), s.min(100)))
    } else if atr_ratio < 0.80 {
        let phase = if atr_ratio < 0.65 { "prelancement" } else { "compression" };
        let mut s = ((1.0 - atr_ratio) * 55.0).round() as i64;
        if ratio_volume >= 1.3        { s += 15; }
        if rsi > 50.0 && rsi < 70.0   { s += 10; }
        if s < 15 { return None; }
        Some((phase.to_string(), s.min(100)))
    } else {
        None
    }
}

// ── Worker de scan ───────────────────────────────────────────────────────────

pub async fn demarrer_worker_scan(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => { tracing::error!("Worker scan HTTP: {}", e); return; }
    };

    loop {
        if let Err(e) = executer_scan(&client, &pool).await {
            tracing::warn!("Scan rockets erreur: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(SCAN_SECS)).await;
    }
}

async fn executer_scan(
    client: &reqwest::Client,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let tickers: Vec<Ticker24h> = client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send().await.context("fetch ticker/24hr")?
        .json().await.context("parse ticker/24hr")?;

    let candidats: Vec<String> = tickers.into_iter()
        .filter(|t| {
            let vol = t.quote_volume.parse::<f64>().unwrap_or(0.0);
            est_eligible(&t.symbol, vol)
        })
        .map(|t| t.symbol[..t.symbol.len() - 4].to_string())
        .collect();

    tracing::info!("Scan rockets: {} candidats", candidats.len());
    *get_total_candidats().write().await = candidats.len();

    let mut resultats: Vec<ScanResultat> = Vec::new();
    for batch in candidats.chunks(BATCH_SIZE) {
        let futs = batch.iter().map(|ticker| analyser_symbol(client, ticker.as_str()));
        let res  = join_all(futs).await;
        resultats.extend(res.into_iter().flatten());
    }

    resultats.sort_by(|a, b| {
        phase_priorite(&b.phase).cmp(&phase_priorite(&a.phase))
            .then(b.score.cmp(&a.score))
    });
    resultats.truncate(MAX_DISPLAY);

    // Auto-save breakout/pré-lancement (le DB déduplique sur 6h)
    for r in resultats.iter().filter(|r| r.phase != "compression") {
        let nouveau = NouveauRocket {
            ticker: r.ticker.clone(), phase: r.phase.clone(), score: r.score,
            prix_entree: r.prix, stop_loss: r.support, target: r.target20,
            target2: None, target3: None,
            ratio_volume: r.ratio_volume, atr_ratio: r.atr_ratio, rsi: r.rsi,
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

fn est_eligible(symbol: &str, quote_volume: f64) -> bool {
    if !symbol.ends_with("USDT") { return false; }
    if symbol.ends_with("UPUSDT") || symbol.ends_with("DOWNUSDT") { return false; }
    if symbol.ends_with("BULLUSDT") || symbol.ends_with("BEARUSDT") { return false; }
    let ticker = &symbol[..symbol.len() - 4];
    !STABLECOINS.contains(&ticker) && quote_volume >= VOL_MIN
}

fn phase_priorite(phase: &str) -> u8 {
    match phase {
        "breakout"     => 2,
        "prelancement" => 1,
        _              => 0,
    }
}
