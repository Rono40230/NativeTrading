//! Job de rétention des données (quotidien) — décision propriétaire 2026-08-15.
//!
//! Lit la configuration utilisateur (`retention_bougies` par TF en mois,
//! `retention_observation_jours`), purge ce qui dépasse, rafraîchit le
//! cache `bougies_stats`, et déclenche un `VACUUM` quand le volume
//! supprimé le justifie (SQLite ne rend pas l'espace disque au DELETE).
//!
//! Aucune valeur imposée : sans configuration, rien n'est jamais supprimé.

use std::sync::Arc;
use std::time::Duration;

use db::Database;

/// Délai initial avant le premier passage (laisser l'app démarrer tranquillement).
const DELAI_INITIAL_SEC: u64 = 300;
/// Périodicité du job.
const PERIODE_SEC: u64 = 86_400; // 24 h

static RETENTION_DEMARREE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Démarre le job de rétention en arrière-plan — non bloquant, idempotent.
pub fn demarrer_job_retention(db: Arc<Database>) {
    if RETENTION_DEMARREE.swap(true, std::sync::atomic::Ordering::SeqCst) {
        tracing::warn!("⚠️  Job rétention déjà démarré — second spawn ignoré");
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(DELAI_INITIAL_SEC)).await;
        loop {
            if let Err(e) = passer(db.as_ref()).await {
                tracing::error!("Rétention : échec du passage quotidien ({})", e);
            }
            tokio::time::sleep(Duration::from_secs(PERIODE_SEC)).await;
        }
    });
    tracing::info!("🧹 Job rétention activé (quotidien, piloté par la configuration utilisateur)");
}

/// Un passage : purges + VACUUM conditionnel.
async fn passer(db: &Database) -> anyhow::Result<()> {
    let debut = std::time::Instant::now();

    // 1. Bougies — rétention par TF (mois), choix utilisateur.
    let retention = db.lire_retention().await;
    let bougies_supprimees = db.purger_bougies_expirees(&retention).await?;

    // 2. Journaux de diagnostic du runtime (observations + émissions) :
    //    même fenêtre de rétention — diagnostic, pas historique de marché.
    let jours_observation = db.lire_retention_observation().await;
    let obs_supprimees = db.purger_observation_expiree(jours_observation).await?;
    let emissions_supprimees = db.purger_emissions_expiree(jours_observation).await?;

    // 3. Presse (mois) — articles ET briefs, clé `retention_presse_mois`,
    //    défaut 12. La bibliothèque de presse n'est pas un historique de
    //    marché : fenêtre glissante courte.
    let mois_presse = db.lire_retention_presse().await;
    let presse_supprimees = db.purger_presse_expiree(mois_presse).await?;
    if presse_supprimees > 0 {
        tracing::info!(
            "Rétention : {} lignes de presse purgées (> {} mois)",
            presse_supprimees,
            mois_presse
        );
    }

    // 4. VACUUM si le volume supprimé le justifie — restitue l'espace disque.
    //    Peut prendre du temps sur une grosse base : on le mérite.
    let total_supprimees =
        bougies_supprimees + obs_supprimees + emissions_supprimees + presse_supprimees;
    if total_supprimees >= db::retention::SEUIL_VACUUM {
        tracing::info!(
            "Rétention : VACUUM en cours ({} lignes supprimées)…",
            total_supprimees
        );
        let t_vacuum = std::time::Instant::now();
        sqlx::query("VACUUM").execute(db.pool()).await?;
        tracing::info!("Rétention : VACUUM terminé en {:?}", t_vacuum.elapsed());
    }

    if total_supprimees > 0 {
        tracing::info!(
            "Rétention : passage terminé en {:?} ({} bougies + {} observations + {} émissions + {} presse purgées)",
            debut.elapsed(),
            bougies_supprimees,
            obs_supprimees,
            emissions_supprimees,
            presse_supprimees
        );
    }
    Ok(())
}
