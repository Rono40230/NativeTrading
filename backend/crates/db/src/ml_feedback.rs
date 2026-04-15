//! Requêtes DB pour le ML Feedback — statistiques de performance par stratégie
//! et historique des suggestions de paramètres appliquées.
//! Ce module n'écrit jamais dans les tables de feedback existantes.
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Structures retournées ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeedbackGlobal {
    pub nb_trades: i64,
    pub nb_gagnants: i64,
    pub win_rate: f64, // 0.0-100.0
    pub pnl_r_moyen: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmcScoreStats {
    pub tranche: String, // "50-65" | "65-75" | "75-85" | "85+"
    pub nb_trades: i64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmcSessionStats {
    pub en_kill_zone: bool,
    pub nb_trades: i64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlCorrelationStats {
    pub tranche: String, // "0.5-0.6" | "0.6-0.7" | "0.7-0.8" | "0.8+"
    pub nb_trades: i64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionLogEntry {
    pub id: i64,
    pub strategie: String,
    pub param_name: String,
    pub valeur_avant: f64,
    pub valeur_apres: f64,
    pub gain_winrate_estime: f64,
    pub confiance: f64,
    pub nb_samples_base: i64,
    pub appliquee_le: String,
}

pub struct NouvelleSuggestionLog<'a> {
    pub strategie: &'a str,
    pub param_name: &'a str,
    pub valeur_avant: f64,
    pub valeur_apres: f64,
    pub gain_winrate_estime: f64,
    pub confiance: f64,
    pub nb_samples_base: i64,
}

// ── Stats globales ────────────────────────────────────────────────────────────

pub async fn stats_globales_smc(pool: &SqlitePool) -> Result<FeedbackGlobal> {
    let r = sqlx::query(
        "SELECT COUNT(*) as nb_trades,
                COALESCE(SUM(CASE WHEN gagnant = 1 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate,
                COALESCE(AVG(pnl_r), 0.0) as pnl_r_moyen
         FROM smc_feedback WHERE verdict IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(FeedbackGlobal {
        nb_trades: r.get("nb_trades"),
        nb_gagnants: r.get("nb_gagnants"),
        win_rate: r.get("win_rate"),
        pnl_r_moyen: r.get("pnl_r_moyen"),
    })
}

pub async fn stats_globales_rockets(pool: &SqlitePool) -> Result<FeedbackGlobal> {
    let r = sqlx::query(
        "SELECT COUNT(*) as nb_trades,
                COALESCE(SUM(CASE WHEN gagnant = 1 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate,
                COALESCE(AVG(pnl_r), 0.0) as pnl_r_moyen
         FROM rockets_feedback WHERE verdict IS NOT NULL AND verdict NOT IN ('invalide','expire')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(FeedbackGlobal {
        nb_trades: r.get("nb_trades"),
        nb_gagnants: r.get("nb_gagnants"),
        win_rate: r.get("win_rate"),
        pnl_r_moyen: r.get("pnl_r_moyen"),
    })
}

pub async fn stats_globales_straddle(pool: &SqlitePool) -> Result<FeedbackGlobal> {
    let r = sqlx::query(
        "SELECT COUNT(*) as nb_trades,
                COALESCE(SUM(CASE WHEN gagnant = 1 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate,
                COALESCE(AVG(pnl_r), 0.0) as pnl_r_moyen
         FROM straddle_feedback WHERE verdict IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(FeedbackGlobal {
        nb_trades: r.get("nb_trades"),
        nb_gagnants: r.get("nb_gagnants"),
        win_rate: r.get("win_rate"),
        pnl_r_moyen: r.get("pnl_r_moyen"),
    })
}

// ── Stats SMC détaillées ──────────────────────────────────────────────────────

pub async fn stats_smc_par_score(pool: &SqlitePool) -> Result<Vec<SmcScoreStats>> {
    let rows = sqlx::query(
        "SELECT CASE
                    WHEN score_smc < 65 THEN '50-65'
                    WHEN score_smc < 75 THEN '65-75'
                    WHEN score_smc < 85 THEN '75-85'
                    ELSE '85+'
                END as tranche,
                COUNT(*) as nb_trades,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate
         FROM smc_feedback
         WHERE verdict IS NOT NULL AND score_smc >= 50
         GROUP BY tranche
         ORDER BY MIN(score_smc)",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| SmcScoreStats {
            tranche: r.get("tranche"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
        })
        .collect())
}

pub async fn stats_smc_par_kill_zone(pool: &SqlitePool) -> Result<Vec<SmcSessionStats>> {
    let rows = sqlx::query(
        "SELECT kill_zone_active,
                COUNT(*) as nb_trades,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate
         FROM smc_feedback
         WHERE verdict IS NOT NULL
         GROUP BY kill_zone_active",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| SmcSessionStats {
            en_kill_zone: r.get::<i64, _>("kill_zone_active") != 0,
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
        })
        .collect())
}

pub async fn stats_smc_ml_correlation(pool: &SqlitePool) -> Result<Vec<MlCorrelationStats>> {
    let rows = sqlx::query(
        "SELECT CASE
                    WHEN confiance_ml < 0.6 THEN '0.5-0.6'
                    WHEN confiance_ml < 0.7 THEN '0.6-0.7'
                    WHEN confiance_ml < 0.8 THEN '0.7-0.8'
                    ELSE '0.8+'
                END as tranche,
                COUNT(*) as nb_trades,
                COALESCE(AVG(CASE WHEN gagnant = 1 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate
         FROM smc_feedback
         WHERE verdict IS NOT NULL AND confiance_ml >= 0.5
         GROUP BY tranche
         ORDER BY MIN(confiance_ml)",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| MlCorrelationStats {
            tranche: r.get("tranche"),
            nb_trades: r.get("nb_trades"),
            win_rate: r.get("win_rate"),
        })
        .collect())
}

// ── Historique suggestions ────────────────────────────────────────────────────

pub async fn sauvegarder_suggestion(
    pool: &SqlitePool,
    s: &NouvelleSuggestionLog<'_>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ml_suggestions_log
         (strategie, param_name, valeur_avant, valeur_apres,
          gain_winrate_estime, confiance, nb_samples_base)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(s.strategie)
    .bind(s.param_name)
    .bind(s.valeur_avant)
    .bind(s.valeur_apres)
    .bind(s.gain_winrate_estime)
    .bind(s.confiance)
    .bind(s.nb_samples_base)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn lister_suggestions(pool: &SqlitePool, limite: i64) -> Result<Vec<SuggestionLogEntry>> {
    let rows = sqlx::query(
        "SELECT id, strategie, param_name, valeur_avant, valeur_apres,
                gain_winrate_estime, confiance, nb_samples_base, appliquee_le
         FROM ml_suggestions_log
         ORDER BY appliquee_le DESC
         LIMIT ?",
    )
    .bind(limite)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| SuggestionLogEntry {
            id: r.get("id"),
            strategie: r.get("strategie"),
            param_name: r.get("param_name"),
            valeur_avant: r.get("valeur_avant"),
            valeur_apres: r.get("valeur_apres"),
            gain_winrate_estime: r.get("gain_winrate_estime"),
            confiance: r.get("confiance"),
            nb_samples_base: r.get("nb_samples_base"),
            appliquee_le: r.get("appliquee_le"),
        })
        .collect())
}
