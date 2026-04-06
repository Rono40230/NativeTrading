//! Couche DB pour la calibration des seuils SMC Directionnel.
//!
//! Stocke les seuils optimaux (score_smc_seuil, conviction_seuil)
//! calculés par grid search sur les feedbacks réels toutes les 6 heures.
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmcCalibrationRow {
    pub asset: String,
    pub timeframe: String,
    pub categorie: String,
    pub score_smc_seuil: f64,
    pub conviction_seuil: i64,
    pub nb_trades: i64,
    pub win_rate: f64,
    pub pnl_moyen_r: f64,
    pub fiabilite: String,
    pub invalide: bool,
    pub maj_le: i64,
}

/// Seuils effectifs utilisés par la boucle SMC (avec valeurs par défaut).
#[derive(Debug, Clone)]
pub struct SeuilsSmc {
    pub score_smc_seuil: f64,
    pub conviction_seuil: i64,
    pub invalide: bool,
}

impl Default for SeuilsSmc {
    fn default() -> Self {
        Self {
            score_smc_seuil: 70.0,
            conviction_seuil: 70,
            invalide: false,
        }
    }
}

// ── Écriture ─────────────────────────────────────────────────────────────────

pub async fn sauvegarder(pool: &SqlitePool, row: &SmcCalibrationRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO smc_calibration
         (asset, timeframe, categorie, score_smc_seuil, conviction_seuil,
          nb_trades, win_rate, pnl_moyen_r, fiabilite, invalide, maj_le)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())
         ON CONFLICT(asset, timeframe, categorie) DO UPDATE SET
             score_smc_seuil  = excluded.score_smc_seuil,
             conviction_seuil = excluded.conviction_seuil,
             nb_trades        = excluded.nb_trades,
             win_rate         = excluded.win_rate,
             pnl_moyen_r      = excluded.pnl_moyen_r,
             fiabilite        = excluded.fiabilite,
             invalide         = excluded.invalide,
             maj_le           = unixepoch()",
    )
    .bind(&row.asset)
    .bind(&row.timeframe)
    .bind(&row.categorie)
    .bind(row.score_smc_seuil)
    .bind(row.conviction_seuil)
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

/// Charge les seuils calibrés pour un triplet (asset, timeframe, categorie).
/// Retourne les valeurs par défaut si aucune entrée n'existe.
pub async fn charger_seuils(
    pool: &SqlitePool,
    asset: &str,
    timeframe: &str,
    categorie: &str,
) -> SeuilsSmc {
    let row = sqlx::query(
        "SELECT score_smc_seuil, conviction_seuil, fiabilite, invalide
         FROM smc_calibration
         WHERE asset = ? AND timeframe = ? AND categorie = ?",
    )
    .bind(asset)
    .bind(timeframe)
    .bind(categorie)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        None => SeuilsSmc::default(),
        Some(r) => {
            let fiabilite: String = r.get("fiabilite");
            let conviction_brut: i64 = r.get("conviction_seuil");
            // Marge de sécurité +3 pts si fiabilité faible
            let conviction_final = if fiabilite == "faible" {
                conviction_brut + 3
            } else {
                conviction_brut
            };
            SeuilsSmc {
                score_smc_seuil: r.get("score_smc_seuil"),
                conviction_seuil: conviction_final,
                invalide: r.get::<i64, _>("invalide") == 1,
            }
        }
    }
}

/// Toutes les calibrations — pour le endpoint monitoring.
pub async fn lister_toutes(pool: &SqlitePool) -> Result<Vec<SmcCalibrationRow>> {
    let rows = sqlx::query(
        "SELECT asset, timeframe, categorie, score_smc_seuil, conviction_seuil,
                nb_trades, win_rate, pnl_moyen_r, fiabilite, invalide, maj_le
         FROM smc_calibration
         ORDER BY asset, timeframe, categorie",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| SmcCalibrationRow {
            asset: r.get("asset"),
            timeframe: r.get("timeframe"),
            categorie: r.get("categorie"),
            score_smc_seuil: r.get("score_smc_seuil"),
            conviction_seuil: r.get("conviction_seuil"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
            pnl_moyen_r: r.get("pnl_moyen_r"),
            fiabilite: r.get("fiabilite"),
            invalide: r.get::<i64, _>("invalide") == 1,
            maj_le: r.get("maj_le"),
        })
        .collect())
}
