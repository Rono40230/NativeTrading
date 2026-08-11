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
        is_long: true,
        prix_entree:     s.prix_entree,
        stop_loss:       s.stop_loss,
        tp1:             s.target,
        tp2,
        atr:             atr14,
        trailing_coeff,
        vente_partielle_active: true,
        pct_cloture_tp1: 0.33,
        pct_cloture_tp2: 0.33,
    };

    match calculer_verdict(&cfg, prix, peak, peak_precedent) {
        Verdict::Tp1Partiel { .. }                         => Some("TP1"),
        Verdict::Tp2Partiel { .. }                         => Some("TP2"),
        Verdict::ClotureTotale { label, .. } if label == "trailing" => Some("TP3"),
        Verdict::ClotureTotale { label, .. } if label == "invalide" => Some("invalide"),
        Verdict::ClotureTotale { label, .. } => match label.as_str() {
            "tp1" => Some("tp1"),
            "be"  => Some("be"),
            "sl"  => Some("sl"),
            // Les labels "trailing"/"invalide" sont interceptés plus haut.
            // Garde défensif si position_tracking évolue :
            autre => {
                tracing::warn!(
                    "calculer_verdict_rocket: label ClotureTotale inattendu '{}', \
                     traité comme 'sl'", autre
                );
                Some("sl")
            }
        },
        Verdict::Rien                                      => None,
    }
}

#[cfg(test)]
mod tests_leak {
    use super::*;

    /// Construit un `RocketSignal` LONG minimal (modèle : rockets_suivi_tests.rs).
    /// `tp2` est fourni pour emprunter la branche principale de `calculer_verdict`
    /// (celle qui passe par le `match` contenant le bras `ClotureTotale` fautif).
    fn signal_min_long(entree: f64, sl: f64, tp1: f64, tp2: f64, atr14: f64) -> RocketSignal {
        RocketSignal {
            id: 1,
            ticker: "TEST".into(),
            phase: "breakout".into(),
            score: 75,
            prix_entree: entree,
            stop_loss: sl,
            target: tp1,
            target2: Some(tp2),
            target3: None,
            ratio_volume: 2.0,
            atr_ratio: 1.5,
            atr14: Some(atr14),
            rsi: 60.0,
            statut: "ouvert".into(),
            prix_peak: None,
            verdict: None,
            prix_verdict: None,
            cree_le: "2026-01-01T00:00:00".into(),
            maj_le: None,
            llm_valide: None,
            llm_conviction: None,
            llm_raison: None,
            trailing_coeff: Some(2.0),
        }
    }

    /// Un littéral &'static str est interné dans .rodata : deux appels retournent
    /// le MÊME pointeur. Une String leakée serait une nouvelle allocation tas à
    /// chaque appel → pointeurs différents. Ce test échoue si quelqu'un
    /// reintroduit `.leak()` ou `.to_string().leak()`.
    #[test]
    fn verdict_retourne_un_litteral_partage_pas_une_string_leakee() {
        // Scénario LONG menant à ClotureTotale label "sl" :
        //   prix_entree=100, stop_loss=90, target(TP1)=110, target2(TP2)=120.
        //   peak=105 (entre entrée et TP1 → sl_effectif "sl", pas "invalide").
        //   prix=89  (≤ stop_loss 90 → déclenche la clôture totale).
        let s = signal_min_long(100.0, 90.0, 110.0, 120.0, 5.0);
        let v1 = calculer_verdict_rocket(&s, 89.0, 105.0, 105.0);
        let v2 = calculer_verdict_rocket(&s, 89.0, 105.0, 105.0);

        assert!(v1.is_some(), "Le scénario doit produire un verdict");
        assert_eq!(v1, Some("sl"), "Le scénario doit atteindre le label 'sl'");

        let p1 = v1.unwrap().as_ptr();
        let p2 = v2.unwrap().as_ptr();
        assert_eq!(
            p1, p2,
            "Le label doit être un littéral .rodata partagé (pas une String leakée)"
        );
    }

    /// Garde supplémentaire : les labels "be" et "tp1" passent aussi par le match
    /// et doivent retourner des littéraux partagés (anti-leak sur ces bras).
    #[test]
    fn verdict_label_be_est_un_litteral_partage() {
        // Scénario LONG → label "be" : peak entre TP1 et TP2 (sl_effectif = "be"),
        // prix redescend sous le break-even (prix entrée).
        //   prix_entree=100, stop_loss=90, TP1=110, TP2=120.
        //   peak=115 (entre TP1 et TP2 → sl_effectif "be" = prix entrée 100).
        //   prix=99  (≤ sl_effectif 100 → clôture totale label "be").
        let s = signal_min_long(100.0, 90.0, 110.0, 120.0, 5.0);
        let v1 = calculer_verdict_rocket(&s, 99.0, 115.0, 115.0);
        let v2 = calculer_verdict_rocket(&s, 99.0, 115.0, 115.0);
        assert_eq!(v1, Some("be"), "Le scénario doit atteindre le label 'be'");
        assert_eq!(
            v1.unwrap().as_ptr(),
            v2.unwrap().as_ptr(),
            "Le label 'be' doit être un littéral partagé"
        );
    }
}
