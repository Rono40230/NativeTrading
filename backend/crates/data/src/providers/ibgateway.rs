//! Provider IB Gateway — connexion via protocole TWS sur port 4002 (paper) ou 4001 (live)

use async_trait::async_trait;
use chrono::DateTime;
use common::{Asset, Candle, Result, Timeframe, TradingError};
use ibapi::contracts::{Contract, SecurityType};
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::market_data::TradingHours;
use ibapi::Client;

use crate::DataProvider;

// ─── Provider IB Gateway ──────────────────────────────────────────────────────

pub struct IbGatewayProvider {
    adresse: String,
    client_id: i32,
}

impl IbGatewayProvider {
    /// Crée un nouveau provider IB Gateway.
    /// `port` : 4002 (paper) ou 4001 (live)
    /// `client_id` : identifiant unique par connexion (ex: 100)
    pub fn new(port: u16, client_id: i32) -> Self {
        Self {
            adresse: format!("127.0.0.1:{}", port),
            client_id,
        }
    }

    fn vers_contrat(asset: &Asset) -> Contract {
        match asset {
            Asset::BTC => Contract::crypto("BTC").build(),
            Asset::ETH => Contract::crypto("ETH").build(),
            // Métaux précieux — Commodity SMART
            Asset::XAUUSD => Contract {
                symbol: "XAUUSD".into(),
                security_type: SecurityType::Commodity,
                exchange: "SMART".into(),
                currency: "USD".into(),
                ..Default::default()
            },
            Asset::XAGUSD => Contract {
                symbol: "XAGUSD".into(),
                security_type: SecurityType::Commodity,
                exchange: "SMART".into(),
                currency: "USD".into(),
                ..Default::default()
            },
            // Paires Forex — ForexPair IDEALPRO
            Asset::EURUSD => Self::forex_pair("EUR", "USD"),
            Asset::GBPJPY => Self::forex_pair("GBP", "JPY"),
            Asset::CADJPY => Self::forex_pair("CAD", "JPY"),
            Asset::NZDJPY => Self::forex_pair("NZD", "JPY"),
            Asset::USDCAD => Self::forex_pair("USD", "CAD"),
            Asset::USDJPY => Self::forex_pair("USD", "JPY"),
            // Indices — SecurityType::Index (symboles canoniques IB)
            // DAX → EUREX | SPX → CBOE | NAS100 → NQ contfut CME
            Asset::DAX => Contract {
                symbol: "DAX".into(),
                security_type: SecurityType::Index,
                exchange: "EUREX".into(),
                currency: "EUR".into(),
                ..Default::default()
            },
            Asset::NAS100 => Contract {
                symbol: "NQ".into(),
                security_type: SecurityType::ContinuousFuture,
                exchange: "CME".into(),
                currency: "USD".into(),
                ..Default::default()
            },
            Asset::SP500 => Contract {
                symbol: "SPX".into(),
                security_type: SecurityType::Index,
                exchange: "CBOE".into(),
                currency: "USD".into(),
                ..Default::default()
            },
        }
    }

    fn forex_pair(symbole: &str, devise: &str) -> Contract {
        Contract {
            symbol: symbole.into(),
            security_type: SecurityType::ForexPair,
            exchange: "IDEALPRO".into(),
            currency: devise.into(),
            ..Default::default()
        }
    }

    fn vers_bar_size(tf: &Timeframe) -> BarSize {
        match tf {
            Timeframe::M1  => BarSize::Min,
            Timeframe::M5  => BarSize::Min5,
            Timeframe::M15 => BarSize::Min15,
            Timeframe::M30 => BarSize::Min30,
            Timeframe::H1  => BarSize::Hour,
            Timeframe::H4  => BarSize::Hour4,
            Timeframe::D1  => BarSize::Day,
            Timeframe::W1  => BarSize::Week,
        }
    }

    /// Calcule la durée IB nécessaire pour couvrir `limit` bougies du timeframe donné.
    fn vers_duration(tf: &Timeframe, limit: usize) -> Duration {
        let nb = limit as i32;
        match tf {
            // 390 bougies M1 par jour de trading (~6,5h)
            Timeframe::M1  => Duration::days((nb / 390 + 1).max(1) * 2),
            // 78 bougies M5 par jour
            Timeframe::M5  => Duration::days((nb / 78 + 1).max(1) * 2),
            // 26 bougies M15 par jour
            Timeframe::M15 => Duration::days((nb / 26 + 1).max(2) * 2),
            // 13 bougies M30 par jour
            Timeframe::M30 => Duration::days((nb / 13 + 1).max(2) * 2),
            // 6 bougies H1 par jour de trading
            Timeframe::H1  => Duration::days((nb / 6 + 1).max(2) * 2),
            // 1,5 bougie H4 par jour
            Timeframe::H4  => Duration::days((nb * 4 / 6 + 2).max(4)),
            Timeframe::D1  => Duration::days(nb + 14),
            Timeframe::W1  => Duration::weeks(nb + 4),
        }
    }

    /// Sélectionne le type de données selon l'asset.
    /// - Indices / Futures continus      → Trades
    /// - Forex, métaux, crypto           → MidPoint
    fn what_to_show(asset: &Asset) -> WhatToShow {
        match asset {
            Asset::DAX | Asset::NAS100 | Asset::SP500 => WhatToShow::Trades,
            _ => WhatToShow::MidPoint,
        }
    }
}

#[async_trait]
impl DataProvider for IbGatewayProvider {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let adresse = self.adresse.clone();
        let client_id = self.client_id;

        let contrat = Self::vers_contrat(&asset);
        let bar_size = Self::vers_bar_size(&timeframe);
        let duration = Self::vers_duration(&timeframe, limit);
        let what_to_show = Self::what_to_show(&asset);

        tracing::info!(
            "IB Gateway: connexion {} pour {} {}",
            adresse,
            asset.as_str(),
            timeframe.as_str()
        );

        let client = Client::connect(&adresse, client_id)
            .await
            .map_err(|e| TradingError::Data(format!("Connexion IB Gateway échouée ({}): {}", adresse, e)))?;

        let historique = client
            .historical_data(
                &contrat,
                None, // end_date = maintenant
                duration,
                bar_size,
                Some(what_to_show),
                TradingHours::Extended,
            )
            .await
            .map_err(|e| TradingError::Data(format!("Données historiques IB échouées pour {}: {}", asset.as_str(), e)))?;

        let bougies: Vec<Candle> = historique
            .bars
            .into_iter()
            .rev()
            .take(limit)
            .rev()
            .map(|bar| {
                #[allow(deprecated)]
                let timestamp = DateTime::from_timestamp(bar.date.unix_timestamp(), 0)
                    .unwrap_or_default();
                Candle {
                    timestamp,
                    open: bar.open,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    volume: bar.volume,
                }
            })
            .collect();

        tracing::info!(
            "IB Gateway: {} bougies {} récupérées pour {}",
            bougies.len(),
            timeframe.as_str(),
            asset.as_str()
        );

        Ok(bougies)
    }
}
