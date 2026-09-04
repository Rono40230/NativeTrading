//! Machine de gestion des trades — commune à toutes les stratégies.
//!
//! Extraite de `smc::v12` (lifecycle + trade) pour que le straddle confie
//! sa jambe survivante à EXACTEMENT le même moteur que la SMC : mêmes
//! règles de fill/SL/BE/TP1/TP2/TP3/trailing/expiration, mêmes verdicts.
//! Le contexte structurel (BOS/MSS opposé, règle de l'un-signal) est fourni
//! par la stratégie hôte via [`HookStructure`] — la SMC l'implémente sur
//! son `SmcOutput` + scoring, le straddle passe [`HookVide`].

pub mod barre;
pub mod lifecycle;
pub mod pondere;
pub mod trade;

pub use barre::BarInput;
pub use lifecycle::{HookStructure, HookVide, ModeBeForce, TradeLifecycle};
pub use pondere::{r_pondere, Fractions};
pub use trade::{CloseReason, Side, Trade, TradeSource, TradeState, Verdict};
