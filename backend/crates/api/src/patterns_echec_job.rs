//! Job de détection automatique de patterns d'échec récurrents (P10).
//!
//! Tourne toutes les 6h. Analyse les feedbacks des 3 stratégies et crée
//! des règles de rejet pour les combinaisons (conditions) dont le win rate
//! est < 35% sur au moins 10 trades.
//!
//! Les règles résultantes sont injectées dans les prompts Ollama via
//! `charger_lecons_systemiques()`.
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ── Seuils ────────────────────────────────────────────────────────────────────

const WIN_RATE_ECHEC: f64 = 35.0; // % en dessous duquel c'est un pattern d'échec
const NB_TRADES_MIN: i64 = 10;    // nombre minimum de trades pour valider

// ── Point d'entrée ────────────────────────────────────────────────────────────

/// Démarre le job en background (toutes les 6h, premier run après 10 min).
pub fn demarrer_job_patterns_echec(db: Arc<Database>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(600)).await;
        loop {
            analyser_tous(&db).await;
            sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
    tracing::info!("🔍 Job détection patterns d'échec démarré (toutes les 6h)");
}

// ── Orchestrateur ─────────────────────────────────────────────────────────────

async fn analyser_tous(db: &Arc<Database>) {
    let mut total = 0usize;
    total += analyser_rockets(db).await;
    total += analyser_smc(db).await;
    total += analyser_straddle(db).await;
    if total > 0 {
        tracing::info!("🧠 Patterns d'échec : {} règles mises à jour", total);
    }
}

// ── Rockets ───────────────────────────────────────────────────────────────────

async fn analyser_rockets(db: &Arc<Database>) -> usize {
    let rows = match sqlx::query(
        "SELECT phase, session_active,
                CASE WHEN atr_ratio < 1.5 THEN '<1.5'
                     WHEN atr_ratio < 2.5 THEN '1.5-2.5'
                     ELSE '>2.5' END AS atr_bucket,
                CASE WHEN rsi < 40 THEN '<40'
                     WHEN rsi < 60 THEN '40-60'
                     ELSE '>60' END AS rsi_bucket,
                COUNT(*) as nb,
                AVG(gagnant) * 100.0 as wr
         FROM rockets_feedback
         WHERE verdict IS NOT NULL AND gagnant IS NOT NULL
         GROUP BY phase, session_active, atr_bucket, rsi_bucket
         HAVING nb >= ?",
    )
    .bind(NB_TRADES_MIN)
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("patterns_echec rockets query: {}", e);
            return 0;
        }
    };

    let mut cles_actives = Vec::new();
    let mut nb = 0usize;

    for row in &rows {
        let wr: f64 = sqlx::Row::get::<f64, _>(row, "wr");
        if wr >= WIN_RATE_ECHEC { continue; }

        let phase: String = sqlx::Row::get(row, "phase");
        let session: String = sqlx::Row::get(row, "session_active");
        let atr: String = sqlx::Row::get(row, "atr_bucket");
        let rsi: String = sqlx::Row::get(row, "rsi_bucket");
        let nb_trades: i64 = sqlx::Row::get(row, "nb");

        let cle = format!("rockets|{}|{}|{}|{}", phase, session, atr, rsi);
        let condition = format!(
            "Rockets {} | session={} | ATR×{} | RSI{} → WR {:.0}% sur {} trades",
            phase, session, atr, rsi, wr, nb_trades
        );

        let regle = db::regles_rejet::NouvelleRegle {
            strategie: "ROCKETS",
            condition: &condition,
            cle_unique: &cle,
            win_rate: wr,
            nb_trades,
        };
        if let Err(e) = db::regles_rejet::upsert_regle(db.pool(), &regle).await {
            tracing::warn!("patterns_echec upsert rockets: {}", e);
        } else {
            cles_actives.push(cle);
            nb += 1;
        }
    }

    if let Err(e) = db::regles_rejet::desactiver_obsoletes(db.pool(), "ROCKETS", &cles_actives).await {
        tracing::warn!("patterns_echec desactiver rockets: {}", e);
    }
    nb
}

// ── SMC ───────────────────────────────────────────────────────────────────────

async fn analyser_smc(db: &Arc<Database>) -> usize {
    let rows = match sqlx::query(
        "SELECT categorie, session_active,
                CASE WHEN score_smc < 55 THEN '<55'
                     WHEN score_smc < 70 THEN '55-70'
                     ELSE '>70' END AS score_bucket,
                CASE WHEN confiance_ml < 0.5 THEN '<50%'
                     WHEN confiance_ml < 0.7 THEN '50-70%'
                     ELSE '>70%' END AS ml_bucket,
                COUNT(*) as nb,
                AVG(gagnant) * 100.0 as wr
         FROM smc_feedback
         WHERE verdict IS NOT NULL AND gagnant IS NOT NULL
         GROUP BY categorie, session_active, score_bucket, ml_bucket
         HAVING nb >= ?",
    )
    .bind(NB_TRADES_MIN)
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("patterns_echec smc query: {}", e);
            return 0;
        }
    };

    let mut cles_actives = Vec::new();
    let mut nb = 0usize;

    for row in &rows {
        let wr: f64 = sqlx::Row::get::<f64, _>(row, "wr");
        if wr >= WIN_RATE_ECHEC { continue; }

        let categorie: String = sqlx::Row::get(row, "categorie");
        let session: String = sqlx::Row::get(row, "session_active");
        let score: String = sqlx::Row::get(row, "score_bucket");
        let ml: String = sqlx::Row::get(row, "ml_bucket");
        let nb_trades: i64 = sqlx::Row::get(row, "nb");

        let cle = format!("smc|{}|{}|{}|{}", categorie, session, score, ml);
        let condition = format!(
            "SMC {} | session={} | score{} | ML{} → WR {:.0}% sur {} trades",
            categorie, session, score, ml, wr, nb_trades
        );

        let regle = db::regles_rejet::NouvelleRegle {
            strategie: "SMC",
            condition: &condition,
            cle_unique: &cle,
            win_rate: wr,
            nb_trades,
        };
        if let Err(e) = db::regles_rejet::upsert_regle(db.pool(), &regle).await {
            tracing::warn!("patterns_echec upsert smc: {}", e);
        } else {
            cles_actives.push(cle);
            nb += 1;
        }
    }

    if let Err(e) = db::regles_rejet::desactiver_obsoletes(db.pool(), "SMC", &cles_actives).await {
        tracing::warn!("patterns_echec desactiver smc: {}", e);
    }
    nb
}

// ── Straddle ─────────────────────────────────────────────────────────────────

async fn analyser_straddle(db: &Arc<Database>) -> usize {
    let rows = match sqlx::query(
        "SELECT categorie, session_active,
                CASE WHEN ratio_atr < 1.5 THEN '<1.5'
                     WHEN ratio_atr < 2.5 THEN '1.5-2.5'
                     ELSE '>2.5' END AS atr_bucket,
                CASE WHEN score_llm < 5 THEN '<5'
                     WHEN score_llm < 7 THEN '5-7'
                     ELSE '>7' END AS score_bucket,
                COUNT(*) as nb,
                AVG(gagnant) * 100.0 as wr
         FROM straddle_feedback
         WHERE verdict IS NOT NULL AND gagnant IS NOT NULL
         GROUP BY categorie, session_active, atr_bucket, score_bucket
         HAVING nb >= ?",
    )
    .bind(NB_TRADES_MIN)
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("patterns_echec straddle query: {}", e);
            return 0;
        }
    };

    let mut cles_actives = Vec::new();
    let mut nb = 0usize;

    for row in &rows {
        let wr: f64 = sqlx::Row::get::<f64, _>(row, "wr");
        if wr >= WIN_RATE_ECHEC { continue; }

        let categorie: String = sqlx::Row::get(row, "categorie");
        let session: String = sqlx::Row::get(row, "session_active");
        let atr: String = sqlx::Row::get(row, "atr_bucket");
        let score: String = sqlx::Row::get(row, "score_bucket");
        let nb_trades: i64 = sqlx::Row::get(row, "nb");

        let cle = format!("straddle|{}|{}|{}|{}", categorie, session, atr, score);
        let condition = format!(
            "Straddle {} | session={} | ATR×{} | score{} → WR {:.0}% sur {} trades",
            categorie, session, atr, score, wr, nb_trades
        );

        let regle = db::regles_rejet::NouvelleRegle {
            strategie: "STRADDLE",
            condition: &condition,
            cle_unique: &cle,
            win_rate: wr,
            nb_trades,
        };
        if let Err(e) = db::regles_rejet::upsert_regle(db.pool(), &regle).await {
            tracing::warn!("patterns_echec upsert straddle: {}", e);
        } else {
            cles_actives.push(cle);
            nb += 1;
        }
    }

    if let Err(e) = db::regles_rejet::desactiver_obsoletes(db.pool(), "STRADDLE", &cles_actives).await {
        tracing::warn!("patterns_echec desactiver straddle: {}", e);
    }
    nb
}

// ── Lecture pour injection dans les prompts ───────────────────────────────────

/// Retourne le bloc "Leçons systémiques" à injecter dans un prompt Ollama.
/// Retourne une chaîne vide s'il n'y a aucune règle active.
pub async fn charger_lecons_systemiques(db: &Arc<Database>, strategie: &str) -> String {
    charger_lecons_pool(db.pool(), strategie).await
}

/// Variante acceptant un `SqlitePool` directement (pour les fonctions sans `Arc<Database>`).
pub async fn charger_lecons_pool(pool: &sqlx::SqlitePool, strategie: &str) -> String {
    let regles = match db::regles_rejet::lister_actives(pool, strategie).await {
        Ok(r) => r,
        Err(_) => return String::new(),
    };
    if regles.is_empty() {
        return String::new();
    }
    let mut bloc = String::from("=== LEÇONS SYSTÉMIQUES (patterns d'échec appris) ===\n");
    for r in &regles {
        bloc.push_str(&format!(
            "  ⚠️  REJET RECOMMANDÉ : {} (WR {:.0}%, {} trades)\n",
            r.condition, r.win_rate, r.nb_trades
        ));
    }
    bloc
}
