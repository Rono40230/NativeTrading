//! Couche DB pour la calibration des seuils Rockets.
//!
//! Stocke et recharge les seuils optimaux (score_min, conviction_min)
//! calculés par grid search sur les feedbacks réels toutes les 6 heures.
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketsCalibrationRow {
    pub phase: String,
    pub session: String,
    pub score_min: i64,
    pub conviction_min: i64,
    pub nb_trades: i64,
    pub win_rate: f64,
    pub pnl_moyen_r: f64,
    pub fiabilite: String,
    pub invalide: bool,
    pub maj_le: i64,
}

/// Seuils effectifs à utiliser dans le filtre LLM (avec valeurs par défaut).
#[derive(Debug, Clone)]
pub struct SeuilsRockets {
    pub score_min: i64,
    pub conviction_min: i64,
    pub invalide: bool,
}

impl Default for SeuilsRockets {
    fn default() -> Self {
        Self {
            score_min: 65,
            conviction_min: 50,
            invalide: false,
        }
    }
}

// ── Écriture ─────────────────────────────────────────────────────────────────

/// Upsert d'un résultat de calibration pour un (phase, session).
pub async fn sauvegarder(pool: &SqlitePool, row: &RocketsCalibrationRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO rockets_calibration
         (phase, session, score_min, conviction_min, nb_trades,
          win_rate, pnl_moyen_r, fiabilite, invalide, maj_le)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())
         ON CONFLICT(phase, session) DO UPDATE SET
             score_min      = excluded.score_min,
             conviction_min = excluded.conviction_min,
             nb_trades      = excluded.nb_trades,
             win_rate       = excluded.win_rate,
             pnl_moyen_r    = excluded.pnl_moyen_r,
             fiabilite      = excluded.fiabilite,
             invalide       = excluded.invalide,
             maj_le         = excluded.maj_le",
    )
    .bind(&row.phase)
    .bind(&row.session)
    .bind(row.score_min)
    .bind(row.conviction_min)
    .bind(row.nb_trades)
    .bind(row.win_rate)
    .bind(row.pnl_moyen_r)
    .bind(&row.fiabilite)
    .bind(row.invalide as i64)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

// ── Lecture ──────────────────────────────────────────────────────────────────

/// Charge les seuils effectifs pour un (phase, session).
/// Retourne les valeurs par défaut si aucune calibration ou fiabilité insuffisante.
pub async fn charger_seuils(pool: &SqlitePool, phase: &str, session: &str) -> SeuilsRockets {
    let row = sqlx::query(
        "SELECT score_min, conviction_min, fiabilite, invalide
         FROM rockets_calibration
         WHERE phase = ? AND session = ?",
    )
    .bind(phase)
    .bind(session)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        None => SeuilsRockets::default(),
        Some(r) => {
            let fiabilite: String = r.get("fiabilite");
            if fiabilite == "insuffisant" {
                return SeuilsRockets::default();
            }
            let invalide: i64 = r.get("invalide");
            let score_brut: i64 = r.get("score_min");
            let conviction_brut: i64 = r.get("conviction_min");
            // Marge de sécurité +3 pts sur conviction si fiabilité "faible"
            let conviction_final = if fiabilite == "faible" {
                (conviction_brut + 3).min(85)
            } else {
                conviction_brut
            };
            SeuilsRockets {
                score_min: score_brut,
                conviction_min: conviction_final,
                invalide: invalide == 1,
            }
        }
    }
}

/// Retourne toutes les calibrations — pour le endpoint `/api/rockets/calibration`.
pub async fn lister_toutes(pool: &SqlitePool) -> Result<Vec<RocketsCalibrationRow>> {
    let rows = sqlx::query(
        "SELECT phase, session, score_min, conviction_min, nb_trades,
                win_rate, pnl_moyen_r, fiabilite, invalide, maj_le
         FROM rockets_calibration
         ORDER BY phase, session",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| RocketsCalibrationRow {
            phase: r.get("phase"),
            session: r.get("session"),
            score_min: r.get("score_min"),
            conviction_min: r.get("conviction_min"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
            pnl_moyen_r: r.get("pnl_moyen_r"),
            fiabilite: r.get("fiabilite"),
            invalide: r.get::<i64, _>("invalide") == 1,
            maj_le: r.get("maj_le"),
        })
        .collect())
}
