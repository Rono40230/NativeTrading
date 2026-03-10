use super::{Signal, Strategy};
use common::{Candle, Result};

pub struct SmcDirectionalStrategy;

impl Strategy for SmcDirectionalStrategy {
    fn analyze(&self, _candles: &[Candle]) -> Result<Option<Signal>> {
        Ok(None)
    }
}
