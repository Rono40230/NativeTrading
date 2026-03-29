//! Tests unitaires pour la progression des positions Rockets.
use super::*;
use db::rockets::RocketSignal;

/// Construit un signal de test minimal
fn signal(
    entree: f64,
    sl: f64,
    tp1: f64,
    tp2: Option<f64>,
    tp3: Option<f64>,
    atr14: f64,
) -> RocketSignal {
    RocketSignal {
        id: 1,
        ticker: "TEST".into(),
        phase: "breakout".into(),
        score: 75,
        prix_entree: entree,
        stop_loss: sl,
        target: tp1,
        target2: tp2,
        target3: tp3,
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
    }
}

// ── Scénario 1 : position simple (TP1 uniquement, sans pyramid) ──────────────

#[test]
fn entre_prix_neutre_aucun_verdict() {
    let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 1.0, 1.0, 1.0), None);
}

#[test]
fn sl_touche_invalide() {
    let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
    assert_eq!(
        calculer_verdict_rocket(&s, 0.89, 0.89, 0.89),
        Some("invalide")
    );
}

#[test]
fn tp1_atteint_sans_tp2_fermeture_totale() {
    // sans TP2 : TP1 = fermeture totale (pas de pyramidal)
    let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 1.10, 1.10, 1.05), Some("TP1"));
}

// ── Scénario 2 : break-even après TP1 (pyramidal complet) ────────────────────

#[test]
fn tp1_atteint_avec_pyramidal_vente_partielle() {
    // avec TP2 + TP3 : TP1 = vente partielle ⅓, position reste ouverte
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    // peak_precedent = 1.05 (< TP1), peak = 1.12 (>= TP1) → transition TP1
    assert_eq!(calculer_verdict_rocket(&s, 1.12, 1.12, 1.05), Some("TP1"));
}

#[test]
fn tp1_deja_atteint_pas_de_re_declenchement() {
    // peak_precedent >= TP1 → TP1 déjà compté, ne pas redéclencher
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 1.12, 1.12, 1.10), None);
}

#[test]
fn retour_breakeven_apres_tp1_invalide() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
    // peak = 1.12 → SL = entrée = 1.0, prix retombe à 1.0 → invalide
    let peak = 1.12;
    assert_eq!(
        calculer_verdict_rocket(&s, 1.0, peak, peak),
        Some("invalide")
    );
}

#[test]
fn retour_sous_breakeven_apres_tp1_invalide() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
    let peak = 1.12;
    assert_eq!(
        calculer_verdict_rocket(&s, 0.95, peak, peak),
        Some("invalide")
    );
}

// ── Scénario 3 : TP2 (pyramidal complet) ─────────────────────────────────────

#[test]
fn tp2_premier_franchissement_vente_partielle() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    // peak_precedent = 1.15 (TP1 déjà hit, < TP2), peak = 1.22 (>= TP2) → transition TP2
    let peak_precedent = 1.15;
    let peak = 1.22;
    assert_eq!(
        calculer_verdict_rocket(&s, peak, peak, peak_precedent),
        Some("TP2")
    );
}

#[test]
fn tp2_deja_atteint_pas_de_re_declenchement() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    // peak_precedent = 1.25 (TP2 déjà compté), peak = 1.30 → pas de re-déclenchement
    let peak = 1.30;
    assert_eq!(calculer_verdict_rocket(&s, peak, peak, 1.25), None);
}

#[test]
fn retour_tp1_apres_tp2_invalide() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    // peak = 1.25 → SL = TP1 = 1.10, prix retombe à 1.10 → invalide
    let peak = 1.25;
    assert_eq!(
        calculer_verdict_rocket(&s, 1.10, peak, peak),
        Some("invalide")
    );
}

#[test]
fn entre_tp1_et_tp2_apres_tp2_depasse_aucun_verdict() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    // peak_precedent = peak = 1.25 → aucune transition en cours
    let peak = 1.25;
    assert_eq!(calculer_verdict_rocket(&s, 1.15, peak, peak), None);
}

// ── Scénario 4 : trailing stop TP3 ───────────────────────────────────────────

#[test]
fn tp3_zone_trailing_stop_non_touche() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.60;
    let trailing = peak - 0.05 * 1.5;
    assert_eq!(
        calculer_verdict_rocket(&s, trailing + 0.01, peak, peak),
        None
    );
}

#[test]
fn tp3_zone_trailing_stop_touche() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.60;
    let trailing = peak - 0.05 * 1.5;
    assert_eq!(
        calculer_verdict_rocket(&s, trailing - 0.001, peak, peak),
        Some("TP3")
    );
}
