//! Analyses LLM stratégiques des signaux Rockets.
//! Séparé de rockets.rs pour respecter la limite de 300 lignes.
use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

use crate::rockets::RocketSignal;

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

/// Retourne les signaux clôturés (hors expire) pour alimenter l'analyse LLM.
pub async fn signaux_pour_analyse(pool: &SqlitePool, limite: i64) -> Result<Vec<RocketSignal>> {
    let rows = sqlx::query(
        "SELECT id, ticker, phase, score, prix_entree, stop_loss, target, target2, target3,
                ratio_volume, atr_ratio, atr14, rsi, statut, prix_peak, verdict, prix_verdict, cree_le, maj_le
         FROM rockets_signaux
         WHERE statut = 'ferme' AND verdict IS NOT NULL AND verdict != 'expire'
         ORDER BY cree_le DESC LIMIT ?",
    )
    .bind(limite)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(crate::rockets::row_to_signal).collect())
}
