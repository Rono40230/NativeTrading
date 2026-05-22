/// Tests unitaires — Phase 0.1 : correction bug `volume_seche` (ROADMAP §0.1)
use db::rockets_config::RocketsConfig;
use strategies::rockets_indicateurs::{calculer_phase, ContextePhase};

/// Contexte minimal : branche prelancement (atr_ratio < 0.65), tous les autres
/// bonus à zéro pour isoler uniquement la contribution de `volume_seche`.
fn ctx_prelancement(volume_seche: f64) -> ContextePhase {
    ContextePhase {
        breakout: false,
        ratio_volume: 1.0, // < 1.3 → pas de bonus
        rsi: 45.0,         // hors [50, 70] → pas de bonus
        atr_ratio: 0.60,   // < 0.65 → phase "prelancement"
        change1h: 0.0,
        nb_bougies_compression: 3, // < 4 → pas de bonus
        tendance_haussiere: false,
        volume_seche,
        contraction_qualite: 0.50, // < 0.70 → pas de bonus
        atr50: 0.0,
        atr14: 1.0,
        ratio_corps: 0.60,
    }
}

fn cfg_defaut() -> RocketsConfig {
    RocketsConfig {
        score_min: 0,
        phases_actives: vec!["prelancement".to_string(), "breakout".to_string()],
        rsi_max: 80.0,
        rsi_min: 30.0,
        ratio_volume_min: 1.5,
        vol_marche_min: 0.0,
        vente_partielle: false,
        sl_mult: 1.0,
        trailing_coeff_min: 2.0,
        trailing_coeff_max: 4.5,
        seuil_score_faible: 60,
        seuil_score_fort: 80,
        pct_cloture_tp1: 0.40,
        pct_cloture_tp2: 0.35,
    }
}

// Base score attendu : ((1.0 - 0.60) * 55.0).round() = 22

#[test]
fn volume_seche_fort_donne_20_pts() {
    // volume_seche = 0.50 < 0.55 → branche forte → +20 pts → total 22 + 20 = 42
    let result = calculer_phase(&ctx_prelancement(0.50), &cfg_defaut());
    let (phase, score) = result.expect("doit retourner Some");
    assert_eq!(phase, "prelancement");
    assert_eq!(score, 42, "assèchement fort attendu à 20 pts");
}

#[test]
fn volume_seche_normal_donne_15_pts() {
    // volume_seche = 0.70 → 0.55 ≤ 0.70 < 0.75 → branche normale → +15 pts → total 22 + 15 = 37
    let result = calculer_phase(&ctx_prelancement(0.70), &cfg_defaut());
    let (phase, score) = result.expect("doit retourner Some");
    assert_eq!(phase, "prelancement");
    assert_eq!(score, 37, "assèchement normal attendu à 15 pts");
}

#[test]
fn volume_seche_absent_donne_0_pts() {
    // volume_seche = 0.80 → ≥ 0.75 → aucun bonus → total 22 + 0 = 22
    let result = calculer_phase(&ctx_prelancement(0.80), &cfg_defaut());
    let (phase, score) = result.expect("doit retourner Some");
    assert_eq!(phase, "prelancement");
    assert_eq!(score, 22, "pas d'assèchement → 0 pts volume");
}
