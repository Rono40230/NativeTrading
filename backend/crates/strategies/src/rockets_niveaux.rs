//! Calcul des niveaux d'entrée, d'invalidation et de verdict pour la stratégie Rockets.
//! Séparé de rockets_indicateurs.rs pour respecter la limite de 300 lignes.
use db::rockets::RocketSignal;

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
/// - `Some("invalide")` → SL touché — fermer la position
/// - `None` → rien à faire
pub fn calculer_verdict_rocket(
    s: &RocketSignal,
    prix: f64,
    peak: f64,
    peak_precedent: f64,
) -> Option<&'static str> {
    let atr14 = s.atr14.unwrap_or(s.prix_entree * 0.01);
    let trailing_coeff = s.trailing_coeff.unwrap_or(2.0);

    // ── Détection premières franchissements — vente partielle ────────────────
    if let Some(tp2) = s.target2 {
        if peak_precedent < s.target && peak >= s.target {
            return Some("TP1");
        }
        if peak_precedent >= s.target && peak_precedent < tp2 && peak >= tp2 {
            return Some("TP2");
        }
    } else if peak_precedent < s.target && peak >= s.target {
        return Some("TP1");
    }

    // ── Trailing stop : actif dès que TP2 est atteint ────────────────────────
    if let Some(tp2) = s.target2 {
        if peak >= tp2 {
            let trailing_stop = peak - atr14 * trailing_coeff;
            if prix < trailing_stop {
                return Some("TP3");
            }
        }
    }

    // ── SL effectif progressif ───────────────────────────────────────────────
    // peak ≥ target3 (R+3) → SL = TP2
    // peak ≥ target2 (R+2) → SL = TP1
    // peak ≥ target  (R+1) → SL = entrée (BE)
    // sinon               → SL original
    let sl_effectif = match (s.target2, s.target3) {
        (Some(tp2), Some(tp3)) if peak >= tp3 => tp2,
        (Some(tp2), _) if peak >= tp2 => s.target,
        _ if peak >= s.target => s.prix_entree,
        _ => s.stop_loss,
    };

    if prix <= sl_effectif {
        return Some("invalide");
    }

    None
}
