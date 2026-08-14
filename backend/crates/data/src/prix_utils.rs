#![allow(unused_variables, dead_code)]
//! Fetch de prix spot dispatché par type d'asset.
//! Crypto + métaux → Bybit/Binance | les autres n'ont pas encore de provider.
use std::sync::Arc;
use std::time::Duration;

// ── Mapping asset → symbole Binance ──────────────────────────────────────────

fn binance_symbol(asset: &str) -> Option<&'static str> {
    match asset {
        "BTC" => Some("BTCUSDT"),
        "ETH" => Some("ETHUSDT"),
        "SOL" => Some("SOLUSDT"),
        "BNB" => Some("BNBUSDT"),
        "XRP" => Some("XRPUSDT"),
        "ADA" => Some("ADAUSDT"),
        "DOGE" => Some("DOGEUSDT"),
        "AVAX" => Some("AVAXUSDT"),
        "LINK" => Some("LINKUSDT"),
        "DOT" => Some("DOTUSDT"),
        "XAUUSD" => Some("XAUUSDT"),
        "XAGUSD" => Some("XAGUSDT"),
        _ => None,
    }
}

// ── Fonctions fetch internes ─────────────────────────────────────────────────

/// Fetch prix spot Binance.
pub async fn fetch_binance(client: &reqwest::Client, symbole: &str) -> Option<f64> {
    let sym = if symbole == "XAUUSD" || symbole == "XAUUSDT" {
        "XAUUSDT".to_string()
    } else if symbole == "XAGUSD" || symbole == "XAGUSDT" {
        "XAGUSDT".to_string()
    } else if symbole.ends_with("USDT") {
        symbole.to_string()
    } else {
        format!("{}USDT", symbole)
    };

    // Bybit API route (évite géo-blocage Binance)
    let category = if sym == "XAUUSDT" || sym == "XAGUSDT" {
        "linear"
    } else {
        "spot"
    };

    let url = format!(
        "https://api.bybit.com/v5/market/tickers?category={}&symbol={}",
        category, sym
    );

    #[derive(serde::Deserialize)]
    struct BybitTickerItem {
        #[serde(rename = "lastPrice")]
        last_price: String,
    }
    #[derive(serde::Deserialize)]
    struct BybitTickerResult {
        list: Vec<BybitTickerItem>,
    }
    #[derive(serde::Deserialize)]
    struct BybitTickerResp {
        result: BybitTickerResult,
    }

    let resp: BybitTickerResp = client
        .get(&url)
        .timeout(Duration::from_millis(1500))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let str_price = resp.result.list.first()?.last_price.clone();
    str_price.parse::<f64>().ok()
}

// ── API publique ─────────────────────────────────────────────────────────────

/// Retourne le prix spot d'un asset selon sa source :
/// crypto + métaux → Bybit/Binance | les autres assets n'ont pas de
/// provider REST (fallback dernier prix en DB côté appelant).
/// Retourne `None` si l'asset est inconnu ou si la source est inaccessible.
pub async fn fetch_prix_asset(client: &reqwest::Client, asset: &str) -> Option<f64> {
    if let Some(sym) = binance_symbol(asset) {
        return fetch_binance(client, sym).await;
    }
    // Fallback : tout asset inconnu est tenté comme paire USDT sur Binance (FRONT, RNDR, etc.)
    let sym = format!("{}USDT", asset);
    fetch_binance(client, &sym).await
}

/// Dernier close connu en DB pour un asset (fallback quand le provider REST
/// est inaccessible).
/// Requête ultra-rapide grâce à l'index (asset, timeframe, timestamp DESC).
pub async fn dernier_prix_db(asset: &str, db: &Arc<db::Database>) -> Option<f64> {
    use common::{Asset, Timeframe};
    let a = Asset::try_from(asset).ok()?;
    // M1 = bougies les plus récentes, sinon M5
    for tf in [Timeframe::M1, Timeframe::M5] {
        if let Ok(bougies) = db.obtenir_bougies(&a, &tf, 1).await {
            if let Some(b) = bougies.last() {
                return Some(b.close);
            }
        }
    }
    None
}
