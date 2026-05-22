/// Tests unitaires — Phase 0.1 : correction bug `volume_seche` (ROADMAP §0.1)
/// Tests Phase 1.1 : migration indicateurs dupliqués → crate indicators
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

// ── Tests Phase 1.1 : migration indicateurs dupliqués → crate indicators ─────

/// ROADMAP §1.1 — Test 1 : indicators::calculer_atr produit le même résultat
/// que l'ancien algorithme Wilder local (delta < 0.0001).
#[test]
fn atr_indicateurs_equivalent_algorithme_wilder() {
    use common::Candle;
    use chrono::Utc;

    // Dataset déterministe de 20 bougies
    let candles: Vec<Candle> = (0..20)
        .map(|i| {
            let i = i as f64;
            Candle {
                timestamp: Utc::now(),
                open: 100.0 + i,
                high: 102.0 + i * 0.5,
                low: 99.0 - i * 0.2,
                close: 101.0 + i * 0.3,
                volume: 1000.0,
            }
        })
        .collect();

    // Ancien algorithme Wilder (inline — référence supprimée lors de la migration)
    let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let n = candles.len();
    let trs: Vec<f64> = (1..n)
        .map(|i| {
            let p = closes[i - 1];
            [highs[i] - lows[i], (highs[i] - p).abs(), (lows[i] - p).abs()]
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .collect();
    let p = 14usize;
    let atr14_ref = {
        let mut val = trs[..p].iter().sum::<f64>() / p as f64;
        for &tr in &trs[p..] {
            val = (val * (p as f64 - 1.0) + tr) / p as f64;
        }
        val
    };

    // Nouveau : indicators::calculer_atr
    let atr_vec = indicators::calculer_atr(&candles, 14);
    let atr14_nouveau = atr_vec
        .iter()
        .rev()
        .find(|&&v| !v.is_nan())
        .copied()
        .unwrap_or(0.0);

    let delta = (atr14_ref - atr14_nouveau).abs();
    assert!(
        delta < 0.0001,
        "delta ATR = {delta:.8}, attendu < 0.0001 (équivalence Wilder)"
    );
}

/// ROADMAP §1.1 — Test 2 : scores de phase stables avec les indicateurs migrés.
/// Build un contexte réaliste avec valeurs calculées via indicators crate.
#[test]
fn scores_stables_apres_migration_indicateurs() {
    use common::Candle;
    use chrono::Utc;

    // 50 bougies en compression progressive
    let candles: Vec<Candle> = (0..50)
        .map(|i| {
            let i = i as f64;
            let amp = (1.0 - i * 0.008).max(0.1);
            Candle {
                timestamp: Utc::now(),
                open: 100.0,
                high: 100.0 + amp,
                low: 100.0 - amp,
                close: 100.0 + amp * 0.1,
                volume: 1000.0 * (1.0 - i * 0.01).max(0.1),
            }
        })
        .collect();

    let atr14 = indicators::calculer_atr(&candles, 14)
        .iter()
        .rev()
        .find(|&&v| !v.is_nan())
        .copied()
        .unwrap_or(0.0);
    let rsi = indicators::calculer_rsi(&candles, 14)
        .iter()
        .rev()
        .find(|&&v| !v.is_nan())
        .copied()
        .unwrap_or(50.0);
    let ema20 = indicators::calculer_ema(&candles, 20)
        .iter()
        .rev()
        .find(|&&v| !v.is_nan())
        .copied()
        .unwrap_or(0.0);
    let ema50 = indicators::calculer_ema(&candles, 50)
        .iter()
        .rev()
        .find(|&&v| !v.is_nan())
        .copied()
        .unwrap_or(0.0);

    // ATR court terme < ATR long terme → phase compression
    let atr50 = indicators::calculer_atr(&candles, 50)
        .iter()
        .rev()
        .find(|&&v| !v.is_nan())
        .copied()
        .unwrap_or(0.0);

    assert!(atr14 > 0.0, "ATR14 doit être > 0");
    assert!((0.0..=100.0).contains(&rsi), "RSI doit être dans [0, 100]");

    let ctx = ContextePhase {
        breakout: false,
        ratio_volume: 0.9,
        rsi,
        atr_ratio: 0.60, // < 0.65 → phase prelancement
        change1h: 0.0,
        nb_bougies_compression: 5,
        tendance_haussiere: ema20 > ema50,
        volume_seche: 0.60,
        contraction_qualite: 0.75,
        atr50,
        atr14,
        ratio_corps: 0.50,
    };

    let result = calculer_phase(&ctx, &cfg_defaut());
    assert!(result.is_some(), "calculer_phase doit retourner Some");
    let (phase, score) = result.unwrap();
    assert!(score > 0 && score <= 100, "score hors bornes [1, 100]: {score}");
    assert_eq!(phase, "prelancement");
}

