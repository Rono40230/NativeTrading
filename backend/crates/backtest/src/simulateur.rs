//! Simulation fidèle d'une position unique, bougie par bougie.
//!
//! Reproduit exactement la mécanique live :
//! - SL progressif : SL initial → Break-Even après TP1 → TP1 après TP2
//! - Trailing stop actif après TP2
//! - Ventes partielles à TP1 et TP2 (si activées)
//!
//! Convention ambiguïté intra-bougie (SL et TP touchés dans la même bougie) :
//! SL a priorité (conservatif).

use chrono::{DateTime, Utc};
use common::Candle;
use strategies::position_tracking::PositionConfig;

use crate::ResultatTrade;

pub struct ResultatSimulation {
    pub resultat:  ResultatTrade,
    pub ferme_a:   Option<DateTime<Utc>>,
    /// P&L total en R (partielles + clôture finale)
    pub pnl_r:     f64,
}

/// SL effectif selon le peak atteint (mécanique de protection progressive).
fn sl_effectif(is_long: bool, peak: f64, sl: f64, tp1: f64, tp2: f64, entree: f64) -> f64 {
    if is_long {
        if peak >= tp2   { tp1 }
        else if peak >= tp1 { entree }
        else             { sl }
    } else {
        // Short : prix favorable = bas — peak = lowest low
        if peak <= tp2   { tp1 }
        else if peak <= tp1 { entree }
        else             { sl }
    }
}

/// Trailing stop actif après TP2.
fn trailing_sl(is_long: bool, peak: f64, atr: f64, coeff: f64) -> f64 {
    if is_long { peak - atr * coeff }
    else       { peak + atr * coeff }
}

/// Simule une position unique bougie par bougie avec la mécanique live.
///
/// `bougies` : série commençant à la bougie SUIVANT l'entrée.
pub fn simuler_position(cfg: &PositionConfig, bougies: &[Candle]) -> ResultatSimulation {
    let mut peak = cfg.prix_entree;
    let mut tp_atteint = 0u8;          // 0 = aucun, 1 = TP1, 2 = TP2
    let mut pnl_partiel = 0.0f64;
    let mut pos_restante = 1.0f64;     // fraction de position encore ouverte

    for b in bougies {
        let peak_prev = peak;

        // Prix favorable / adverse selon direction
        let (favorable, adverse) = if cfg.is_long {
            (b.high, b.low)
        } else {
            (b.low, b.high)
        };

        // Mise à jour du peak (meilleur prix atteint dans le sens du trade)
        if cfg.is_long { peak = peak.max(favorable); }
        else           { peak = peak.min(favorable); }

        // SL effectif calculé sur le peak PRÉCÉDENT (avant cette bougie)
        let sl_eff = sl_effectif(cfg.is_long, peak_prev, cfg.stop_loss, cfg.tp1, cfg.tp2, cfg.prix_entree);

        // Trailing stop (actif seulement après TP2, calculé sur le peak précédent)
        let a_depasse_tp2 = if cfg.is_long { peak_prev >= cfg.tp2 } else { peak_prev <= cfg.tp2 };
        let sl_final = if a_depasse_tp2 {
            let t = trailing_sl(cfg.is_long, peak_prev, cfg.atr, cfg.trailing_coeff);
            if cfg.is_long { sl_eff.max(t) } else { sl_eff.min(t) }
        } else {
            sl_eff
        };

        // ── CHECK SL ADVERSE EN PREMIER ──────────────────────────────────────
        let sl_touche = if cfg.is_long { adverse <= sl_final }
                        else           { adverse >= sl_final };

        if sl_touche {
            pnl_partiel += cfg.calculer_r(sl_final) * pos_restante;
            let resultat = match tp_atteint {
                2 => ResultatTrade::Tp2,
                1 => ResultatTrade::Tp1,
                _ => ResultatTrade::StopLoss,
            };
            return ResultatSimulation { resultat, ferme_a: Some(b.timestamp), pnl_r: pnl_partiel };
        }

        // ── CHECK TP1 ─────────────────────────────────────────────────────────
        let tp1_franchi = if cfg.is_long { favorable >= cfg.tp1 && peak_prev < cfg.tp1 }
                          else           { favorable <= cfg.tp1 && peak_prev > cfg.tp1 };
        if tp1_franchi && tp_atteint < 1 {
            if cfg.vente_partielle_active && cfg.pct_cloture_tp1 > 0.0 {
                pnl_partiel += cfg.calculer_r(cfg.tp1) * cfg.pct_cloture_tp1;
                pos_restante -= cfg.pct_cloture_tp1;
            }
            tp_atteint = 1;
        }

        // ── CHECK TP2 ─────────────────────────────────────────────────────────
        let tp2_franchi = if cfg.is_long { favorable >= cfg.tp2 && peak_prev < cfg.tp2 }
                          else           { favorable <= cfg.tp2 && peak_prev > cfg.tp2 };
        if tp2_franchi && tp_atteint < 2 {
            if cfg.vente_partielle_active && cfg.pct_cloture_tp2 > 0.0 {
                pnl_partiel += cfg.calculer_r(cfg.tp2) * cfg.pct_cloture_tp2;
                pos_restante -= cfg.pct_cloture_tp2;
            }
            tp_atteint = 2;
        }
    }

    // Horizon épuisé sans clôture
    ResultatSimulation {
        resultat: ResultatTrade::NonFerme,
        ferme_a: None,
        pnl_r: pnl_partiel,
    }
}
