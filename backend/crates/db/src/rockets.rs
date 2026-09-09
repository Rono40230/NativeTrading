//! Persistance Rockets. Historiquement le v1 (table `rockets_signaux`,
//! purge différée au tranchage ML §11 — des modules feedback/features la
//! lisent encore) ; depuis le recâblage du 05/09 (§10), il ne reste ici
//! que la persistance des analyses LLM stratégiques (`rockets_analyses_llm`).

use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AnalyseLlm {
    pub id: i64,
    pub nb_trades: i64,
    pub synthese: String,
    pub meilleur_setup: Option<String>,
    pub pire_setup: Option<String>,
    pub recommandations: String, // JSON brut
    pub cree_le: String,
}

pub async fn sauvegarder_analyse(
    pool: &SqlitePool,
    nb_trades: i64,
    synthese: &str,
    meilleur_setup: Option<&str>,
    pire_setup: Option<&str>,
    recommandations: &str,
) -> Result<i64> {
    let id = sqlx::query(
        "INSERT INTO rockets_analyses_llm (nb_trades, synthese, meilleur_setup, pire_setup, recommandations)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(nb_trades)
    .bind(synthese)
    .bind(meilleur_setup)
    .bind(pire_setup)
    .bind(recommandations)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?
    .last_insert_rowid();
    Ok(id)
}

pub async fn derniere_analyse(pool: &SqlitePool) -> Result<Option<AnalyseLlm>> {
    let row = sqlx::query(
        "SELECT id, nb_trades, synthese, meilleur_setup, pire_setup, recommandations, cree_le
         FROM rockets_analyses_llm ORDER BY cree_le DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.map(|r| AnalyseLlm {
        id: r.get("id"),
        nb_trades: r.get("nb_trades"),
        synthese: r.get("synthese"),
        meilleur_setup: r.get("meilleur_setup"),
        pire_setup: r.get("pire_setup"),
        recommandations: r.get("recommandations"),
        cree_le: r.get("cree_le"),
    }))
}
