use crate::DataProvider;
use chrono::{DateTime, Utc};
use common::{Asset, Candle, Result, Timeframe, TradingError};
use serde_json::Value;

/// Provider Yahoo Finance pour XAUUSD / XAGUSD (gratuit, sans clé API)
/// Endpoint: https://query1.finance.yahoo.com/v8/finance/chart/{symbol}
pub struct YahooFinanceProvider {
    client: reqwest::Client,
}

impl YahooFinanceProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Symbole Yahoo Finance pour les métaux précieux
    pub fn vers_symbole(asset: &Asset) -> Option<&'static str> {
        match asset {
            Asset::XAUUSD => Some("XAUUSD=X"),
            Asset::XAGUSD => Some("XAGUSD=X"),
            _ => None,
        }
    }

    /// Résolution Yahoo Finance
    fn vers_interval(timeframe: &Timeframe) -> &'static str {
        match timeframe {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::H1 => "60m",
            Timeframe::H4 => "60m", // agrégation côté frontend
            Timeframe::D1 => "1d",
            Timeframe::W1 => "1wk",
        }
    }
}

impl Default for YahooFinanceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DataProvider for YahooFinanceProvider {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let symbole = Self::vers_symbole(&asset).ok_or_else(|| {
            TradingError::Data(format!(
                "{} non supporté sur Yahoo Finance (utiliser Binance)",
                asset.as_str()
            ))
        })?;

        let interval = Self::vers_interval(&timeframe);
        let secondes_par_bougie: i64 = match &timeframe {
            Timeframe::M1 => 60,
            Timeframe::M5 => 300,
            Timeframe::M15 => 900,
            Timeframe::H1 | Timeframe::H4 => 3600,
            Timeframe::D1 => 86400,
            Timeframe::W1 => 604800,
        };
        let to = Utc::now().timestamp();
        let from = to - secondes_par_bougie * (limit as i64 + 20);

        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval={}&period1={}&period2={}&includePrePost=false",
            symbole, interval, from, to
        );

        tracing::info!(
            "Yahoo Finance: {} interval={} limit={}",
            symbole,
            interval,
            limit
        );

        let reponse = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
            )
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| TradingError::Api(format!("Yahoo Finance HTTP: {}", e)))?;

        if !reponse.status().is_success() {
            let status = reponse.status();
            let corps = reponse.text().await.unwrap_or_default();
            return Err(TradingError::Api(format!(
                "Yahoo Finance erreur {}: {}",
                status, corps
            )));
        }

        let json: Value = reponse
            .json()
            .await
            .map_err(|e| TradingError::Api(format!("Yahoo Finance JSON: {}", e)))?;

        // Structure: { chart: { result: [{ timestamp: [...], indicators: { quote: [{ open, high, low, close, volume }] } }] } }
        let result = json
            .pointer("/chart/result/0")
            .ok_or_else(|| TradingError::Api("Yahoo Finance: aucun résultat (marché fermé ?)".into()))?;

        let timestamps = result
            .pointer("/timestamp")
            .and_then(|v| v.as_array())
            .ok_or_else(|| TradingError::Api("Yahoo Finance: timestamps manquants".into()))?;

        let quote = result
            .pointer("/indicators/quote/0")
            .ok_or_else(|| TradingError::Api("Yahoo Finance: cotations manquantes".into()))?;

        let opens = quote["open"].as_array().ok_or_else(|| TradingError::Api("Yahoo Finance: open manquant".into()))?;
        let highs = quote["high"].as_array().ok_or_else(|| TradingError::Api("Yahoo Finance: high manquant".into()))?;
        let lows = quote["low"].as_array().ok_or_else(|| TradingError::Api("Yahoo Finance: low manquant".into()))?;
        let closes = quote["close"].as_array().ok_or_else(|| TradingError::Api("Yahoo Finance: close manquant".into()))?;
        let volumes = quote["volume"].as_array().ok_or_else(|| TradingError::Api("Yahoo Finance: volume manquant".into()))?;

        let mut bougies: Vec<Candle> = timestamps
            .iter()
            .enumerate()
            .filter_map(|(i, ts)| {
                let timestamp_sec = ts.as_i64()?;
                // Ignorer les barres avec OHLC null (données manquantes)
                let open = opens.get(i)?.as_f64()?;
                let high = highs.get(i)?.as_f64()?;
                let low = lows.get(i)?.as_f64()?;
                let close = closes.get(i)?.as_f64()?;
                let volume = volumes.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let timestamp: DateTime<Utc> = DateTime::from_timestamp(timestamp_sec, 0)?;
                Some(Candle {
                    timestamp,
                    open,
                    high,
                    low,
                    close,
                    volume,
                })
            })
            .collect();

        // Garder les `limit` dernières bougies, triées par timestamp croissant
        bougies.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        if bougies.len() > limit {
            bougies = bougies.into_iter().rev().take(limit).rev().collect();
        }

        if bougies.is_empty() {
            return Err(TradingError::Api(
                "Yahoo Finance: aucune donnée reçue (marché fermé ?)".into(),
            ));
        }

        tracing::info!(
            "Yahoo Finance: {} bougies reçues pour {}",
            bougies.len(),
            symbole
        );

        Ok(bougies)
    }
}
