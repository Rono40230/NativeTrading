use common::{Candle, Direction, Result};

pub mod smc_directional;
pub mod straddle;

pub trait Strategy: Send + Sync {
    fn analyze(&self, candles: &[Candle]) -> Result<Option<Signal>>;
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub direction: Direction,
    pub confidence: f64,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
}
