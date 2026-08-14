//! Statut runtime des workers d'ingestion — atomiques globaux lisibles par
//! l'API (`GET /api/worker/status`) sans Handle de tâche ni couplage au spawn.
//!
//! Convention : les timestamps Unix sont en secondes ; `0` = inconnu/jamais.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

use chrono::Utc;

/// Compteurs de statut d'un worker. Tous les champs sont atomiques → la
/// structure est constructible en `static` et partagée sans lock.
pub struct StatutWorker {
    /// Worker connecté / opérationnel (session WS ouverte, cycle IG OK).
    pub connecte: AtomicBool,
    /// Nombre d'actifs suivis par la session/le cycle en cours.
    pub nb_assets: AtomicU64,
    /// Timestamp Unix de la dernière connexion réussie (0 = jamais).
    pub derniere_connexion: AtomicI64,
    /// Timestamp Unix de la dernière bougie insérée (0 = aucune).
    pub derniere_bougie: AtomicI64,
    /// Compteur cumulé de bougies insérées depuis le démarrage du process.
    pub bougies_inserees: AtomicU64,
}

/// Snapshot sérialisable — renvoyé tel quel par l'API.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct InstantaneWorker {
    pub connecte: bool,
    pub nb_assets: u64,
    pub derniere_connexion: i64,
    pub derniere_bougie: i64,
    pub bougies_inserees: u64,
}

impl StatutWorker {
    /// Constructeur `const` pour initialisation statique.
    pub const fn vide() -> Self {
        Self {
            connecte: AtomicBool::new(false),
            nb_assets: AtomicU64::new(0),
            derniere_connexion: AtomicI64::new(0),
            derniere_bougie: AtomicI64::new(0),
            bougies_inserees: AtomicU64::new(0),
        }
    }

    /// Marque le worker connecté avec sa liste d'actifs courante.
    pub fn marque_connecte(&self, nb_assets: u64) {
        self.connecte.store(true, Ordering::Relaxed);
        self.nb_assets.store(nb_assets, Ordering::Relaxed);
        self.derniere_connexion
            .store(Utc::now().timestamp(), Ordering::Relaxed);
    }

    /// Marque le worker déconnecté (fin de session, erreur réseau, cycle KO).
    pub fn marque_deconnecte(&self) {
        self.connecte.store(false, Ordering::Relaxed);
    }

    /// Consigne l'insertion réussie d'une bougie (timestamp Unix secondes).
    pub fn consigne_bougie(&self, ts_unix: i64) {
        self.derniere_bougie.store(ts_unix, Ordering::Relaxed);
        self.bougies_inserees.fetch_add(1, Ordering::Relaxed);
    }

    /// Photographie l'état courant (lecture cohérente pour l'API).
    pub fn instantane(&self) -> InstantaneWorker {
        InstantaneWorker {
            connecte: self.connecte.load(Ordering::Relaxed),
            nb_assets: self.nb_assets.load(Ordering::Relaxed),
            derniere_connexion: self.derniere_connexion.load(Ordering::Relaxed),
            derniere_bougie: self.derniere_bougie.load(Ordering::Relaxed),
            bougies_inserees: self.bougies_inserees.load(Ordering::Relaxed),
        }
    }
}

/// Statut du worker Bybit WebSocket.
pub static STATUT_BYBIT: StatutWorker = StatutWorker::vide();
/// Statut du worker IG REST.
pub static STATUT_IG: StatutWorker = StatutWorker::vide();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_de_vie_connecte_deconnecte_bougie() {
        // Statut local (pas la globale) pour ne pas polluer les autres tests.
        let statut = StatutWorker::vide();
        assert!(!statut.instantane().connecte);

        statut.marque_connecte(12);
        let snap = statut.instantane();
        assert!(snap.connecte);
        assert_eq!(snap.nb_assets, 12);
        assert!(snap.derniere_connexion > 0);

        statut.consigne_bougie(1_786_521_600);
        let snap = statut.instantane();
        assert_eq!(snap.derniere_bougie, 1_786_521_600);
        assert_eq!(snap.bougies_inserees, 1);

        statut.marque_deconnecte();
        assert!(!statut.instantane().connecte);
        // Les compteurs survivent à la déconnexion.
        assert_eq!(statut.instantane().bougies_inserees, 1);
    }
}
