pub use crate::rockets_config::{lire_config, sauvegarder_config, RocketsConfig};
pub use crate::rockets_listing::{
    enregistrer_tp_partiel, historique_ticker, lister_actifs, lister_en_attente, lister_ouverts,
    supprimer,
};
use common::{Result, TradingError};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

#[derive(Serialize, Clone)]
pub struct RocketSignal {
    pub id: i64,
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub atr14: Option<f64>,
    pub rsi: f64,
    pub statut: String,
    pub prix_peak: Option<f64>,
    pub verdict: Option<String>,
    pub prix_verdict: Option<f64>,
    pub cree_le: String,
    pub maj_le: Option<String>,
    pub llm_valide: Option<i64>,
    pub llm_conviction: Option<i64>,
    pub llm_raison: Option<String>,
    pub trailing_coeff: Option<f64>,
}

pub struct NouveauRocket {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub atr14: Option<f64>,
    pub rsi: f64,
    pub llm_valide: Option<bool>,
    pub llm_conviction: Option<i64>,
    pub llm_raison: Option<String>,
    pub llm_sl_suggere: Option<f64>,
    pub llm_tp1_suggere: Option<f64>,
    pub trailing_coeff: f64,
    pub pct_tp1: f64,
    pub pct_tp2: f64,
    pub pct_trailing: f64,
    pub entree_limite: Option<f64>,
    pub entree_stop: Option<f64>,
    pub niveau_invalidation: Option<f64>,
    pub type_entree_rec: Option<String>,
}

pub(crate) fn row_to_signal(row: &sqlx::sqlite::SqliteRow) -> RocketSignal {
    RocketSignal {
        id: row.get("id"),
        ticker: row.get("ticker"),
        phase: row.get("phase"),
        score: row.get("score"),
        prix_entree: row.get("prix_entree"),
        stop_loss: row.get("stop_loss"),
        target: row.get("target"),
        target2: row.get("target2"),
        target3: row.get("target3"),
        ratio_volume: row.get("ratio_volume"),
        atr_ratio: row.get("atr_ratio"),
        atr14: row.get("atr14"),
        rsi: row.get("rsi"),
        statut: row.get("statut"),
        prix_peak: row.get("prix_peak"),
        verdict: row.get("verdict"),
        prix_verdict: row.get("prix_verdict"),
        cree_le: row.get("cree_le"),
        maj_le: row.get("maj_le"),
        llm_valide: row.try_get("llm_valide").unwrap_or(None),
        llm_conviction: row.try_get("llm_conviction").unwrap_or(None),
        llm_raison: row.try_get("llm_raison").unwrap_or(None),
        trailing_coeff: row.try_get("trailing_coeff").unwrap_or(None),
    }
}

/// Insère uniquement si aucun signal identique (ticker+phase) dans les 6 dernières heures.
pub async fn sauvegarder(pool: &SqlitePool, s: &NouveauRocket) -> Result<Option<i64>> {
    let id = sqlx::query(
        "INSERT INTO rockets_signaux
         (ticker, phase, score, prix_entree, stop_loss, target, target2, target3, ratio_volume, atr_ratio, atr14, rsi,
          llm_valide, llm_conviction, llm_raison, llm_sl_suggere, llm_tp1_suggere,
          trailing_coeff, pct_tp1, pct_tp2, pct_trailing,
          entree_limite, entree_stop, niveau_invalidation, type_entree_rec)
         SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
         WHERE NOT EXISTS (
           SELECT 1 FROM rockets_signaux
           WHERE ticker = ? AND phase = ? AND cree_le >= datetime('now', '-6 hours')
         )",
    )
    .bind(&s.ticker)
    .bind(&s.phase)
    .bind(s.score)
    .bind(s.prix_entree)
    .bind(s.stop_loss)
    .bind(s.target)
    .bind(s.target2)
    .bind(s.target3)
    .bind(s.ratio_volume)
    .bind(s.atr_ratio)
    .bind(s.atr14)
    .bind(s.rsi)
    .bind(s.llm_valide.map(|v| v as i64))
    .bind(s.llm_conviction)
    .bind(&s.llm_raison)
    .bind(s.llm_sl_suggere)
    .bind(s.llm_tp1_suggere)
    .bind(s.trailing_coeff)
    .bind(s.pct_tp1)
    .bind(s.pct_tp2)
    .bind(s.pct_trailing)
    .bind(s.entree_limite)
    .bind(s.entree_stop)
    .bind(s.niveau_invalidation)
    .bind(&s.type_entree_rec)
    .bind(&s.ticker)
    .bind(&s.phase)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?
    .last_insert_rowid();

    Ok(if id > 0 { Some(id) } else { None })
}

pub async fn entrer_position(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE rockets_signaux SET statut = 'ouvert', maj_le = datetime('now') WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn maj_prix_peak(pool: &SqlitePool, id: i64, peak: f64) -> Result<()> {
    sqlx::query("UPDATE rockets_signaux SET prix_peak = ?, maj_le = datetime('now') WHERE id = ?")
        .bind(peak)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn maj_verdict(pool: &SqlitePool, id: i64, verdict: &str, prix: f64) -> Result<()> {
    sqlx::query(
        "UPDATE rockets_signaux
         SET verdict = ?, prix_verdict = ?, statut = 'ferme', maj_le = datetime('now')
         WHERE id = ?",
    )
    .bind(verdict)
    .bind(prix)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn marquer_expires(pool: &SqlitePool) -> Result<u64> {
    let res = sqlx::query(
        "UPDATE rockets_signaux SET verdict = 'expire', statut = 'ferme', maj_le = datetime('now')
         WHERE statut = 'attente' AND cree_le <= datetime('now', '-6 hours')",
    )
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(res.rows_affected())
}

pub async fn historique(pool: &SqlitePool, limite: i64) -> Result<Vec<RocketSignal>> {
    let rows = sqlx::query(
        "SELECT id, ticker, phase, score, prix_entree, stop_loss, target, target2, target3,
                ratio_volume, atr_ratio, atr14, rsi, statut, prix_peak, verdict, prix_verdict, cree_le, maj_le,
                llm_valide, llm_conviction, llm_raison
         FROM rockets_signaux ORDER BY cree_le DESC LIMIT ?",
    )
    .bind(limite)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows.iter().map(row_to_signal).collect())
}

// ── Analyses LLM stratégiques ─────────────────────────────────────────────────

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
    Ok(rows.iter().map(row_to_signal).collect())
}
