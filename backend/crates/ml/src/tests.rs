use super::*;
use crate::labels_strategies::{labelliser_rockets, labelliser_smc, labelliser_straddle};
use chrono::Utc;
use common::Candle;

fn bougie(c: f64) -> Candle {
    Candle {
        timestamp: Utc::now(),
        open: c,
        high: c + 1.0,
        low: c - 1.0,
        close: c,
        volume: 100.0,
    }
}

fn bougie_full(open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        timestamp: Utc::now(),
        open,
        high,
        low,
        close,
        volume: 100.0,
    }
}

#[test]
fn pipeline_nouveau_non_pret() {
    let pipeline = PipelineML::new();
    assert!(
        !pipeline.est_pret(),
        "Un pipeline non entraîné ne doit pas être prêt"
    );
}

#[test]
fn predire_erreur_si_pas_assez_de_bougies() {
    let pipeline = PipelineML::new();
    let bougies: Vec<Candle> = (1..=30).map(|i| bougie(i as f64)).collect();
    assert!(pipeline.predire(&bougies).is_err());
}

#[test]
fn predire_erreur_si_modele_non_entraine() {
    let pipeline = PipelineML::new();
    let bougies: Vec<Candle> = (1..=70).map(|i| bougie(i as f64 * 100.0)).collect();
    assert!(pipeline.predire(&bougies).is_err());
}

/// ROADMAP 3.3 — Label Straddle : mouvement ample → 1.0
/// Crée 20 bougies stables (ATR14 ≈ 2.0) puis une bougie avec spike de +10
/// (amplitude = 10 ≥ 2.0 × ATR(≈2) = 4.0) → attendu : 1.0
#[test]
fn label_straddle_mouvement_ample_est_1() {
    // 20 bougies stables : high = c+1, low = c-1 → true range ≈ 2 → ATR14 ≈ 2
    let mut bougies: Vec<Candle> = (0..20).map(|_| bougie(100.0)).collect();
    // bougie future avec spike haussier de +10 (amplitude = 10 >> 2×ATR≈2)
    bougies.push(bougie_full(100.0, 110.0, 99.0, 100.0));
    let index = 19; // dernière bougie stable
    let label = labelliser_straddle(&bougies, index, 1, 2.0);
    assert_eq!(label, Some(1.0), "Amplitude > 2×ATR doit donner label=1.0");
}

/// ROADMAP 3.3 — Label Straddle : mouvement faible → 0.0
#[test]
fn label_straddle_mouvement_faible_est_0() {
    let mut bougies: Vec<Candle> = (0..20).map(|_| bougie(100.0)).collect();
    // spike de seulement +0.5 (amplitude << 2×ATR≈2)
    bougies.push(bougie_full(100.0, 100.5, 99.8, 100.0));
    let index = 19;
    let label = labelliser_straddle(&bougies, index, 1, 2.0);
    assert_eq!(label, Some(0.0), "Amplitude < 2×ATR doit donner label=0.0");
}

/// ROADMAP 3.3 — Label Rockets : breakout +8% en 5 bougies → 1.0
#[test]
fn label_rockets_breakout_8pct_en_5_bougies_est_1() {
    // 20 bougies stables à 100
    let mut bougies: Vec<Candle> = (0..20).map(|_| bougie(100.0)).collect();
    // 4 bougies sans mouvement, puis breakout +9%
    for _ in 0..4 {
        bougies.push(bougie(100.0));
    }
    bougies.push(bougie(109.0)); // +9% > 8%
    let index = 19;
    let label = labelliser_rockets(&bougies, index, 5, 0.08);
    assert_eq!(label, Some(1.0), "Breakout +9% doit donner label=1.0 (seuil 8%)");
}

/// ROADMAP 3.3 — Label Rockets : pas de breakout → 0.0
#[test]
fn label_rockets_sans_breakout_est_0() {
    let mut bougies: Vec<Candle> = (0..20).map(|_| bougie(100.0)).collect();
    for _ in 0..5 {
        bougies.push(bougie(103.0)); // +3% < 8%
    }
    let index = 19;
    let label = labelliser_rockets(&bougies, index, 5, 0.08);
    assert_eq!(label, Some(0.0), "Hausse de +3% ne doit pas déclencher label=1.0 (seuil 8%)");
}

/// ROADMAP 3.3 — Label SMC : tenue jusqu'au TP1 (mouvement +2% haussier) → 1.0
#[test]
fn label_smc_tenue_tp1_haussier_est_1() {
    let mut bougies: Vec<Candle> = (0..20).map(|_| bougie(100.0)).collect();
    bougies.push(bougie(102.0)); // +2%
    let index = 19;
    let label = labelliser_smc(&bougies, index, 1, 0.02, true);
    assert_eq!(label, Some(1.0), "Hausse +2% doit valider TP1 SMC haussier");
}

/// ROADMAP 3.3 — Label SMC : non tenu → 0.0
#[test]
fn label_smc_non_tenu_est_0() {
    let mut bougies: Vec<Candle> = (0..20).map(|_| bougie(100.0)).collect();
    bougies.push(bougie(100.5)); // +0.5% < 2%
    let index = 19;
    let label = labelliser_smc(&bougies, index, 1, 0.02, true);
    assert_eq!(label, Some(0.0), "Hausse +0.5% ne doit pas valider TP1 SMC (seuil 2%)");
}