//! GET /api/prix?assets=XAUUSD,BTC,EURUSD
//! Retourne les prix actuels depuis Binance (crypto) ou IG Markets (autres).
//!
//! GET /api/marche/tickers  — proxy Binance 24h
//! GET /api/marche/klines   — proxy Binance OHLCV (sparklines)
//! GET /api/marche/variation1h — variation 1h via Binance 1h kline
use actix_web::{web, HttpResponse, Responder};
use futures_util::future::join_all;
use std::collections::HashMap;

use crate::prix_utils;
use crate::state::AppState;

/// TickerData publié vers le frontend
#[derive(serde::Serialize)]
struct TickerData {
    prix: f64,
    change24h: f64,
    volume24h: f64,
    nb_trades: u64,
}

/// GET /api/marche/tickers
/// Proxifie Binance spot tickers 24h.
/// Retourne `{ "BTC": { prix, change24h, volume24h, nb_trades }, … }`.
pub async fn get_tickers_crypto() -> impl Responder {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("get_tickers_crypto: client HTTP: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    // Binance: GET /api/v3/ticker/24hr → Vec<{ symbol, lastPrice, priceChangePercent, quoteVolume, count }>
    let resp = match client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status().as_u16();
            tracing::warn!("get_tickers_crypto: Binance {}", status);
            return HttpResponse::BadGateway()
                .json(serde_json::json!({ "error": format!("Binance {}", status) }));
        }
        Err(e) => {
            tracing::error!("get_tickers_crypto: {}", e);
            return HttpResponse::BadGateway()
                .json(serde_json::json!({ "error": e.to_string() }));
        }
    };

    #[derive(serde::Deserialize)]
    struct BinanceTicker {
        symbol: String,
        #[serde(rename = "lastPrice")]
        last_price: String,
        #[serde(rename = "priceChangePercent")]
        price_change_pct: String,  // déjà en % (ex: "1.23")
        #[serde(rename = "quoteVolume")]
        quote_volume: String,
        count: Option<u64>,
    }

    let data: Vec<BinanceTicker> = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("get_tickers_crypto: parse JSON: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Parse JSON" }));
        }
    };

    let mut tickers: HashMap<String, TickerData> = HashMap::new();
    for item in data {
        // Garder seulement les paires USDT
        if !item.symbol.ends_with("USDT") { continue; }
        // Exclure les tokens levier
        if item.symbol.ends_with("UPUSDT") || item.symbol.ends_with("DOWNUSDT")
            || item.symbol.ends_with("BULLUSDT") || item.symbol.ends_with("BEARUSDT") { continue; }

        let ticker = item.symbol.trim_end_matches("USDT").to_string();
        let prix = item.last_price.parse::<f64>().unwrap_or(0.0);
        let change24h = item.price_change_pct.parse::<f64>().unwrap_or(0.0);
        let volume24h = item.quote_volume.parse::<f64>().unwrap_or(0.0);
        let nb_trades = item.count.unwrap_or(0);

        tickers.insert(ticker, TickerData { prix, change24h, volume24h, nb_trades });
    }

    HttpResponse::Ok().json(tickers)
}

#[derive(serde::Deserialize)]
pub struct PrixQuery {
    pub assets: String,
}

/// GET /api/prix?assets=XAUUSD,BTC,EURUSD
/// Retourne `{ "XAUUSD": 3200.5, "BTC": 85000.0, … }`.
/// Assets inconnus ou sources inaccessibles sont silencieusement omis.
pub async fn get_prix(
    state: web::Data<AppState>,
    query: web::Query<PrixQuery>,
) -> impl Responder {
    let client = match prix_utils::client_http() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client HTTP /api/prix: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    // Validation : uniquement alphanumériques, max 50 assets, noms ≤ 10 chars
    let assets: Vec<String> = query
        .assets
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(50)
        .collect();

    let ig = state.ig_session.clone();
    let db = state.db.clone();

    let futs: Vec<_> = assets
        .iter()
        .map(|asset| {
            let c = client.clone();
            let a = asset.clone();
            let ig = ig.clone();
            let db = db.clone();
            async move { (a.clone(), prix_utils::fetch_prix_asset(&c, &a, &ig, &db).await) }
        })
        .collect();

    let resultats = join_all(futs).await;

    let map: HashMap<String, f64> = resultats
        .into_iter()
        .filter_map(|(asset, prix)| prix.map(|p| (asset, p)))
        .collect();

    HttpResponse::Ok().json(map)
}

// ─── Proxy Binance klines (sparklines) ────────────────────────────────────────

/// Intervalles Binance autorisés (liste blanche)
const INTERVALS_AUTORISES: &[&str] = &[
    "1m","3m","5m","15m","30m","1h","2h","4h","6h","8h","12h","1d","3d","1w","1M",
];

#[derive(serde::Deserialize)]
pub struct KlinesQuery {
    pub symbol: String,
    pub interval: String,
    pub limit: Option<u32>,
}

/// GET /api/marche/klines?symbol=BTC&interval=1h&limit=24
/// Proxifie Binance /api/v3/klines côté backend.
pub async fn get_klines_crypto(query: web::Query<KlinesQuery>) -> impl Responder {
    let symbol = query.symbol.trim().to_uppercase();
    if symbol.is_empty() || symbol.len() > 10 || !symbol.chars().all(|c| c.is_ascii_alphanumeric()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "symbol invalide" }));
    }
    if !INTERVALS_AUTORISES.contains(&query.interval.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "interval invalide" }));
    }

    let limit = query.limit.unwrap_or(100).min(1000);
    let symbol_usdt = format!("{}USDT", symbol);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": "Client HTTP indisponible" })),
    };

    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
        symbol_usdt, query.interval, limit
    );

    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => {
            // Binance retourne Vec<Vec<serde_json::Value>>, ordre chronologique
            let data: Vec<Vec<serde_json::Value>> = match r.json().await {
                Ok(d) => d,
                Err(_) => return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": "Parse JSON" })),
            };
            HttpResponse::Ok().json(data)
        }
        Ok(r) => HttpResponse::BadGateway()
            .json(serde_json::json!({ "error": format!("Binance {}", r.status()) })),
        Err(e) => HttpResponse::BadGateway()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

// ─── Proxy Binance variation 1h ───────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct Variation1hQuery {
    pub symbols: String,
}

/// GET /api/marche/variation1h?symbols=BTC,ETH,SOL
/// Calcule la variation 1h depuis la bougie 1h en cours (close - open) / open * 100.
pub async fn get_variation1h_crypto(query: web::Query<Variation1hQuery>) -> impl Responder {
    let tickers: Vec<String> = query.symbols
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .take(30)
        .collect();

    if tickers.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "symbols requis" }));
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": "Client HTTP indisponible" })),
    };

    // Fetch 1h kline (limit=1 = bougie courante) pour chaque ticker en parallèle
    let futs: Vec<_> = tickers.iter().map(|t| {
        let client = client.clone();
        let symbol = format!("{}USDT", t);
        let ticker = t.clone();
        async move {
            let url = format!(
                "https://api.binance.com/api/v3/klines?symbol={}&interval=1h&limit=1",
                symbol
            );
            // Binance kline: [openTime, open, high, low, close, volume, ...]
            let raw: Vec<Vec<serde_json::Value>> = client.get(&url).send().await.ok()?.json().await.ok()?;
            let row = raw.first()?;
            let open: f64 = row.get(1)?.as_str()?.parse().ok()?;
            let close: f64 = row.get(4)?.as_str()?.parse().ok()?;
            if open == 0.0 { return None; }
            Some((ticker, (close - open) / open * 100.0))
        }
    }).collect();

    let resultats = futures_util::future::join_all(futs).await;
    let variations: HashMap<String, f64> = resultats.into_iter().flatten().collect();
    HttpResponse::Ok().json(variations)
}

