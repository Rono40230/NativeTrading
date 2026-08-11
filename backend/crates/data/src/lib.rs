use common::{Asset, Candle, Result, Timeframe, TradingError};

pub mod ig_lightstreamer;
pub mod ig_session;
pub mod prix_utils;
pub mod providers;

#[async_trait::async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>>;
}

#[derive(Default)]
pub struct DataAggregator {
    providers: Vec<Box<dyn DataProvider>>,
}

impl DataAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_provider(&mut self, provider: Box<dyn DataProvider>) {
        self.providers.push(provider);
    }

    pub async fn get_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        if self.providers.is_empty() {
            return Err(TradingError::Data("No providers configured".into()));
        }

        self.providers[0]
            .fetch_candles(asset, timeframe, limit)
            .await
    }
}
