//! Phase 8.1 — Sauvegarde et lecture des samples ML issus des trades clôturés.

use crate::TradingError;
use common::Result;
use sqlx::SqlitePool;

/// Données à persister pour un trade clôturé.
pub struct MlSample {
    pub strategie:   String, // "SMC" | "ROCKETS" | "STRADDLE"
    pub asset:       String,
    pub timeframe:   String,
    pub direction:   String, // "Long" | "Short" | "LONG" | "STRADDLE"
    pub prix_entree: f64,
    pub prix_sortie: f64,
    pub stop_loss:   f64,
    pub outcome:     String, // "tp1"|"tp2"|"tp3"|"sl"|"invalide"|"expire"
    pub rr_realise:  Option<f64>,
}

/// Persiste un sample de trade clôturé.
pub async fn sauvegarder_sample(pool: &SqlitePool, s: &MlSample) -> Result<()> {
    sqlx::query(
        "INSERT INTO ml_training_samples
            (strategie, asset, timeframe, direction, prix_entree, prix_sortie, stop_loss, outcome, rr_realise)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&s.strategie)
    .bind(&s.asset)
    .bind(&s.timeframe)
    .bind(&s.direction)
    .bind(s.prix_entree)
    .bind(s.prix_sortie)
    .bind(s.stop_loss)
    .bind(&s.outcome)
    .bind(s.rr_realise)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

/// Compte les samples créés dans les dernières `depuis_heures` heures.
/// Utilisé pour déclencher le réentraînement automatique si ≥ 100 nouveaux trades.
pub async fn compter_nouveaux_samples(pool: &SqlitePool, depuis_heures: i64) -> Result<i64> {
    use sqlx::Row;
    let modifier = format!("{} hours", depuis_heures);
    let row = sqlx::query(
        "SELECT COUNT(*) AS n FROM ml_training_samples
         WHERE cree_le >= datetime('now', ?)",
    )
    .bind(&modifier)
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(row.get("n"))
}
