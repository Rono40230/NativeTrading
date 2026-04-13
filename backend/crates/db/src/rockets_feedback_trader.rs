//! Saisie manuelle du verdict trader pour la stratégie Rockets.
//! Fonctions séparées de rockets_feedback.rs pour respecter la limite de taille.

use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

// ── Saisie trader ─────────────────────────────────────────────────────────────

/// Enregistre le résultat d'un trade saisi manuellement par le trader.
/// Si `verdict == "ignore"` : marque sans pnl_r (signal ignoré).
/// Sinon : calcule pnl_r à partir de l'atr14 du signal et des prix réels.
pub async fn saisir_verdict_trader(
    pool: &SqlitePool,
    signal_id: i64,
    verdict: &str,
    prix_entree_reel: f64,
    prix_sortie_reel: Option<f64>,
    notes: Option<&str>,
) -> Result<()> {
    if verdict == "ignore" {
        sqlx::query(
            "UPDATE rockets_feedback
             SET verdict = 'ignore', ferme_le = strftime('%s','now')
             WHERE signal_id = ?",
        )
        .bind(signal_id)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        return Ok(());
    }

    let sortie = prix_sortie_reel.unwrap_or(0.0);

    // Récupérer atr14 + timestamp_signal depuis les tables liées
    let row = sqlx::query(
        "SELECT rs.atr14, rf.timestamp_signal
         FROM rockets_signaux rs
         JOIN rockets_feedback rf ON rf.signal_id = rs.id
         WHERE rs.id = ?",
    )
    .bind(signal_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let (atr14, timestamp_signal) = match row {
        Some(r) => {
            let atr: f64 = r.try_get::<Option<f64>, _>("atr14").ok().flatten().unwrap_or(1.0);
            let ts: i64 = r.get("timestamp_signal");
            (atr, ts)
        }
        None => (1.0, 0),
    };

    let now = chrono::Utc::now().timestamp();
    let duree_min = if timestamp_signal > 0 { (now - timestamp_signal) / 60 } else { 0 };
    let pnl_r = if atr14 > 0.0 { (sortie - prix_entree_reel) / atr14 } else { 0.0 };
    let gagnant: i64 = if verdict.starts_with("tp") { 1 } else { 0 };

    sqlx::query(
        "UPDATE rockets_feedback
         SET verdict = ?, pnl_r = ?, gagnant = ?,
             duree_trade_min = ?, ferme_le = ?
         WHERE signal_id = ?",
    )
    .bind(verdict)
    .bind(pnl_r)
    .bind(gagnant)
    .bind(duree_min)
    .bind(now)
    .bind(signal_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    // Stocker les notes si fournies (champ optionnel ignoré silencieusement si absent)
    if let Some(n) = notes {
        let _ = sqlx::query(
            "UPDATE rockets_feedback SET notes_trader = ? WHERE signal_id = ?",
        )
        .bind(n)
        .bind(signal_id)
        .execute(pool)
        .await;
    }

    Ok(())
}
