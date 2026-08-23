//! STRADDLE — news trading par ordres stop (Phase 3.1).
//!
//! Définition canonique (recherche 2026-08-17, roadmap Phase 3) :
//! - buy-stop + sell-stop au-delà du range pré-annonce, posés quelques
//!   minutes avant l'annonce tier 1 (NFP/CPI 08:30 ET, FOMC 14:00 ET) ;
//! - **OCO forcé à la seconde du fill** (l'autre ordre est annulé) ;
//! - gestion : SL, TP1/2/3, break-even dès profit = 1R, time-stop 60 min ;
//! - bougie d'annonce attendue à 3-10× ATR normal.
//!
//! Modèle d'exécution : plugin derrière le trait `Engine` — le range se
//! construit sur `on_tick`/`on_close` dans la fenêtre [T-30 min, T-5 min],
//! les ordres sont posés à T-5 min (offset × ATR au-delà du range), le fill
//! se détecte AU TICK (premier prix au-delà d'un stop) et déclenche l'OCO.
//! Les annonces sont INJECTÉES de l'extérieur (`programmer`) — jamais de DB
//! dans le chemin du signal (R4) : le runtime amorce le calendrier.
//!
//! Échelles : les pips forex de la définition canonique sont remplacés par
//! des multiples d'ATR14 (portables BTC/XAU/forex) — valeurs par défaut
//! alignées sur les params DB existants (sl 0,5×, TP 1,5/2,5/5×).

pub mod moteur;
pub mod types;

pub use moteur::StraddleEngine;
pub use types::{Annonce, ParamsStraddle};
