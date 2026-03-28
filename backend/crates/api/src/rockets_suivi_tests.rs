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

// ── Scénario 1 : position simple (TP1 uniquement) ────────────────────────────

#[test]
fn entre_prix_neutre_aucun_verdict() {
    let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 1.0, 1.0), None);
}

#[test]
fn sl_touche_invalide() {
    let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 0.89, 0.89), Some("invalide"));
}

#[test]
fn tp1_atteint_sans_tp2_fermeture() {
    let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 1.10, 1.10), Some("TP1"));
}

// ── Scénario 2 : break-even après TP1 ────────────────────────────────────────

#[test]
fn prix_sur_tp1_avec_tp2_pas_de_fermeture() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
    assert_eq!(calculer_verdict_rocket(&s, 1.10, 1.10), None);
}

#[test]
fn retour_breakeven_apres_tp1_invalide() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
    let peak = 1.12;
    assert_eq!(calculer_verdict_rocket(&s, 1.0, peak), Some("invalide"));
}

#[test]
fn retour_sous_breakeven_apres_tp1_invalide() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
    let peak = 1.12;
    assert_eq!(calculer_verdict_rocket(&s, 0.95, peak), Some("invalide"));
}

// ── Scénario 3 : progression TP2, SL monte à TP1 ─────────────────────────────

#[test]
fn tp2_atteint_fermeture() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.20;
    assert_eq!(calculer_verdict_rocket(&s, 1.20, peak), Some("TP2"));
}

#[test]
fn retour_tp1_apres_tp2_invalide() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.25;
    assert_eq!(calculer_verdict_rocket(&s, 1.10, peak), Some("invalide"));
}

#[test]
fn entre_tp1_et_tp2_apres_tp2_depasse_aucun_verdict() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.25;
    assert_eq!(calculer_verdict_rocket(&s, 1.15, peak), None);
}

// ── Scénario 4 : trailing stop TP3 ───────────────────────────────────────────

#[test]
fn tp3_zone_trailing_stop_non_touche() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.60;
    let trailing = peak - 0.05 * 1.5;
    assert_eq!(calculer_verdict_rocket(&s, trailing + 0.01, peak), None);
}

#[test]
fn tp3_zone_trailing_stop_touche() {
    let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
    let peak = 1.60;
    let trailing = peak - 0.05 * 1.5;
    assert_eq!(
        calculer_verdict_rocket(&s, trailing - 0.001, peak),
        Some("TP3")
    );
}
