//! Fetch de prix dispatché par type d'asset.
//! Crypto → Binance | Métaux / Forex / Indices → Yahoo Finance
use std::time::Duration;

// ── Désérialisation Binance ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

// ── Désérialisation Yahoo Finance v8 ────────────────────────────────────────

#[derive(serde::Deserialize)]
struct YahooMeta {
    #[serde(rename = "regularMarketPrice")]
    prix: Option<f64>,
}

#[derive(serde::Deserialize)]
struct YahooResultItem {
    meta: YahooMeta,
}

#[derive(serde::Deserialize)]
struct YahooChartResult {
    result: Option<Vec<YahooResultItem>>,
}

#[derive(serde::Deserialize)]
struct YahooResponse {
    chart: YahooChartResult,
}

// ── Mapping asset → symbole source ──────────────────────────────────────────

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
        _ => None,
    }
}

/// Symboles Yahoo Finance pré-encodés (= → %3D, ^ → %5E).
fn yahoo_symbol(asset: &str) -> Option<&'static str> {
    match asset {
        "XAUUSD" => Some("GC%3DF"),
        "XAGUSD" => Some("SI%3DF"),
        "XPTUSD" => Some("PL%3DF"),
        "XPDUSD" => Some("PA%3DF"),
        "EURUSD" => Some("EURUSD%3DX"),
        "GBPUSD" => Some("GBPUSD%3DX"),
        "USDJPY" => Some("JPY%3DX"),
        "USDCHF" => Some("CHF%3DX"),
        "AUDUSD" => Some("AUDUSD%3DX"),
        "USDCAD" => Some("CAD%3DX"),
        "NZDUSD" => Some("NZDUSD%3DX"),
        "GBPJPY" => Some("GBPJPY%3DX"),
        "CADJPY" => Some("CADJPY%3DX"),
        "NZDJPY" => Some("NZDJPY%3DX"),
        "EURJPY" => Some("EURJPY%3DX"),
        "EURGBP" => Some("EURGBP%3DX"),
        "DAX" => Some("%5EGDAXI"),
        "NAS100" => Some("NQ%3DF"),
        "SP500" => Some("%5EGSPC"),
        "US30" => Some("%5EDJI"),
        "FTSE100" => Some("%5EFTSE"),
        "CAC40" => Some("%5EFCHI"),
        "JP225" => Some("%5EN225"),
        _ => None,
    }
}

// ── Fonctions fetch internes ─────────────────────────────────────────────────

async fn fetch_binance(client: &reqwest::Client, symbole: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}",
        symbole
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

async fn fetch_yahoo(client: &reqwest::Client, symbole: &str) -> Option<f64> {
    let url = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=2d",
        symbole
    );
    let resp: YahooResponse = client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    resp.chart.result?.into_iter().next()?.meta.prix
}

// ── API publique ─────────────────────────────────────────────────────────────

/// Retourne le prix spot d'un asset selon sa source :
/// crypto → Binance | métaux / forex / indices → Yahoo Finance.
/// Retourne `None` si l'asset est inconnu ou si la source est inaccessible.
pub async fn fetch_prix_asset(client: &reqwest::Client, asset: &str) -> Option<f64> {
    if let Some(sym) = binance_symbol(asset) {
        return fetch_binance(client, sym).await;
    }
    if let Some(sym) = yahoo_symbol(asset) {
        return fetch_yahoo(client, sym).await;
    }
    tracing::debug!("fetch_prix_asset: asset inconnu '{}'", asset);
    None
}

/// Crée un client HTTP réutilisable avec timeout 10s.
pub fn client_http() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
}
