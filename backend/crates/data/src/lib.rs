use common::{Asset, Candle, Result, Timeframe};

pub mod bybit_ws;
pub mod prix_utils;
pub mod providers;
pub mod backfill;
pub mod worker_config;
pub mod worker_status;

#[async_trait::async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_candles(
        &self,
        asset: Asset,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>>;
}
