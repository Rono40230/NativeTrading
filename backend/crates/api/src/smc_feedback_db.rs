//! Types et requêtes DB pour le job de feedback SMC.

use common::{Result, TradingError};
use db::Database;
use std::sync::Arc;

// ── Signal SMC ouvert ─────────────────────────────────────────────────────────

pub struct SignalSmcOuvert {
    pub id: String,
    pub asset: String,
    pub timeframe: String,
    pub direction: String, // "Long" | "Short"
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: Vec<f64>, // [tp1, tp2, tp3]
    pub cree_le: i64,
}

// ── Requêtes ──────────────────────────────────────────────────────────────────

pub async fn charger_signaux_smc_ouverts(db: &Arc<Database>) -> Result<Vec<SignalSmcOuvert>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, asset, timeframe, direction, prix_entree, stop_loss, take_profit, cree_le
         FROM signaux
         WHERE statut = 'Actif' AND strategie IN ('SMC', 'SMC Directionnel')
         ORDER BY cree_le ASC",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let take_profit: Vec<f64> =
                serde_json::from_str(r.get::<&str, _>("take_profit")).unwrap_or_default();
            SignalSmcOuvert {
                id: r.get("id"),
                asset: r.get("asset"),
                timeframe: r.get("timeframe"),
                direction: r.get("direction"),
                prix_entree: r.get("prix_entree"),
                stop_loss: r.get("stop_loss"),
                take_profit,
                cree_le: r.get("cree_le"),
            }
        })
        .collect())
}

pub async fn lire_atr14_feedback(db: &Arc<Database>, signal_id: &str) -> Result<f64> {
    use sqlx::Row;
    let row = sqlx::query("SELECT atr14 FROM smc_feedback WHERE signal_id = ? LIMIT 1")
        .bind(signal_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    match row {
        Some(r) => Ok(r.get::<f64, _>("atr14")),
        None => Err(TradingError::Data("atr14 introuvable".into())),
    }
}
