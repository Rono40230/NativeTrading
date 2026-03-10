use common::{Candle, Direction};

pub mod fibonacci;
pub mod ifvg;
pub mod imbalance;
pub mod order_blocks;
pub mod tendances;

pub struct SmcIndicators;

impl SmcIndicators {
    pub fn detect_trend(_candles: &[Candle]) -> Direction {
        Direction::Long
    }

    pub fn detect_order_blocks(_candles: &[Candle]) -> Vec<OrderBlock> {
        vec![]
    }

    pub fn detect_imbalances(_candles: &[Candle]) -> Vec<Imbalance> {
        vec![]
    }
}

pub struct OrderBlock {
    pub price: f64,
    pub direction: Direction,
}

pub struct Imbalance {
    pub start: f64,
    pub end: f64,
}
