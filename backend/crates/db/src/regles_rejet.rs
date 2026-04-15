//! Module DB pour les règles de rejet de patterns d'échec (P10).
//!
//! Chaque règle représente une combinaison de conditions dont le win rate
//! est statistiquement trop faible (<35%, min 10 trades).
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegleRejet {
    pub id: i64,
    pub strategie: String,
    pub condition: String,
    pub cle_unique: String,
    pub win_rate: f64,
    pub nb_trades: i64,
    pub active: bool,
    pub apprise_le: String,
}

pub struct NouvelleRegle<'a> {
    pub strategie: &'a str,
    pub condition: &'a str,
    pub cle_unique: &'a str,
    pub win_rate: f64,
    pub nb_trades: i64,
}

// ── Écriture ─────────────────────────────────────────────────────────────────

/// Insère ou met à jour une règle (UPSERT sur cle_unique).
pub async fn upsert_regle(pool: &SqlitePool, r: &NouvelleRegle<'_>) -> Result<()> {
    sqlx::query(
        "INSERT INTO regles_rejet_apprises
             (strategie, condition, cle_unique, win_rate, nb_trades, active, apprise_le)
         VALUES (?, ?, ?, ?, ?, 1, datetime('now'))
         ON CONFLICT(cle_unique) DO UPDATE SET
             win_rate   = excluded.win_rate,
             nb_trades  = excluded.nb_trades,
             condition  = excluded.condition,
             apprise_le = datetime('now')",
    )
    .bind(r.strategie)
    .bind(r.condition)
    .bind(r.cle_unique)
    .bind(r.win_rate)
    .bind(r.nb_trades)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

/// Désactive les règles dont le win rate est remonté au-dessus du seuil (> 45%),
/// pour éviter de bloquer des setups qui se sont améliorés.
pub async fn desactiver_obsoletes(pool: &SqlitePool, strategie: &str, cles_actives: &[String]) -> Result<()> {
    if cles_actives.is_empty() {
        // Désactiver toutes les règles de cette stratégie si aucune n'est détectée
        sqlx::query(
            "UPDATE regles_rejet_apprises SET active = 0 WHERE strategie = ?",
        )
        .bind(strategie)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        return Ok(());
    }
    // Désactiver les règles non présentes dans la liste courante
    let placeholders = cles_actives.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "UPDATE regles_rejet_apprises SET active = 0
         WHERE strategie = ? AND cle_unique NOT IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql).bind(strategie);
    for cle in cles_actives {
        q = q.bind(cle);
    }
    q.execute(pool).await.map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

// ── Lecture ──────────────────────────────────────────────────────────────────

/// Retourne les règles actives pour une stratégie — injectées dans les prompts.
pub async fn lister_actives(pool: &SqlitePool, strategie: &str) -> Result<Vec<RegleRejet>> {
    let rows = sqlx::query(
        "SELECT id, strategie, condition, cle_unique, win_rate, nb_trades, active, apprise_le
         FROM regles_rejet_apprises
         WHERE strategie = ? AND active = 1
         ORDER BY win_rate ASC
         LIMIT 10",
    )
    .bind(strategie)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper).collect())
}

/// Toutes les règles (pour l'UI de monitoring future).
pub async fn lister_toutes(pool: &SqlitePool) -> Result<Vec<RegleRejet>> {
    let rows = sqlx::query(
        "SELECT id, strategie, condition, cle_unique, win_rate, nb_trades, active, apprise_le
         FROM regles_rejet_apprises
         ORDER BY strategie, win_rate ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper).collect())
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn mapper(r: &sqlx::sqlite::SqliteRow) -> RegleRejet {
    RegleRejet {
        id:          r.get("id"),
        strategie:   r.get("strategie"),
        condition:   r.get("condition"),
        cle_unique:  r.get("cle_unique"),
        win_rate:    r.get("win_rate"),
        nb_trades:   r.get("nb_trades"),
        active:      r.get::<i64, _>("active") == 1,
        apprise_le:  r.get("apprise_le"),
    }
}
