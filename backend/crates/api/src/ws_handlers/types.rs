use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(super) struct CandleData {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Serialize)]
pub(super) struct CandleEvent {
    pub r#type: &'static str,
    pub asset: String,
    pub timeframe: String,
    pub data: CandleData,
}

// ─── Binance kline WS format ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct BinanceKlineMsg {
    #[serde(rename = "k")]
    pub kline: BinanceKline,
}

#[derive(Deserialize)]
pub(super) struct BinanceKline {
    #[serde(rename = "t")]
    pub open_time_ms: u64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "v")]
    pub volume: String,
}
