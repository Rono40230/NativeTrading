//! Rétention des données de marché (décision propriétaire 2026-08-15).
//!
//! - `retention_bougies` (JSON `{"M1":24,"M5":24,...}`) : conservation en
//!   MOIS par timeframe. **TF absent du JSON = illimité.** Aucune valeur
//!   imposée par le code — c'est un choix utilisateur, modifiable via la
//!   table `configuration`.
//! - `retention_observation_jours` : journal de diagnostic du runtime
//!   (Gate 1/2), conservation courte en jours.
//!
//! La purge rafraîchit le cache `bougies_stats` et le job appelant déclenche
//! un `VACUUM` si le volume supprimé le justifie (SQLite ne rend pas
//! l'espace disque au simple DELETE).

use std::collections::HashMap;

use chrono::Utc;
use common::Timeframe;

use crate::Database;

/// Clé de configuration : rétention des bougies par TF (JSON, mois).
pub const CLE_RETENTION_BOUVIES: &str = "retention_bougies";
/// Clé de configuration : rétention du journal d'observation (jours).
pub const CLE_RETENTION_OBSERVATION: &str = "retention_observation_jours";

/// Seuil de lignes supprimées déclenchant un VACUUM (restitution disque).
pub const SEUIL_VACUUM: u64 = 50_000;

/// Timeframes gérables par la rétention (ordre canonique).
pub const TOUS_TF: &[Timeframe] = &[
    Timeframe::M1,
    Timeframe::M5,
    Timeframe::M15,
    Timeframe::M30,
    Timeframe::H1,
    Timeframe::H4,
    Timeframe::D1,
    Timeframe::W1,
];

/// Parse une valeur de rétention `'{"M5":24,"H1":0}'` en map TF → mois.
/// Valeur 0 ou négative = illimité pour ce TF (pas d'entrée dans la map).
/// Fonction pure → testable.
pub fn parse_retention(valeur: &str) -> HashMap<Timeframe, i64> {
    let brute: HashMap<String, i64> = serde_json::from_str(valeur).unwrap_or_default();
    TOUS_TF
        .iter()
        .copied()
        .filter_map(|tf| {
            let mois = *brute.get(tf.as_str())?;
            if mois > 0 {
                Some((tf, mois))
            } else {
                None // 0/négatif = illimité
            }
        })
        .collect()
}

/// Sérialise une map de rétention en valeur de configuration.
pub fn serialise_retention(retention: &HashMap<Timeframe, i64>) -> String {
    let mut map: HashMap<&str, i64> = HashMap::new();
    for (tf, mois) in retention {
        map.insert(tf.as_str(), *mois);
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

impl Database {
    /// Lit la rétention configurée. Absence/erreur → map vide = tout
    /// illimité (on ne supprime JAMAIS sans instruction explicite).
    pub async fn lire_retention(&self) -> HashMap<Timeframe, i64> {
        match self.lire_config(CLE_RETENTION_BOUVIES).await {
            Ok(Some(valeur)) => parse_retention(&valeur),
            _ => HashMap::new(),
        }
    }

    /// Purge les bougies au-delà de la rétention configurée, par TF.
    /// Rafraîchit `bougies_stats` pour les TF purgés. Retourne le nombre
    /// total de lignes supprimées.
    pub async fn purger_bougies_expirees(&self, retention: &HashMap<Timeframe, i64>) -> anyhow::Result<u64> {
        let maintenant = Utc::now().timestamp();
        let mut total: u64 = 0;

        for (tf, mois) in retention {
            let cutoff = maintenant - mois * 30 * 24 * 3600; // mois moyens de 30 j
            let tf_str = tf.as_str();

            let supprimees = sqlx::query(
                "DELETE FROM bougies WHERE timeframe = ?1 AND timestamp < ?2",
            )
            .bind(tf_str)
            .bind(cutoff)
            .execute(self.pool())
            .await?
            .rows_affected();

            if supprimees > 0 {
                // Rafraîchit le cache de comptage pour ce TF (même logique
                // que l'insertion — voir bougies.rs).
                sqlx::query(
                    "INSERT INTO bougies_stats (asset, timeframe, nb)
                     SELECT asset, ?1, COUNT(*) FROM bougies WHERE timeframe = ?1 GROUP BY asset
                     ON CONFLICT(asset, timeframe) DO UPDATE SET nb = excluded.nb",
                )
                .bind(tf_str)
                .execute(self.pool())
                .await?;
                sqlx::query("DELETE FROM bougies_stats WHERE timeframe = ?1 AND nb = 0")
                    .bind(tf_str)
                    .execute(self.pool())
                    .await?;

                tracing::info!(
                    "Rétention : {} bougies {} purgées (> {} mois)",
                    supprimees,
                    tf_str,
                    mois
                );
                total += supprimees;
            }
        }
        Ok(total)
    }

    /// Purge le journal d'observation du runtime au-delà de N jours.
    pub async fn purger_observation_expiree(&self, jours: i64) -> anyhow::Result<u64> {
        if jours <= 0 {
            return Ok(0); // illimité
        }
        let cutoff_ms = Utc::now().timestamp_millis() - jours * 24 * 3600 * 1000;
        let supprimees = sqlx::query("DELETE FROM runtime_observation WHERE cloture_le_ms < ?1")
            .bind(cutoff_ms)
            .execute(self.pool())
            .await?
            .rows_affected();
        if supprimees > 0 {
            tracing::info!(
                "Rétention : {} lignes du journal d'observation purgées (> {} jours)",
                supprimees,
                jours
            );
        }
        Ok(supprimees)
    }

    /// Lit la rétention du journal d'observation (jours). Absence = 90
    /// (journal de diagnostic, pas un historique de marché).
    pub async fn lire_retention_observation(&self) -> i64 {
        match self.lire_config(CLE_RETENTION_OBSERVATION).await {
            Ok(Some(valeur)) => valeur.trim().parse().unwrap_or(90),
            _ => 90,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use common::{Asset, Candle};

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    fn bougie_vieillie(asset: &Asset, tf: Timeframe, age_jours: i64, decalage_sec: i64) -> Candle {
        Candle {
            timestamp: Utc::now()
                - chrono::Duration::days(age_jours)
                + chrono::Duration::seconds(decalage_sec),
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        }
    }

    #[test]
    fn parse_retention_nominal() {
        let r = parse_retention(r#"{"M5":24,"H1":0,"M15":3,"ZZZ":99}"#);
        assert_eq!(r.len(), 2, "H1=0 → illimité (absent), ZZZ inconnu ignoré");
        assert_eq!(r.get(&Timeframe::M5), Some(&24));
        assert_eq!(r.get(&Timeframe::M15), Some(&3));
    }

    #[test]
    fn parse_retention_invalide_ou_vide_ne_purge_rien() {
        assert!(parse_retention("pas json").is_empty());
        assert!(parse_retention("{}").is_empty());
    }

    #[test]
    fn serialise_puis_parse_fait_un_tour_complet() {
        let mut r = HashMap::new();
        r.insert(Timeframe::M1, 24);
        r.insert(Timeframe::D1, 24);
        assert_eq!(parse_retention(&serialise_retention(&r)), r);
    }

    #[tokio::test]
    async fn purge_supprime_seulement_les_expirees() {
        let db = db_test().await;
        // 10 bougies M5 de 3 ans (périmées, timestamps distincts) + 10 d'hier
        let vieilles: Vec<Candle> = (0..10)
            .map(|i| bougie_vieillie(&Asset::from("BTC"), Timeframe::M5, 365 * 3, i * 60))
            .collect();
        let recentes: Vec<Candle> = (0..10)
            .map(|i| bougie_vieillie(&Asset::from("BTC"), Timeframe::M5, 1, i * 60))
            .collect();
        db.inserer_bougies(&Asset::from("BTC"), &Timeframe::M5, &vieilles).await.unwrap();
        db.inserer_bougies(&Asset::from("BTC"), &Timeframe::M5, &recentes).await.unwrap();

        let mut retention = HashMap::new();
        retention.insert(Timeframe::M5, 24); // 24 mois

        let supprimees = db.purger_bougies_expirees(&retention).await.unwrap();
        assert_eq!(supprimees, 10, "seules les bougies > 24 mois partent");

        let restantes = db
            .obtenir_bougies(&Asset::from("BTC"), &Timeframe::M5, 100)
            .await
            .unwrap();
        assert_eq!(restantes.len(), 10);
    }

    #[tokio::test]
    async fn tf_illimite_n_est_pas_purge() {
        let db = db_test().await;
        let vieilles: Vec<Candle> = (0..5)
            .map(|i| bougie_vieillie(&Asset::from("BTC"), Timeframe::D1, 365 * 10, i * 86_400))
            .collect();
        db.inserer_bougies(&Asset::from("BTC"), &Timeframe::D1, &vieilles).await.unwrap();

        // Rétention vide = tout illimité : rien n'est supprimé.
        let supprimees = db.purger_bougies_expirees(&HashMap::new()).await.unwrap();
        assert_eq!(supprimees, 0);
    }

    #[tokio::test]
    async fn purge_rafraichit_bougies_stats() {
        let db = db_test().await;
        let mix: Vec<Candle> = vec![
            bougie_vieillie(&Asset::from("BTC"), Timeframe::M5, 365 * 3, 0),
            bougie_vieillie(&Asset::from("BTC"), Timeframe::M5, 1, 60),
        ];
        db.inserer_bougies(&Asset::from("BTC"), &Timeframe::M5, &mix).await.unwrap();

        let mut retention = HashMap::new();
        retention.insert(Timeframe::M5, 24);
        db.purger_bougies_expirees(&retention).await.unwrap();

        let nb: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT nb FROM bougies_stats WHERE asset = 'BTC' AND timeframe = 'M5'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(nb, 1, "le cache de comptage reflète la purge");
    }

    #[tokio::test]
    async fn purge_observation_par_jours() {
        let db = db_test().await;
        // Une observation clôturée il y a 200 jours + une d'hier
        // (timestamps de bougie distincts — clé UNIQUE).
        let ancienne = Utc::now() - chrono::Duration::days(200);
        let recente = Utc::now();
        for (i, (cloture, close)) in [(ancienne, 10.0), (recente, 11.0)].iter().enumerate() {
            db.inserer_observation_runtime(
                &Asset::from("BTC"),
                &Timeframe::M5,
                &Candle {
                    timestamp: Utc.timestamp_opt((i as i64) * 300, 0).unwrap(),
                    open: *close,
                    high: *close,
                    low: *close,
                    close: *close,
                    volume: 1.0,
                },
                "confirmation",
                *cloture,
            )
            .await
            .unwrap();
        }

        let supprimees = db.purger_observation_expiree(90).await.unwrap();
        assert_eq!(supprimees, 1);
        let restantes: i64 =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runtime_observation")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(restantes, 1);
    }
}
