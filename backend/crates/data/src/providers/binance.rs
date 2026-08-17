//! Provider Bybit — données OHLCV pour crypto via API publique (sans clé).
//! Remplace Binance qui est bloqué en France (451 geo-restriction).
//! Bybit : même nommage des paires (BTCUSDT), API publique ouverte.

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Timelike};
use common::{Asset, Candle, Result, Timeframe, TradingError};

use crate::DataProvider;

pub struct BinanceProvider;

impl BinanceProvider {
    /// Symbole Bybit dérivé du ticker — **règle générique, aucune liste
    /// codée** (décision propriétaire 2026-08-15) :
    /// - métaux → contrats linéaires (XAUUSD → XAUUSDT, XAGUSD → XAGUSDT) ;
    /// - crypto → paire USDT standard (TON → TONUSDT) — couvre tout ajout
    ///   futur sans toucher au code.
    /// Note : le symbole autoritaire reste `assets.symbol_bybit` en DB (chemin
    /// WS) ; cette dérivation ne sert qu'au REST (backfill de queue/collecte).
    fn symbole(asset: &Asset) -> Result<String> {
        let ticker = asset.as_str();
        if ticker == "XAUUSD" || ticker == "XAGUSD" {
            Ok(format!("{}USDT", ticker.strip_suffix("USD").unwrap_or(ticker)))
        } else if ticker.ends_with("USD") && ticker.len() > 3 && !ticker.contains('/') {
            // Autres métaux/actifs forex-like mappés en linéaire si ajoutés.
            Ok(format!("{}USDT", ticker.strip_suffix("USD").unwrap_or(ticker)))
        } else {
            Ok(format!("{}USDT", ticker))
        }
    }

    /// Interval Bybit en minutes (1,3,5,15,30,60,120,240,360,720) ou "D","W"
    /// Page de klines STRICTEMENT ANTÉRIEURES à `end_ms` (pagination
    /// descendante, API Bybit v5 `end=`) — brique du backfill profond.
    /// Retourne max 1000 bougies, ordre chronologique. Fonction pure côté
    /// requête : ni DB ni état. Filtrage week-end métaux identique à
    /// fetch_candles.
    pub async fn fetch_page_avant_brute(
        &self,
        asset: &Asset,
        timeframe: Timeframe,
        end_ms: i64,
    ) -> Result<(Vec<Candle>, Option<i64>)> {
        let symbole = Self::symbole(asset)?;
        let interval = Self::interval(&timeframe);
        let category = if matches!(symbole.as_str(), "XAUUSDT" | "XAGUSDT") {
            "linear"
        } else {
            "spot"
        };
        let url = format!(
            "https://api.bybit.com/v5/market/kline?category={}&symbol={}&interval={}&limit=1000&end={}",
            category, symbole, interval, end_ms
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| TradingError::Data(format!("Bybit client error: {}", e)))?;
        let resp = client.get(&url).send().await
            .map_err(|e| TradingError::Data(format!("Bybit réseau: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct BybitResult {
            #[serde(default)]
            list: Vec<Vec<String>>,
        }
        #[derive(serde::Deserialize)]
        struct BybitResp {
            #[serde(default)]
            ret_code: i64,
            #[serde(default)]
            ret_msg: String,
            result: BybitResult,
        }
        let data: BybitResp = resp.json().await
            .map_err(|e| TradingError::Data(format!("Bybit parse JSON: {}", e)))?;
        if data.ret_code != 0 {
            return Err(TradingError::Data(format!(
                "Bybit refuse {} (code {}) : {}", symbole, data.ret_code, data.ret_msg
            )));
        }

        // Plus ancienne bougie BRUTE (avant filtrage) : le curseur de
        // pagination doit descendre même sur une page entièrement filtrée
        // (week-end métaux) — sinon le backfill boucle à vide.
        let plus_ancienne_brute_ms: Option<i64> = data.result.list.iter()
            .filter_map(|row| row.first().and_then(|t| t.parse::<i64>().ok()))
            .min();
        let mut bougies: Vec<Candle> = data.result.list.into_iter().filter_map(|row| {
            let ts_ms: i64 = row.first()?.parse().ok()?;
            let timestamp = DateTime::from_timestamp(ts_ms / 1000, 0)?;
            if matches!(asset.as_str(), "XAUUSD" | "XAGUSD" | "XPTUSD" | "XPDUSD") {
                let w = timestamp.weekday();
                let h = timestamp.hour();
                let is_weekend = match w {
                    chrono::Weekday::Sat => true,
                    chrono::Weekday::Fri if h >= 22 => true,
                    chrono::Weekday::Sun if h < 22 => true,
                    _ => false,
                };
                if is_weekend { return None; }
            }
            Some(Candle {
                timestamp,
                open: row.get(1)?.parse().ok()?,
                high: row.get(2)?.parse().ok()?,
                low: row.get(3)?.parse().ok()?,
                close: row.get(4)?.parse().ok()?,
                volume: row.get(5).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
            })
        }).collect();
        bougies.sort_by_key(|c| c.timestamp);
        Ok((bougies, plus_ancienne_brute_ms))
    }

    fn interval(tf: &Timeframe) -> &'static str {
        match tf {
            Timeframe::M1 => "1",
            Timeframe::M5 => "5",
            Timeframe::M15 => "15",
            Timeframe::M30 => "30",
            Timeframe::H1 => "60",
            Timeframe::H4 => "240",
            Timeframe::D1 => "D",
            Timeframe::W1 => "W",
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
        let category = if symbole == "XAUUSDT" || symbole == "XAGUSDT" {
            "linear"
        } else {
            "spot"
        };
        let url = format!(
            "https://api.bybit.com/v5/market/kline?category={}&symbol={}&interval={}&limit={}",
            category, symbole, interval, max
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
        struct BybitResult {
            #[serde(default)]
            list: Vec<Vec<String>>,
        }
        #[derive(serde::Deserialize)]
        struct BybitResp {
            #[serde(default)]
            ret_code: i64,
            #[serde(default)]
            ret_msg: String,
            result: BybitResult,
        }

        let data: BybitResp = resp
            .json()
            .await
            .map_err(|e| TradingError::Data(format!("Bybit parse JSON: {}", e)))?;
        // Bybit répond 200 avec un payload d'erreur métier (ex : symbole non
        // supporté en spot) — message clair plutôt qu'un échec de décodage.
        if data.ret_code != 0 {
            return Err(TradingError::Data(format!(
                "Bybit refuse {} (code {}) : {} — vérifier le symbole/catégorie",
                symbole, data.ret_code, data.ret_msg
            )));
        }

        let mut bougies: Vec<Candle> = data
            .result
            .list
            .into_iter()
            .filter_map(|row| {
                let ts_ms: i64 = row.first()?.parse().ok()?;
                let timestamp = DateTime::from_timestamp(ts_ms / 1000, 0)?;

                // Si c'est un métal (XAU/XAG), on filtre strictement le week-end
                // Vendredi 22h00 UTC au Dimanche 22h00 UTC (horaires classiques).
                if matches!(asset.as_str(), "XAUUSD" | "XAGUSD" | "XPTUSD" | "XPDUSD") {
                    let w = timestamp.weekday();
                    let h = timestamp.hour();
                    let is_weekend = match w {
                        chrono::Weekday::Sat => true,
                        chrono::Weekday::Fri if h >= 22 => true,
                        chrono::Weekday::Sun if h < 22 => true,
                        _ => false,
                    };
                    if is_weekend {
                        return None; // On rejette cette bougie
                    }
                }

                Some(Candle {
                    timestamp,
                    open: row.get(1)?.parse().ok()?,
                    high: row.get(2)?.parse().ok()?,
                    low: row.get(3)?.parse().ok()?,
                    close: row.get(4)?.parse().ok()?,
                    volume: row.get(5)?.parse::<f64>().unwrap_or(0.0),
                })
            })
            .collect();

        // Bybit retourne du plus récent au plus ancien → inverser
        bougies.reverse();

        tracing::info!(
            "Bybit: {} bougies {} pour {}",
            bougies.len(),
            interval,
            symbole
        );
        Ok(bougies)
    }
}
