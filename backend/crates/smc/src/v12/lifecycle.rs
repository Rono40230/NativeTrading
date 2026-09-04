//! Cycle de vie des trades — délégué à la crate `gestion_trades`
//! (extraction : même moteur partagé SMC/straddle). Les chemins
//! `smc::v12::lifecycle::*` restent valides pour les consommateurs.

pub use gestion_trades::lifecycle::{
    HookStructure, HookVide, ModeBeForce, TradeLifecycle,
};
