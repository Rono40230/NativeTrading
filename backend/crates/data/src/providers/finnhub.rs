use crate::DataProvider;
use chrono::Utc;
use common::{Asset, Candle, Result, Timeframe, TradingError};
use serde_json::Value;

const FINNHUB_API_BASE: &str = "https://finnhub.io/api/v1";

pub struct FinnhubProvider {
    client: reqwest::Client,
    api_key: String,
}

impl FinnhubProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
        }
    }

    /// Résolution Finnhub : 1, 5, 15, 30, 60, D, W, M
    fn vers_resolution(timeframe: &Timeframe) -> &'static str {
        match timeframe {
            Timeframe::M1 => "1",
            Timeframe::M5 => "5",
            Timeframe::M15 => "15",
            Timeframe::H1 => "60",
            Timeframe::H4 => "60", // H4 non dispo → H1 (agrégation frontend)
            Timeframe::D1 => "D",
            Timeframe::W1 => "W",
        }
    }
}

impl Default for FinnhubProvider {
    fn default() -> Self {
        Self::new(std::env::var("FINNHUB_API_KEY").unwrap_or_default())
    }
}

#[async_trait::async_trait]
impl DataProvider for FinnhubProvider {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let symbole = asset.vers_finnhub().ok_or_else(|| {
            TradingError::Data(format!(
                "{} n'est pas disponible sur Finnhub (utiliser Binance)",
                asset.as_str()
            ))
        })?;

        if self.api_key.is_empty() {
            return Err(TradingError::Api(
                "Clé API Finnhub non configurée — aller dans ⚙️ Paramètres".into(),
            ));
        }

        let resolution = Self::vers_resolution(&timeframe);
        let to = Utc::now().timestamp();
        // Calcul du timestamp de début selon la résolution et le nombre de bougies souhaité
        let secondes_par_bougie: i64 = match &timeframe {
            Timeframe::M1 => 60,
            Timeframe::M5 => 300,
            Timeframe::M15 => 900,
            Timeframe::H1 | Timeframe::H4 => 3600,
            Timeframe::D1 => 86400,
            Timeframe::W1 => 604800,
        };
        let from = to - secondes_par_bougie * (limit as i64 + 10);

        let url = format!(
            "{}/forex/candle?symbol={}&resolution={}&from={}&to={}&token={}",
            FINNHUB_API_BASE, symbole, resolution, from, to, self.api_key
        );

        tracing::info!(
            "Finnhub candle: {} resolution={} limit={}",
            symbole,
            resolution,
            limit
        );

        let reponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| TradingError::Api(format!("Finnhub HTTP: {}", e)))?;

        if !reponse.status().is_success() {
            let status = reponse.status();
            let corps = reponse.text().await.unwrap_or_default();
            return Err(TradingError::Api(format!(
                "Finnhub erreur {}: {}",
                status, corps
            )));
        }

        let json: Value = reponse
            .json()
            .await
            .map_err(|e| TradingError::Api(format!("Finnhub parse JSON: {}", e)))?;

        // Vérification statut Finnhub ("ok" ou "no_data")
        if json.get("s").and_then(|v| v.as_str()) != Some("ok") {
            let statut = json.get("s").and_then(|v| v.as_str()).unwrap_or("?");
            return Err(TradingError::Api(format!(
                "Finnhub: statut '{}' — clé invalide ou marché fermé",
                statut
            )));
        }

        let times = json["t"].as_array().ok_or_else(|| TradingError::Api("Finnhub: 't' manquant".into()))?;
        let opens = json["o"].as_array().ok_or_else(|| TradingError::Api("Finnhub: 'o' manquant".into()))?;
        let highs = json["h"].as_array().ok_or_else(|| TradingError::Api("Finnhub: 'h' manquant".into()))?;
        let lows  = json["l"].as_array().ok_or_else(|| TradingError::Api("Finnhub: 'l' manquant".into()))?;
        let closes = json["c"].as_array().ok_or_else(|| TradingError::Api("Finnhub: 'c' manquant".into()))?;
        let volumes = json.get("v").and_then(|v| v.as_array());

        let n = times.len();
        let debut = if n > limit { n - limit } else { 0 };

        let bougies: Vec<Candle> = times[debut..]
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let idx = debut + i;
                let ts = t.as_i64()?;
                let timestamp = chrono::DateTime::from_timestamp(ts, 0)?;
                Some(Candle {
                    timestamp,
                    open:   opens.get(idx)?.as_f64()?,
                    high:   highs.get(idx)?.as_f64()?,
                    low:    lows.get(idx)?.as_f64()?,
                    close:  closes.get(idx)?.as_f64()?,
                    volume: volumes.and_then(|v| v.get(idx)).and_then(|v| v.as_f64()).unwrap_or(0.0),
                })
            })
            .collect();

        tracing::info!("Finnhub: {} bougies reçues pour {}", bougies.len(), symbole);
        Ok(bougies)
    }
}
