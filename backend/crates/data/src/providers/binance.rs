use crate::DataProvider;
use chrono::{TimeZone, Utc};
use common::{Asset, Candle, Result, Timeframe, TradingError};
use serde_json::Value;

const BINANCE_API_BASE: &str = "https://api.binance.com/api/v3";

pub struct BinanceProvider {
    client: reqwest::Client,
}

impl BinanceProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Construit l'URL klines Binance
    fn url_klines(symbole: &str, interval: &str, limit: usize) -> String {
        format!(
            "{}/klines?symbol={}&interval={}&limit={}",
            BINANCE_API_BASE, symbole, interval, limit
        )
    }

    /// Parse un tableau kline Binance en Candle
    fn parser_kline(kline: &Value) -> Option<Candle> {
        let arr = kline.as_array()?;
        // Format: [timestamp_ms, open, high, low, close, volume, ...]
        let ts_ms = arr.first()?.as_i64()?;
        let parse_f64 = |v: &Value| v.as_str()?.parse::<f64>().ok();
        Some(Candle {
            timestamp: Utc.timestamp_millis_opt(ts_ms).single()?,
            open: parse_f64(arr.get(1)?)?,
            high: parse_f64(arr.get(2)?)?,
            low: parse_f64(arr.get(3)?)?,
            close: parse_f64(arr.get(4)?)?,
            volume: parse_f64(arr.get(5)?)?,
        })
    }
}

impl Default for BinanceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DataProvider for BinanceProvider {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let symbole = asset.vers_binance().ok_or_else(|| {
            TradingError::Data(format!(
                "{} n'est pas disponible sur Binance (utiliser MT5)",
                asset.as_str()
            ))
        })?;

        let url = Self::url_klines(symbole, timeframe.vers_binance(), limit.min(1000));

        tracing::info!(
            "Binance klines: {} {} limit={}",
            symbole,
            timeframe.vers_binance(),
            limit
        );

        let reponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| TradingError::Api(format!("Binance HTTP: {}", e)))?;

        if !reponse.status().is_success() {
            let status = reponse.status();
            let corps = reponse.text().await.unwrap_or_default();
            return Err(TradingError::Api(format!(
                "Binance erreur {}: {}",
                status, corps
            )));
        }

        let json: Value = reponse
            .json()
            .await
            .map_err(|e| TradingError::Api(format!("Binance parse JSON: {}", e)))?;

        let klines = json
            .as_array()
            .ok_or_else(|| TradingError::Api("Format klines invalide".into()))?;

        let bougies: Vec<Candle> = klines.iter().filter_map(Self::parser_kline).collect();

        tracing::info!(
            "{} bougies récupérées ({}/{})",
            bougies.len(),
            symbole,
            timeframe.vers_binance()
        );
        Ok(bougies)
    }
}
