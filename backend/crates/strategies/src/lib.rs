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
    /// TP1 — objectif principal
    pub take_profit: f64,
    /// TP2 — objectif étendu (optionnel)
    pub take_profit_2: Option<f64>,
    /// TP3 — objectif maximal (optionnel)
    pub take_profit_3: Option<f64>,
}
