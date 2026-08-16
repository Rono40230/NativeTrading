//! Crate `engine` — le runtime tick : cœur temps réel de l'application.
//!
//! Conformément à `docs/ROADMAP.md` (Phase 1) :
//! - les prix arrivent événement par événement (tick ou kline en formation) ;
//! - les moteurs ([`Engine`]) sont évalués À CHAQUE événement prix (intrabar) ;
//! - les clôtures de bougies déclenchent `on_close` (confirmations Pine) ;
//! - le chemin du signal ne traverse ni DB ni timer (règle R4).

pub mod agregateur;
pub mod bus;
pub mod engine;
pub mod runtime;
pub mod types;

pub use agregateur::{AgregateurBougie, BougieEnFormation, ModeCloture};
pub use bus::{BougieCloturee, BusBougies, BusEvenements, BusSignaux};
pub use engine::{ContexteCloture, ContexteTick, Engine};
pub use runtime::Runtime;
pub use types::{
    EvenementPrix, EvenementTrade, PrixEvent, SignalBrut, SortieMoteur, Tick, TypeEvenementTrade,
};
