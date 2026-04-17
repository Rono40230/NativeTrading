//! Calcul des niveaux d'entrée, d'invalidation et de verdict pour la stratégie Rockets.
//! Séparé de rockets_indicateurs.rs pour respecter la limite de 300 lignes.
//!
//! La logique de progression de position délègue à `position_tracking` (module commun).
//! L'API publique est inchangée — `rockets_suivi.rs` n'a aucune modification à faire.
use db::rockets::RocketSignal;
use crate::position_tracking::{PositionConfig, Verdict, calculer_verdict};

// ── Niveaux d'entrée ─────────────────────────────────────────────────────────

/// Entrée limite : attendre un pullback vers l'ancienne résistance (maintenant support).
/// Breakout → target20 | Compression/Pré-lancement → milieu du range de consolidation
pub fn calculer_entree_limite(prix: f64, target20: f64, support: f64, phase: &str) -> f64 {
    if phase == "breakout" {
        target20 // Pullback vers le niveau cassé, maintenant support
    } else {
        (prix + support) / 2.0 // Milieu de la zone de compression
    }
}

/// Entrée stop : déclenchée sur confirmation de momentum.
/// Breakout → 0.3% au-dessus du prix actuel | Pré-lancement → 0.2% au-dessus de la résistance
pub fn calculer_entree_stop(prix: f64, target20: f64, phase: &str) -> f64 {
    if phase == "breakout" {
        prix * 1.003 // Continuation confirmée par le momentum actuel
    } else {
        target20 * 1.002 // Cassure de la résistance confirmée
    }
}

/// Niveau d'invalidation : en dessous de ce prix, le setup est structurellement mort.
/// = support de base − 0.3× ATR14 (légèrement sous le plancher de consolidation)
pub fn calculer_niveau_invalidation(support: f64, atr14: f64) -> f64 {
    (support - atr14 * 0.3).max(0.0)
}

/// Recommande le type d'entrée algorithmiquement.
/// "stop" si momentum fort (ATR expansion + corps plein + change élevé), "limite" sinon
pub fn recommander_type_entree(atr_ratio: f64, ratio_corps: f64, change1h: f64) -> &'static str {
    if atr_ratio >= 1.2 && ratio_corps >= 0.6 && change1h > 1.5 {
        "stop"
    } else {
        "limite"
    }
}

// ── Logique de progression de position ──────────────────────────────────────

/// Calcule le verdict pour un signal ouvert.
/// `peak`           : max prix atteint depuis l'entrée (tick courant inclus).
/// `peak_precedent` : valeur `prix_peak` en DB avant ce tick (détection premières franchissements).
///
/// Retours :
/// - `Some("TP1")` / `Some("TP2")` → vente partielle — NE PAS fermer la position
/// - `Some("TP3")`  → trailing touché (dès TP2 atteint) — fermer pct_trailing
/// - `Some("invalide")` → SL touché avant ouverture — fermer la position
/// - `None` → rien à faire
///
/// Délègue à `position_tracking::calculer_verdict` — logique commune Rockets / Straddle / SMC.
pub fn calculer_verdict_rocket(
    s: &RocketSignal,
    prix: f64,
    peak: f64,
    peak_precedent: f64,
) -> Option<&'static str> {
    let atr14 = s.atr14.unwrap_or(s.prix_entree * 0.01);
    let trailing_coeff = s.trailing_coeff.unwrap_or(2.0);

    let tp2 = match s.target2 {
        Some(v) => v,
        // Sans TP2 : logique simplifiée — seulement TP1 et SL
        None => {
            // TP1 franchi pour la 1ère fois
            if peak_precedent < s.target && peak >= s.target {
                return Some("TP1");
            }
            // SL
            if peak < s.prix_entree && prix <= s.stop_loss {
                return Some("invalide");
            }
            let sl_eff = if peak >= s.target { s.prix_entree } else { s.stop_loss };
            let label = if peak >= s.target { "be" } else { "sl" };
            if prix <= sl_eff { return Some(label); }
            return None;
        }
    };

    let cfg = PositionConfig {
        prix_entree:     s.prix_entree,
        stop_loss:       s.stop_loss,
        tp1:             s.target,
        tp2,
        atr:             atr14,
        trailing_coeff,
        vente_partielle: true, // Rockets : toujours vente partielle (flag vérifié dans rockets_suivi)
    };

    match calculer_verdict(&cfg, prix, peak, peak_precedent) {
        Verdict::Tp1Partiel                    => Some("TP1"),
        Verdict::Tp2Partiel                    => Some("TP2"),
        Verdict::TrailingTouche { .. }         => Some("TP3"),
        Verdict::Cloture { label: "invalide", .. } => Some("invalide"),
        Verdict::Cloture { label, .. }         => Some(label),
        Verdict::Rien                          => None,
    }
}
