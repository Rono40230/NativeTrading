//! Reclassification rétroactive des verdicts Rockets.
//! Séparé de rockets_feedback.rs pour respecter la limite de 300 lignes.
use common::{Result, TradingError};
use sqlx::SqlitePool;

/// Reclassifie les anciens trades clôturés avec verdict='sl' qui auraient dû
/// être 'be', 'tp1' ou 'tp2' (SL progressif non reconnu à l'époque).
/// S'appuie sur prix_peak stocké en continu pendant le suivi du trade.
/// Idempotente : ne touche que les lignes encore à 'sl'.
pub async fn reclassifier_verdicts_sl(pool: &SqlitePool) -> Result<()> {
    // Priorité décroissante : tp2 d'abord pour éviter de passer par tp1 si peak >= target3
    let r2 = sqlx::query(
        "UPDATE rockets_signaux SET verdict='tp2'
         WHERE statut='ferme' AND verdict='sl'
           AND prix_peak IS NOT NULL AND target2 IS NOT NULL AND target3 IS NOT NULL
           AND prix_peak >= target3",
    )
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let r1 = sqlx::query(
        "UPDATE rockets_signaux SET verdict='tp1'
         WHERE statut='ferme' AND verdict='sl'
           AND prix_peak IS NOT NULL AND target2 IS NOT NULL
           AND prix_peak >= target2",
    )
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let r0 = sqlx::query(
        "UPDATE rockets_signaux SET verdict='be'
         WHERE statut='ferme' AND verdict='sl'
           AND prix_peak IS NOT NULL
           AND prix_peak >= target",
    )
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let total = r2.rows_affected() + r1.rows_affected() + r0.rows_affected();
    if total > 0 {
        tracing::info!(
            "reclassifier_verdicts_sl: {} reclassifiés (be={}, tp1={}, tp2={})",
            total,
            r0.rows_affected(),
            r1.rows_affected(),
            r2.rows_affected()
        );
        // Synchroniser rockets_feedback pour les lignes reclassifiées
        sqlx::query(
            "UPDATE rockets_feedback
             SET verdict = (SELECT rs.verdict FROM rockets_signaux rs WHERE rs.id = signal_id),
                 pnl_r  = CASE (SELECT rs.verdict FROM rockets_signaux rs WHERE rs.id = signal_id)
                            WHEN 'be' THEN 0.0
                            ELSE (SELECT CASE WHEN rs.atr14 > 0
                                         THEN (rs.prix_verdict - rs.prix_entree) / rs.atr14
                                         ELSE 0.0 END
                                  FROM rockets_signaux rs WHERE rs.id = signal_id)
                          END,
                 gagnant = CASE (SELECT rs.verdict FROM rockets_signaux rs WHERE rs.id = signal_id)
                            WHEN 'be' THEN 0 ELSE 1
                          END
             WHERE signal_id IN (
                   SELECT id FROM rockets_signaux
                   WHERE statut='ferme' AND verdict IN ('be','tp1','tp2')
               )
               AND verdict != (SELECT rs.verdict FROM rockets_signaux rs WHERE rs.id = signal_id)",
        )
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    }

    Ok(())
}
