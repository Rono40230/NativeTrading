//! Helpers prix pour le suivi des signaux Rockets.

// ── Prix Binance ──────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct BinancePrix {
    price: String,
}

pub async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

