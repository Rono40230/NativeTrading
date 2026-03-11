use crate::DataProvider;
use chrono::NaiveDateTime;
use common::{Asset, Candle, Result, Timeframe, TradingError};
use serde_json::Value;

const TWELVEDATA_API_BASE: &str = "https://api.twelvedata.com";

pub struct TwelvedataProvider {
    client: reqwest::Client,
    api_key: String,
}

impl TwelvedataProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
        }
    }

    fn vers_interval(timeframe: &Timeframe) -> &'static str {
        match timeframe {
            Timeframe::M1 => "1min",
            Timeframe::M5 => "5min",
            Timeframe::M15 => "15min",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1day",
            Timeframe::W1 => "1week",
        }
    }
}

impl Default for TwelvedataProvider {
    fn default() -> Self {
        Self::new(std::env::var("TWELVEDATA_API_KEY").unwrap_or_default())
    }
}

#[async_trait::async_trait]
impl DataProvider for TwelvedataProvider {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let symbole = asset.vers_twelvedata().ok_or_else(|| {
            TradingError::Data(format!(
                "{} n'est pas disponible sur Twelvedata (utiliser Binance)",
                asset.as_str()
            ))
        })?;

        if self.api_key.is_empty() {
            return Err(TradingError::Api(
                "Clé API Twelvedata non configurée — aller dans ⚙️ Paramètres".into(),
            ));
        }

        let interval = Self::vers_interval(&timeframe);
        let outputsize = limit.min(5000);

        let url = format!(
            "{}/time_series?symbol={}&interval={}&outputsize={}&apikey={}",
            TWELVEDATA_API_BASE, symbole, interval, outputsize, self.api_key
        );

        tracing::info!(
            "Twelvedata time_series: {} {} outputsize={}",
            symbole,
            interval,
            outputsize
        );

        let reponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| TradingError::Api(format!("Twelvedata HTTP: {}", e)))?;

        if !reponse.status().is_success() {
            let status = reponse.status();
            let corps = reponse.text().await.unwrap_or_default();
            return Err(TradingError::Api(format!(
                "Twelvedata erreur {}: {}",
                status, corps
            )));
        }

        let json: Value = reponse
            .json()
            .await
            .map_err(|e| TradingError::Api(format!("Twelvedata parse JSON: {}", e)))?;

        // Vérification statut API Twelvedata
        if let Some(statut) = json.get("status").and_then(|v| v.as_str()) {
            if statut != "ok" {
                let msg = json
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Erreur inconnue");
                return Err(TradingError::Api(format!("Twelvedata: {}", msg)));
            }
        }

        let valeurs = json
            .get("values")
            .and_then(|v| v.as_array())
            .ok_or_else(|| TradingError::Api("Twelvedata: champ 'values' manquant".into()))?;

        let mut bougies: Vec<Candle> = valeurs.iter().filter_map(parser_bougie).collect();

        // Twelvedata retourne du plus récent au plus ancien → inverser pour ordre ASC
        bougies.reverse();

        tracing::info!(
            "Twelvedata: {} bougies reçues pour {}",
            bougies.len(),
            symbole
        );

        Ok(bougies)
    }
}

fn parser_bougie(v: &Value) -> Option<Candle> {
    let datetime_str = v.get("datetime")?.as_str()?;
    let parse_f64 = |k: &str| v.get(k)?.as_str()?.parse::<f64>().ok();

    // Format: "2024-01-15 10:00:00" (intraday) ou "2024-01-15" (daily)
    let datetime_full = if datetime_str.len() == 10 {
        format!("{} 00:00:00", datetime_str)
    } else {
        datetime_str.to_owned()
    };

    let timestamp = NaiveDateTime::parse_from_str(&datetime_full, "%Y-%m-%d %H:%M:%S")
        .ok()?
        .and_utc();

    Some(Candle {
        timestamp,
        open: parse_f64("open")?,
        high: parse_f64("high")?,
        low: parse_f64("low")?,
        close: parse_f64("close")?,
        volume: parse_f64("volume").unwrap_or(0.0), // XAUUSD/XAGUSD: volume parfois absent
    })
}
