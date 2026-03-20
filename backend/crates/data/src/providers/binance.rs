//! Provider Binance — données OHLCV pour BTC et ETH via API publique (sans clé)

use async_trait::async_trait;
use chrono::DateTime;
use common::{Asset, Candle, Result, Timeframe, TradingError};

use crate::DataProvider;

pub struct BinanceProvider;

impl BinanceProvider {
    fn symbole(asset: &Asset) -> Result<&'static str> {
        match asset {
            Asset::BTC => Ok("BTCUSDT"),
            Asset::ETH => Ok("ETHUSDT"),
            _ => Err(TradingError::Data(
                "BinanceProvider ne supporte que BTC et ETH".into(),
            )),
        }
    }

    fn interval(tf: &Timeframe) -> &'static str {
        match tf {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::M30 => "30m",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1d",
            Timeframe::W1 => "1w",
        }
    }
}

/// Réponse brute Binance : chaque kline est un tableau JSON de 12 éléments
/// [open_time, open, high, low, close, volume, ...]
/// On ne désérialise que les 6 premiers champs utiles.
type KlineRaw = serde_json::Value;

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
            .map_err(|e| TradingError::Data(format!("Binance client error: {}", e)))?;

        tracing::info!("Binance: GET {} ({} bougies demandées)", symbole, limit);

        let mut toutes: Vec<Candle> = Vec::with_capacity(limit);
        let mut end_time_ms: Option<u64> = None; // None = présent

        // Pagination : Binance /api/v3/klines répond au maximum 1000 bougies par requête
        while toutes.len() < limit {
            let restant = limit - toutes.len();
            let batch = restant.min(1000);

            let url = match end_time_ms {
                Some(t) => format!(
                    "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}&endTime={}",
                    symbole, interval, batch, t
                ),
                None => format!(
                    "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
                    symbole, interval, batch
                ),
            };

            let resp = client
                .get(&url)
                .header("User-Agent", "NativeTradingAI/1.0")
                .send()
                .await
                .map_err(|e| TradingError::Data(format!("Binance réseau: {}", e)))?;

            if !resp.status().is_success() {
                return Err(TradingError::Data(format!(
                    "Binance HTTP {}: {}",
                    resp.status(),
                    resp.text().await.unwrap_or_default()
                )));
            }

            let klines: Vec<KlineRaw> = resp
                .json()
                .await
                .map_err(|e| TradingError::Data(format!("Binance parse JSON: {}", e)))?;

            if klines.is_empty() {
                break;
            }

            let mut batch_bougies: Vec<Candle> = klines
                .into_iter()
                .filter_map(|k| {
                    let arr = k.as_array()?;
                    let ts_ms = arr.first()?.as_u64()?;
                    let timestamp = DateTime::from_timestamp((ts_ms / 1000) as i64, 0)?;
                    Some(Candle {
                        timestamp,
                        open: arr.get(1)?.as_str()?.parse().ok()?,
                        high: arr.get(2)?.as_str()?.parse().ok()?,
                        low: arr.get(3)?.as_str()?.parse().ok()?,
                        close: arr.get(4)?.as_str()?.parse().ok()?,
                        volume: arr.get(5)?.as_str()?.parse::<f64>().unwrap_or(0.0),
                    })
                })
                .collect();

            // La plus ancienne bougie du batch → prochain endTime (en excluant cette bougie)
            let plus_ancienne_ms = batch_bougies
                .iter()
                .map(|b| b.timestamp.timestamp_millis() as u64)
                .min();
            let recu = batch_bougies.len();

            toutes.append(&mut batch_bougies);

            match plus_ancienne_ms {
                Some(t) if t > 0 => end_time_ms = Some(t - 1),
                _ => break,
            }

            // Si on a reçu moins que demandé, plus de données disponibles
            if recu < batch {
                break;
            }
        }

        // Tri chronologique et déduplication (chevauchements possibles entre batches)
        toutes.sort_by_key(|b| b.timestamp);
        toutes.dedup_by_key(|b| b.timestamp);

        // Garder les `limit` plus récentes
        let debut = toutes.len().saturating_sub(limit);
        let bougies = toutes.into_iter().skip(debut).collect::<Vec<_>>();

        tracing::info!(
            "Binance: {} bougies {} pour {}",
            bougies.len(),
            interval,
            symbole
        );
        Ok(bougies)
    }
}
