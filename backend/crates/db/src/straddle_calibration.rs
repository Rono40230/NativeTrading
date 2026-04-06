//! Couche DB pour la calibration des seuils Straddle.
//!
//! Stocke et recharge les seuils calculés par (asset, categorie).
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRow {
    pub asset: String,
    pub categorie: String,
    pub score_llm_seuil: f64,
    pub atr_seuil: f64,
    pub nb_trades: i64,
    pub win_rate: f64,
    pub pnl_moyen_r: f64,
    pub fiabilite: String,
    pub invalide: bool,
    pub maj_le: i64,
}

/// Seuils effectifs à utiliser dans la boucle (avec valeurs par défaut si insuffisant).
#[derive(Debug, Clone)]
pub struct SeuilsEffectifs {
    pub score_llm: f64,
    pub ratio_atr: f64,
    pub invalide: bool,
}

impl Default for SeuilsEffectifs {
    fn default() -> Self {
        Self {
            score_llm: 6.0,
            ratio_atr: 1.5,
            invalide: false,
        }
    }
}

// ── Écriture ─────────────────────────────────────────────────────────────────

/// Upsert d'un résultat de calibration pour un (asset, categorie).
pub async fn sauvegarder(pool: &SqlitePool, row: &CalibrationRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO straddle_calibration
         (asset, categorie, score_llm_seuil, atr_seuil, nb_trades,
          win_rate, pnl_moyen_r, fiabilite, invalide, maj_le)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())
         ON CONFLICT(asset, categorie) DO UPDATE SET
             score_llm_seuil = excluded.score_llm_seuil,
             atr_seuil       = excluded.atr_seuil,
             nb_trades       = excluded.nb_trades,
             win_rate        = excluded.win_rate,
             pnl_moyen_r     = excluded.pnl_moyen_r,
             fiabilite       = excluded.fiabilite,
             invalide        = excluded.invalide,
             maj_le          = excluded.maj_le",
    )
    .bind(&row.asset)
    .bind(&row.categorie)
    .bind(row.score_llm_seuil)
    .bind(row.atr_seuil)
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

/// Charge les seuils effectifs pour un (asset, categorie).
/// Retourne les valeurs par défaut si aucune calibration ou fiabilité insuffisante.
pub async fn charger_seuils(pool: &SqlitePool, asset: &str, categorie: &str) -> SeuilsEffectifs {
    let row = sqlx::query(
        "SELECT score_llm_seuil, atr_seuil, fiabilite, invalide
         FROM straddle_calibration WHERE asset = ? AND categorie = ?",
    )
    .bind(asset)
    .bind(categorie)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        None => SeuilsEffectifs::default(),
        Some(r) => {
            let fiabilite: String = r.get("fiabilite");
            if fiabilite == "insuffisant" {
                return SeuilsEffectifs::default();
            }
            let invalide: i64 = r.get("invalide");
            let score_brut: f64 = r.get("score_llm_seuil");
            let atr_brut: f64 = r.get("atr_seuil");
            // Marge de sécurité +0.5 sur score LLM si fiabilité "faible"
            let score_final = if fiabilite == "faible" {
                (score_brut + 0.5).min(9.5)
            } else {
                score_brut
            };
            SeuilsEffectifs {
                score_llm: score_final,
                ratio_atr: atr_brut,
                invalide: invalide == 1,
            }
        }
    }
}

/// Retourne toutes les calibrations — pour le endpoint `/api/straddle/calibration`.
pub async fn lister_toutes(pool: &SqlitePool) -> Result<Vec<CalibrationRow>> {
    let rows = sqlx::query(
        "SELECT asset, categorie, score_llm_seuil, atr_seuil, nb_trades,
                win_rate, pnl_moyen_r, fiabilite, invalide, maj_le
         FROM straddle_calibration
         ORDER BY asset, categorie",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| CalibrationRow {
            asset: r.get("asset"),
            categorie: r.get("categorie"),
            score_llm_seuil: r.get("score_llm_seuil"),
            atr_seuil: r.get("atr_seuil"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
            pnl_moyen_r: r.get("pnl_moyen_r"),
            fiabilite: r.get("fiabilite"),
            invalide: r.get::<i64, _>("invalide") == 1,
            maj_le: r.get("maj_le"),
        })
        .collect())
}
