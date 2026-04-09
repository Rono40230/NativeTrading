//! Provider Bybit — données OHLCV pour crypto via API publique (sans clé).
//! Remplace Binance qui est bloqué en France (451 geo-restriction).
//! Bybit : même nommage des paires (BTCUSDT), API publique ouverte.

use async_trait::async_trait;
use chrono::DateTime;
use common::{Asset, Candle, Result, Timeframe, TradingError};

use crate::DataProvider;

pub struct BinanceProvider;

impl BinanceProvider {
    fn symbole(asset: &Asset) -> Result<String> {
        match asset {
            Asset::BTC => Ok("BTCUSDT".into()),
            Asset::ETH => Ok("ETHUSDT".into()),
            Asset::SOL => Ok("SOLUSDT".into()),
            Asset::BNB => Ok("BNBUSDT".into()),
            Asset::XRP => Ok("XRPUSDT".into()),
            Asset::ADA => Ok("ADAUSDT".into()),
            Asset::DOGE => Ok("DOGEUSDT".into()),
            Asset::AVAX => Ok("AVAXUSDT".into()),
            Asset::LINK => Ok("LINKUSDT".into()),
            Asset::DOT => Ok("DOTUSDT".into()),
            _ => Err(TradingError::Data(format!(
                "BinanceProvider: {} n'est pas une crypto Bybit",
                asset.as_str()
            ))),
        }
    }

    /// Interval Bybit en minutes (1,3,5,15,30,60,120,240,360,720) ou "D","W"
    fn interval(tf: &Timeframe) -> &'static str {
        match tf {
            Timeframe::M1  => "1",
            Timeframe::M5  => "5",
            Timeframe::M15 => "15",
            Timeframe::M30 => "30",
            Timeframe::H1  => "60",
            Timeframe::H4  => "240",
            Timeframe::D1  => "D",
            Timeframe::W1  => "W",
        }
    }
}

#[async_trait]
impl DataProvider for BinanceProvider {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let symbole = Self::symbole(&asset)?;
        let interval = Self::interval(&timeframe);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| TradingError::Data(format!("Bybit client error: {}", e)))?;

        tracing::info!("Bybit: GET {} {} ({} bougies)", symbole, interval, limit);

        // Bybit retourne max 1000 bougies par appel — pas de pagination nécessaire ici
        let max = limit.min(1000);
        let url = format!(
            "https://api.bybit.com/v5/market/kline?category=spot&symbol={}&interval={}&limit={}",
            symbole, interval, max
        );

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| TradingError::Data(format!("Bybit réseau: {}", e)))?;

        if !resp.status().is_success() {
            return Err(TradingError::Data(format!(
                "Bybit HTTP {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }

        // Bybit response: { retCode: 0, result: { list: [[startTime, open, high, low, close, volume, turnover], ...] } }
        // list est trié du plus récent au plus ancien — on inverse après parsing
        #[derive(serde::Deserialize)]
        struct BybitResult { list: Vec<Vec<String>> }
        #[derive(serde::Deserialize)]
        struct BybitResp { result: BybitResult }

        let data: BybitResp = resp
            .json()
            .await
            .map_err(|e| TradingError::Data(format!("Bybit parse JSON: {}", e)))?;

        let mut bougies: Vec<Candle> = data.result.list
            .into_iter()
            .filter_map(|row| {
                let ts_ms: i64 = row.first()?.parse().ok()?;
                let timestamp = DateTime::from_timestamp(ts_ms / 1000, 0)?;
                Some(Candle {
                    timestamp,
                    open:   row.get(1)?.parse().ok()?,
                    high:   row.get(2)?.parse().ok()?,
                    low:    row.get(3)?.parse().ok()?,
                    close:  row.get(4)?.parse().ok()?,
                    volume: row.get(5)?.parse::<f64>().unwrap_or(0.0),
                })
            })
            .collect();

        // Bybit retourne du plus récent au plus ancien → inverser
        bougies.reverse();

        tracing::info!(
            "Bybit: {} bougies {} pour {}",
            bougies.len(), interval, symbole
        );
        Ok(bougies)
    }
}

