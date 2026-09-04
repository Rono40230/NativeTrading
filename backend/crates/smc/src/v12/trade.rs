//! Trade + verdicts — délégué à la crate `gestion_trades`.
//! Les chemins `smc::v12::trade::*` restent valides.

pub use gestion_trades::trade::{
    CloseReason, Side, Trade, TradeSource, TradeState, Verdict,
};
