//! Tests unitaires pour la machine à états des jambes Straddle.
//! Vérifie que Option 1 (vente partielle) et Option 2 (lot entier)
//! produisent la même progression SL — seul le logging diffère.
use super::*;
use chrono::Utc;
use common::Candle;

fn bougie(open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        timestamp: Utc::now(),
        open,
        high,
        low,
        close,
        volume: 1000.0,
    }
}

// ── Option 1 : TP1 touché → SL déplacé au Break-Even ────────────────────────

/// Scénario : bougie touche TP1 puis s'arrête.
/// SL doit passer au prix d'entrée (Break-Even).
#[test]
fn test_suivi_option1_tp1_deplace_sl() {
    let entree = 100.0;
    let sl = 98.0; // SL initial = entrée - 2
    let tps = [103.0, 104.0, 106.0]; // TP1, TP2, TP3

    let b1 = bougie(100.0, 103.5, 99.0, 103.0); // touche TP1 (high 103.5 >= 103.0)
    let bougies: Vec<&Candle> = vec![&b1];

    let etat = jouer_machine_etats(
        &bougies, sl, entree, &tps, true, // is_long
        true, // vente_partielle = Option 1
        "TEST", "LONG",
    )
    .expect("machine à états doit retourner un état");

    // TP1 atteint : SL doit être au Break-Even (= prix_entree)
    assert_eq!(etat.tps_done, vec!["tp1"]);
    assert!(
        (etat.sl_courant - entree).abs() < 1e-9,
        "Option 1 : SL attendu à {} (break-even), obtenu {}",
        entree,
        etat.sl_courant
    );
    assert!(etat.verdict.is_none(), "pas encore de verdict terminal");
    assert!(etat.etat_change);
}

/// Même scénario avec Option 2 : comportement SL identique.
/// La différence Option 1/2 est uniquement dans vente_partielle (logging).
#[test]
fn test_suivi_option2_pas_vente_partielle() {
    let entree = 100.0;
    let sl = 98.0;
    let tps = [103.0, 104.0, 106.0];

    let b1 = bougie(100.0, 103.5, 99.0, 103.0);
    let bougies: Vec<&Candle> = vec![&b1];

    let etat = jouer_machine_etats(
        &bougies, sl, entree, &tps, true, false, // vente_partielle = Option 2
        "TEST", "LONG",
    )
    .expect("machine à états doit retourner un état");

    // Option 2 : même progression SL que Option 1
    assert_eq!(etat.tps_done, vec!["tp1"]);
    assert!(
        (etat.sl_courant - entree).abs() < 1e-9,
        "Option 2 : SL attendu à {} (break-even), obtenu {}",
        entree,
        etat.sl_courant
    );
    assert!(etat.verdict.is_none());
}

// ── TP2 touché → SL progresse vers TP1 ──────────────────────────────────────

#[test]
fn test_tp2_deplace_sl_en_tp1() {
    let entree = 100.0;
    let sl = 98.0;
    let tps = [103.0, 104.0, 106.0];

    // b1 touche TP1, b2 touche TP2
    let b1 = bougie(100.0, 103.5, 99.0, 103.0);
    let b2 = bougie(103.0, 104.5, 102.5, 104.0);
    let bougies: Vec<&Candle> = vec![&b1, &b2];

    let etat = jouer_machine_etats(&bougies, sl, entree, &tps, true, true, "TEST", "LONG")
        .expect("état valide");

    assert_eq!(etat.tps_done, vec!["tp1", "tp2"]);
    assert!(
        (etat.sl_courant - tps[0]).abs() < 1e-9,
        "Après TP2 : SL attendu en TP1 ({:.1}), obtenu {:.1}",
        tps[0],
        etat.sl_courant
    );
    assert!(etat.verdict.is_none());
}

// ── SL touché avant TP1 → verdict sl ────────────────────────────────────────

#[test]
fn test_sl_touche_avant_tp() {
    let entree = 100.0;
    let sl = 98.0;
    let tps = [103.0, 104.0, 106.0];

    let b1 = bougie(100.0, 101.0, 97.5, 98.5); // low 97.5 <= sl 98.0
    let bougies: Vec<&Candle> = vec![&b1];

    let etat = jouer_machine_etats(&bougies, sl, entree, &tps, true, true, "TEST", "LONG")
        .expect("état valide");

    assert!(etat.tps_done.is_empty());
    assert_eq!(etat.verdict, Some(("sl", sl)));
}

// ── TP3 déclenche verdict terminal ───────────────────────────────────────────

#[test]
fn test_tp3_verdict_terminal() {
    let entree = 100.0;
    let sl = 98.0;
    let tps = [103.0, 104.0, 106.0];

    let b1 = bougie(100.0, 103.5, 99.0, 103.0);
    let b2 = bougie(103.0, 104.5, 102.5, 104.0);
    let b3 = bougie(104.0, 106.5, 103.5, 106.0); // touche TP3
    let bougies: Vec<&Candle> = vec![&b1, &b2, &b3];

    let etat = jouer_machine_etats(&bougies, sl, entree, &tps, true, true, "TEST", "LONG")
        .expect("état valide");

    assert_eq!(etat.verdict, Some(("tp3", tps[2])));
}
