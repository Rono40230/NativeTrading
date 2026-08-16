//! Journal des émissions LIVE du runtime (Phase 2.6 — shadow mode).
//!
//! Persistance de chaque signal/événement publié par les moteurs, à
//! l'instant exact de l'émission. Alimente le test de vérité (Gate 2).

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::Database;

/// Ligne d'émission journalisée.
#[derive(Debug, Serialize)]
pub struct Emission {
    pub id: i64,
    #[serde(rename = "type")]
    pub genre: String,
    pub moteur: String,
    pub asset: String,
    pub timeframe: String,
    pub direction: Option<String>,
    pub prix: f64,
    pub stop_loss: Option<f64>,
    pub take_profits: Option<String>,
    pub score: Option<i64>,
    pub raison: Option<String>,
    pub cle_trade: Option<String>,
    pub type_evenement: Option<String>,
    pub detail: Option<String>,
    pub debut_barre: i64,
    pub emis_le: i64,
}

impl Database {
    /// Journalise un signal live.
    #[allow(clippy::too_many_arguments)]
    pub async fn inserer_emission_signal(
        &self,
        moteur: &str,
        asset: &str,
        timeframe: &str,
        direction: String,
        prix: f64,
        stop_loss: f64,
        take_profits: &str,
        score: i32,
        raison: &str,
        debut_barre: i64,
        emis_le: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO runtime_emissions
                (type, moteur, asset, timeframe, direction, prix, stop_loss,
                 take_profits, score, raison, debut_barre, emis_le)
             VALUES ('signal', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(moteur)
        .bind(asset)
        .bind(timeframe)
        .bind(direction)
        .bind(prix)
        .bind(stop_loss)
        .bind(take_profits)
        .bind(score as i64)
        .bind(raison)
        .bind(debut_barre)
        .bind(emis_le.timestamp_millis())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Journalise un événement lifecycle live.
    #[allow(clippy::too_many_arguments)]
    pub async fn inserer_emission_evenement(
        &self,
        moteur: &str,
        asset: &str,
        timeframe: &str,
        cle_trade: &str,
        type_evenement: String,
        detail: &str,
        prix: f64,
        debut_barre: i64,
        emis_le: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO runtime_emissions
                (type, moteur, asset, timeframe, cle_trade, type_evenement,
                 detail, prix, debut_barre, emis_le)
             VALUES ('evenement', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(moteur)
        .bind(asset)
        .bind(timeframe)
        .bind(cle_trade)
        .bind(type_evenement)
        .bind(detail)
        .bind(prix)
        .bind(debut_barre)
        .bind(emis_le.timestamp_millis())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Émissions depuis `depuis_ms` (epoch ms), filtrables par asset/TF et
    /// par type, les plus récentes d'abord (max 1000).
    pub async fn lister_emissions(
        &self,
        depuis_ms: i64,
        asset: Option<&str>,
        timeframe: Option<&str>,
        genre: Option<&str>,
    ) -> anyhow::Result<Vec<Emission>> {
        let lignes = sqlx::query_as::<_, (
            i64, String, String, String, String,
            Option<String>, f64, Option<f64>, Option<String>, Option<i64>,
            Option<String>, Option<String>, Option<String>, Option<String>,
            i64, i64,
        )>(
            "SELECT id, type, moteur, asset, timeframe,
                    direction, prix, stop_loss, take_profits, score,
                    raison, cle_trade, type_evenement, detail,
                    debut_barre, emis_le
             FROM runtime_emissions
             WHERE emis_le >= ?1
               AND (?2 IS NULL OR asset = ?2)
               AND (?3 IS NULL OR timeframe = ?3)
               AND (?4 IS NULL OR type = ?4)
             ORDER BY emis_le DESC
             LIMIT 1000",
        )
        .bind(depuis_ms)
        .bind(asset)
        .bind(timeframe)
        .bind(genre)
        .fetch_all(self.pool())
        .await?;

        Ok(lignes
            .into_iter()
            .map(|l| Emission {
                id: l.0,
                genre: l.1,
                moteur: l.2,
                asset: l.3,
                timeframe: l.4,
                direction: l.5,
                prix: l.6,
                stop_loss: l.7,
                take_profits: l.8,
                score: l.9,
                raison: l.10,
                cle_trade: l.11,
                type_evenement: l.12,
                detail: l.13,
                debut_barre: l.14,
                emis_le: l.15,
            })
            .collect())
    }

    /// Purge des émissions au-delà de N jours (rétention diagnostique).
    pub async fn purger_emissions_expiree(&self, jours: i64) -> anyhow::Result<u64> {
        if jours <= 0 {
            return Ok(0);
        }
        let cutoff_ms = Utc::now().timestamp_millis() - jours * 24 * 3600 * 1000;
        let n = sqlx::query("DELETE FROM runtime_emissions WHERE emis_le < ?1")
            .bind(cutoff_ms)
            .execute(self.pool())
            .await?
            .rows_affected();
        if n > 0 {
            tracing::info!("Rétention : {} émissions runtime purgées (> {} jours)", n, jours);
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    #[tokio::test]
    async fn inserer_et_lister_signaux_et_evenements() {
        let db = db_test().await;
        db.inserer_emission_signal(
            "smc_v12", "BTC", "M15", "Long".into(), 100.0, 95.0, "[110,120,130]", 80,
            "v11-OB BUY", 1000, Utc::now(),
        )
        .await
        .unwrap();
        db.inserer_emission_evenement(
            "smc_v12", "BTC", "M15", "42:0:0:123", "Fill".into(), "retest", 100.0, 1000,
            Utc::now(),
        )
        .await
        .unwrap();

        let toutes = db.lister_emissions(0, None, None, None).await.unwrap();
        assert_eq!(toutes.len(), 2);

        let signaux = db.lister_emissions(0, Some("BTC"), Some("M15"), Some("signal")).await.unwrap();
        assert_eq!(signaux.len(), 1);
        assert_eq!(signaux[0].direction.as_deref(), Some("Long"));

        let evs = db.lister_emissions(0, None, None, Some("evenement")).await.unwrap();
        assert_eq!(evs[0].type_evenement.as_deref(), Some("Fill"));
    }

    #[tokio::test]
    async fn purge_par_jours() {
        let db = db_test().await;
        db.inserer_emission_signal(
            "smc_v12", "BTC", "M15", "Long".into(), 1.0, 1.0, "[]", 1, "r", 0,
            Utc::now() - chrono::Duration::days(200),
        )
        .await
        .unwrap();
        db.inserer_emission_signal(
            "smc_v12", "BTC", "M15", "Long".into(), 2.0, 1.0, "[]", 1, "r", 0,
            Utc::now(),
        )
        .await
        .unwrap();
        assert_eq!(db.purger_emissions_expiree(90).await.unwrap(), 1);
        let restantes = db.lister_emissions(0, None, None, None).await.unwrap();
        assert_eq!(restantes.len(), 1);
    }
}
