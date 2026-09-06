//! Phase 8.1 — Sauvegarde et lecture des samples ML issus des trades clôturés.

use crate::TradingError;
use common::Result;
use sqlx::SqlitePool;

/// Données à persister pour un trade clôturé.
pub struct MlSample {
    pub strategie: String, // "SMC" | "ROCKETS" | "STRADDLE"
    pub asset: String,
    pub timeframe: String,
    pub direction: String, // "Long" | "Short" | "LONG" | "STRADDLE"
    pub prix_entree: f64,
    pub prix_sortie: f64,
    pub stop_loss: f64,
    pub outcome: String, // "tp1"|"tp2"|"tp3"|"sl"|"invalide"|"expire"
    pub rr_realise: Option<f64>,
    /// Signal source — index unique : INSERT OR IGNORE, jamais de doublon
    /// (rattrapage multi-boots + collecteur continu coexistent).
    pub signal_id: Option<String>,
}

/// Persiste un sample de trade clôturé.
pub async fn sauvegarder_sample(pool: &SqlitePool, s: &MlSample) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO ml_training_samples
            (strategie, asset, timeframe, direction, prix_entree, prix_sortie, stop_loss, outcome, rr_realise, signal_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(&s.signal_id)
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

/// §11 étape 1 (06/09) — boucle ML v2 : chaque clôture officielle (SMC,
/// straddle, rockets — point de passage commun `fermer_signal_par_cle`)
/// alimente ml_training_samples. Silencieux et idempotent : la matière
/// première statistique s'accumule, jamais le ML ne décide.
pub async fn collecter_a_la_cloture(
    pool: &SqlitePool,
    cle_moteur: &str,
    asset: &str,
    verdict: &str,
    prix_verdict: f64,
    r_realise: f64,
) {
    use sqlx::Row as _;
    if let Ok(Some(sig)) = sqlx::query(
        "SELECT id, strategie, timeframe, direction, prix_entree, stop_loss
         FROM signaux WHERE cle_moteur = ? AND asset = ? AND statut = 'Fermé'
         ORDER BY ferme_le DESC LIMIT 1",
    )
    .bind(cle_moteur)
    .bind(asset)
    .fetch_optional(pool)
    .await
    {
        let sample = MlSample {
            strategie: sig.get::<String, _>("strategie"),
            asset: asset.to_string(),
            timeframe: sig.get::<String, _>("timeframe"),
            direction: sig.get::<String, _>("direction"),
            prix_entree: sig.get::<f64, _>("prix_entree"),
            prix_sortie: prix_verdict,
            stop_loss: sig.get::<f64, _>("stop_loss"),
            outcome: verdict.to_string(),
            rr_realise: Some(r_realise),
            signal_id: Some(sig.get::<String, _>("id")),
        };
        let _ = sauvegarder_sample(pool, &sample).await;
    }
}
