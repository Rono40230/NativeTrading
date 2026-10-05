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

// ── Stats globales ────────────────────────────────────────────────────────────

pub async fn stats_globales_smc(pool: &SqlitePool) -> Result<FeedbackGlobal> {
    let r = sqlx::query(
        "SELECT COUNT(*) as nb_trades,
                COALESCE(SUM(CASE WHEN rr_realise > 0 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(CASE WHEN rr_realise > 0 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate,
                COALESCE(AVG(rr_realise), 0.0) as pnl_r_moyen
         FROM ml_training_samples
         WHERE LOWER(strategie) LIKE '%smc%' AND rr_realise IS NOT NULL
           AND LOWER(outcome) NOT IN ('expire','invalide')",
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
                COALESCE(SUM(CASE WHEN rr_realise > 0 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(CASE WHEN rr_realise > 0 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate,
                COALESCE(AVG(rr_realise), 0.0) as pnl_r_moyen
         FROM ml_training_samples
         WHERE LOWER(strategie) LIKE '%rocket%' AND rr_realise IS NOT NULL
           AND LOWER(outcome) NOT IN ('expire','invalide')",
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
                COALESCE(SUM(CASE WHEN rr_realise > 0 THEN 1 ELSE 0 END), 0) as nb_gagnants,
                COALESCE(AVG(CASE WHEN rr_realise > 0 THEN 100.0 ELSE 0.0 END), 0.0) as win_rate,
                COALESCE(AVG(rr_realise), 0.0) as pnl_r_moyen
         FROM ml_training_samples
         WHERE LOWER(strategie) LIKE '%straddle%' AND rr_realise IS NOT NULL
           AND LOWER(outcome) NOT IN ('expire','invalide')",
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn db_test() -> crate::Database {
        let db = crate::Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations");
        db
    }

    async fn echantillon(db: &crate::Database, strategie: &str, rr: Option<f64>, outcome: &str) {
        sqlx::query(
            "INSERT INTO ml_training_samples (strategie, asset, timeframe, direction, prix_entree,
             prix_sortie, stop_loss, outcome, rr_realise) VALUES (?, 'BTC', 'M1', 'Long', 100.0, 101.0, 99.0, ?, ?)",
        )
        .bind(strategie)
        .bind(outcome)
        .bind(rr)
        .execute(db.pool())
        .await
        .expect("échantillon");
    }

    /// Régression 05/10 (étape 11) : les globales rockets/straddle référençaient
    /// des colonnes inexistantes (gagnant, pnl_r) — SQL en erreur silencieuse,
    /// réponse null à l'UI. Alignées sur rr_realise, le patron SMC.
    #[tokio::test]
    async fn globales_trois_strategies_sur_echantillons_reels() {
        let db = db_test().await;
        // rockets : 2 trades (+1R, −1R), 1 expiré exclu → 1 trade, 100 % WR.
        echantillon(&db, "ROCKETS", Some(1.0), "tp2").await;
        echantillon(&db, "ROCKETS", Some(-1.0), "sl").await;
        echantillon(&db, "ROCKETS", None, "expire").await;
        // straddle : idem.
        echantillon(&db, "STRADDLE", Some(2.0), "tp2").await;
        echantillon(&db, "STRADDLE", Some(-1.0), "sl").await;
        echantillon(&db, "STRADDLE", None, "invalide").await;

        let r = stats_globales_rockets(db.pool()).await.expect("rockets");
        assert_eq!(r.nb_trades, 2, "l'expiré est exclu");
        assert_eq!(r.nb_gagnants, 1);
        assert!((r.win_rate - 50.0).abs() < 1e-9);

        let st = stats_globales_straddle(db.pool()).await.expect("straddle");
        assert_eq!(st.nb_trades, 2);
        assert!((st.pnl_r_moyen - 0.5).abs() < 1e-9, "moyenne (+2 −1)/2");
    }
}
