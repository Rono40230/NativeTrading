//! Journal des replays du moteur v12 (Phase 2.5 ROADMAP) — archive
//! reproductible des runs du harness, avec verdict de parité vs la
//! référence. C'est la matière première de la Gate 2 (méthode R).

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::Database;

/// Résumé d'un run archivé (sans le journal complet).
#[derive(Debug, Serialize)]
pub struct RunReplayResume {
    pub id: i64,
    pub asset: String,
    pub timeframe: String,
    pub simule_ticks: bool,
    pub nb_bougies: i64,
    pub periode_de: i64,
    pub periode_a: i64,
    pub nb_signaux: i64,
    pub nb_evenements: i64,
    pub conforme_reference: bool,
    pub nb_trades_reference: i64,
    pub duree_ms: i64,
    pub cree_le: i64,
}

impl Database {
    /// Archive un run de replay avec son journal complet.
    pub async fn inserer_run_replay(
        &self,
        asset: &str,
        timeframe: &str,
        simule_ticks: bool,
        nb_bougies: usize,
        periode_de: i64,
        periode_a: i64,
        nb_signaux: usize,
        nb_evenements: usize,
        conforme_reference: bool,
        nb_trades_reference: usize,
        duree_ms: u64,
        journal: &Value,
    ) -> anyhow::Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO runtime_replay
                (asset, timeframe, simule_ticks, nb_bougies, periode_de, periode_a,
                 nb_signaux, nb_evenements, conforme_reference, nb_trades_reference,
                 duree_ms, journal, cree_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             RETURNING id",
        )
        .bind(asset)
        .bind(timeframe)
        .bind(simule_ticks as i64)
        .bind(nb_bougies as i64)
        .bind(periode_de)
        .bind(periode_a)
        .bind(nb_signaux as i64)
        .bind(nb_evenements as i64)
        .bind(conforme_reference as i64)
        .bind(nb_trades_reference as i64)
        .bind(duree_ms as i64)
        .bind(journal.to_string())
        .bind(Utc::now().timestamp())
        .fetch_one(self.pool())
        .await?;
        Ok(id)
    }

    /// Derniers runs archivés (résumés, sans journaux).
    pub async fn lister_runs_replay(&self, limite: i64) -> anyhow::Result<Vec<RunReplayResume>> {
        let lignes = sqlx::query_as::<_, (
            i64, String, String, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64,
        )>(
            "SELECT id, asset, timeframe, simule_ticks, nb_bougies, periode_de, periode_a,
                    nb_signaux, nb_evenements, conforme_reference, nb_trades_reference,
                    duree_ms, cree_le
             FROM runtime_replay ORDER BY id DESC LIMIT ?1",
        )
        .bind(limite)
        .fetch_all(self.pool())
        .await?;

        Ok(lignes
            .into_iter()
            .map(|l| RunReplayResume {
                id: l.0,
                asset: l.1,
                timeframe: l.2,
                simule_ticks: l.3 != 0,
                nb_bougies: l.4,
                periode_de: l.5,
                periode_a: l.6,
                nb_signaux: l.7,
                nb_evenements: l.8,
                conforme_reference: l.9 != 0,
                nb_trades_reference: l.10,
                duree_ms: l.11,
                cree_le: l.12,
            })
            .collect())
    }

    /// Journal complet d'un run (JSON brut).
    pub async fn journal_run_replay(&self, id: i64) -> anyhow::Result<Option<Value>> {
        let journal: Option<String> =
            sqlx::query_scalar("SELECT journal FROM runtime_replay WHERE id = ?1")
                .bind(id)
                .fetch_optional(self.pool())
                .await?;
        match journal {
            Some(j) => Ok(Some(serde_json::from_str(&j).unwrap_or(Value::Null))),
            None => Ok(None),
        }
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
    async fn inserer_lister_et_relire_un_run() {
        let db = db_test().await;
        let journal = serde_json::json!({"signaux": [], "evenements": []});
        let id = db
            .inserer_run_replay(
                "XAUUSD", "M15", false, 1920, 1000, 2000, 5, 12, true, 5, 250, &journal,
            )
            .await
            .unwrap();
        assert!(id > 0);

        let runs = db.lister_runs_replay(10).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].asset, "XAUUSD");
        assert!(runs[0].conforme_reference);
        assert_eq!(runs[0].nb_signaux, 5);

        let relu = db.journal_run_replay(id).await.unwrap().unwrap();
        assert!(relu.get("signaux").is_some());

        assert!(db.journal_run_replay(999).await.unwrap().is_none());
    }
}
