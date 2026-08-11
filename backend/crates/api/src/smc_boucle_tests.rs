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
