//! Cache figé de l'ancien scanner Rockets (v1, ML suspendu).
//!
//! Le worker de scan a été supprimé (nettoyage code mort) : le vivant scanner
//! VCP est `rockets_verticale` (D1 + gestion). Ce module ne conserve que les
//! deux getters servis par l'endpoint `/api/rockets/scan` — le cache reste
//! vide tant que personne n'y écrit.

use strategies::rockets_indicateurs::ScanResultat;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

// ── État partagé (lecture depuis le handler HTTP) ────────────────────────────

static SCAN_RESULTS: OnceLock<Arc<RwLock<Vec<ScanResultat>>>> = OnceLock::new();
static TOTAL_CANDIDATS: OnceLock<Arc<RwLock<usize>>> = OnceLock::new();

pub fn get_scan_results() -> Arc<RwLock<Vec<ScanResultat>>> {
    SCAN_RESULTS
        .get_or_init(|| Arc::new(RwLock::new(vec![])))
        .clone()
}

pub fn get_total_candidats() -> Arc<RwLock<usize>> {
    TOTAL_CANDIDATS
        .get_or_init(|| Arc::new(RwLock::new(0)))
        .clone()
}
