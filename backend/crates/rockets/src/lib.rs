//! Rockets — stratégie VCP/Minervini × Rocket Hunter (définition canonique
//! docs/reference/rockets/DEFINITION.md, figée le 24/08/2026).
//!
//! DEUX briques :
//!   1. `scanner` — le classement /10 sur bougies D1 (trend template, squeeze
//!      Bollinger, contractions, volumes, Marubozu au pivot, force relative) ;
//!   2. `gestion` — le cycle de vie du trade (stop-limit au pivot,
//!      invalidation, R1 → vendre 50 % + trailing %, sortie).

pub mod classement;
pub mod gestion;
pub mod types;

pub use classement::{classement_rocket, BougieD1, ContexteMarche, ResultatClassement, VerdictRockets};
pub use gestion::{pas_gestion, PositionRocket, VerdictRocket};
pub use types::{ParamsRockets, ProfilRisque};
