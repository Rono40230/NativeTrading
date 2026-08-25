//! Crate `strategies` — briques de calcul partagées par les verticales
//! (Rockets : indicateurs/niveaux/positions/filtres, Straddle : précision,
//! suivi de position commun dans `position_tracking`).

pub mod position_tracking;
pub mod rockets_filtres;
pub mod rockets_indicateurs;
pub mod rockets_niveaux;
pub mod rockets_position;
pub mod straddle_precision;
