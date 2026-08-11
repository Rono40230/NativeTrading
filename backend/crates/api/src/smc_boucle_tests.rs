//! Tests unitaires pour smc_boucle — idempotence du démarrage + calcul SL/TP.
//! Extrait dans un fichier dédié pour respecter la limite de 300 lignes sur
//! smc_boucle.rs (pattern identique à rockets_suivi_tests.rs).
use super::*;

/// Test de concurrence : 10 lecteurs simultanés sur un RwLock ne causent pas de deadlock.
#[tokio::test]
async fn test_rwlock_concurrent_readers_no_deadlock() {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;

    let shared = Arc::new(RwLock::new(0u64));
    let mut handles = Vec::new();
    for _ in 0..10 {
        let s = shared.clone();
        handles.push(tokio::spawn(async move {
            let guard = s.read().await;
            tokio::time::sleep(Duration::from_millis(5)).await;
            *guard
        }));
    }
    let results = futures_util::future::join_all(handles).await;
    for r in &results {
        assert!(r.is_ok(), "Tâche en erreur ou deadlock détecté");
    }
}

/// Test de performance : join_all sur 20 tâches de 10ms doit terminer en < 200ms
/// (parallélisme réel, pas 20 × 10ms = 200ms séquentiel).
#[tokio::test]
async fn test_join_all_parallelisme_20_assets() {
    use std::time::{Duration, Instant};

    let futs = (0..20_u32).map(|_| async {
        tokio::time::sleep(Duration::from_millis(10)).await;
    });
    let debut = Instant::now();
    futures_util::future::join_all(futs).await;
    let duree = debut.elapsed();
    assert!(
        duree < Duration::from_millis(200),
        "join_all trop lent ({:?}) — pas de parallélisme",
        duree
    );
}

#[test]
fn marquer_smc_demarree_est_idempotente() {
    // RESET (l'état static persiste entre les tests d'un même binaire ;
    // on le remet à false pour ce test isolé).
    SMC_DEMARREE.store(false, std::sync::atomic::Ordering::SeqCst);

    // Premier appel : doit retourner true (autorise le spawn).
    let premier = marquer_smc_demarree();
    // Second appel : doit retourner false (no-op).
    let second = marquer_smc_demarree();

    assert!(premier, "Le premier appel doit autoriser le démarrage");
    assert!(!second, "Le second appel doit être un no-op (anti-double-spawn)");

    // Cleanup pour ne pas polluer les autres tests.
    SMC_DEMARREE.store(false, std::sync::atomic::Ordering::SeqCst);
}

// ── Tests Bug 3 : calculer_sl_tp utilise SmcParams (pas de hardcodage) ────────

#[test]
fn calculer_sl_tp_utilise_les_params_pas_hardcodes() {
    // Params custom : atr_sl = 2.0, atr_tp1 = 3.0
    let params = db::strategies_params::SmcParams {
        atr_sl: 2.0,
        atr_tp1: 3.0,
        ..Default::default()
    };
    let (sl, tp1) = calculer_sl_tp(common::Direction::Long, 100.0, 1.0, &params).unwrap();
    // SL = prix - 2.0*atr = 98.0 (et NON 99.0 = prix - 1.0*atr hardcodé)
    assert!((sl - 98.0).abs() < 1e-9, "SL doit utiliser atr_sl=2.0 → 98.0, obtenu {}", sl);
    // TP1 = prix + 3.0*atr = 103.0 (et NON 101.5 = prix + 1.5*atr hardcodé)
    assert!((tp1 - 103.0).abs() < 1e-9, "TP1 doit utiliser atr_tp1=3.0 → 103.0, obtenu {}", tp1);
}

#[test]
fn calculer_sl_tp_valeurs_par_defaut_sont_correctes() {
    // Garde anti-régression : avec les défauts (atr_sl=1.0, atr_tp1=2.0),
    // TP1 = 102.0 (et non 101.5 comme l'ancien code hardcodé).
    let def = db::strategies_params::SmcParams::default();
    let (_, tp1) = calculer_sl_tp(common::Direction::Long, 100.0, 1.0, &def).unwrap();
    assert!(
        (tp1 - 102.0).abs() < 1e-9,
        "TP1 par défaut doit être 2.0*atr = 102.0 (était 101.5 avant fix)"
    );
}

#[test]
fn calculer_sl_tp_direction_short_est_symetrique() {
    let params =
        db::strategies_params::SmcParams { atr_sl: 2.0, atr_tp1: 3.0, ..Default::default() };
    let (sl, tp1) = calculer_sl_tp(common::Direction::Short, 100.0, 1.0, &params).unwrap();
    assert!((sl - 102.0).abs() < 1e-9, "SL Short = prix + atr_sl = 102.0");
    assert!((tp1 - 97.0).abs() < 1e-9, "TP1 Short = prix - atr_tp1 = 97.0");
}

#[test]
fn calculer_sl_tp_direction_both_retourne_none() {
    let params = db::strategies_params::SmcParams::default();
    assert!(calculer_sl_tp(common::Direction::Both, 100.0, 1.0, &params).is_none());
}
