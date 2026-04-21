//! Calcul et mise à jour périodique des seuils calibrés Rockets.
//!
//! Tourne toutes les 6h. Pour chaque (phase, session) avec assez de feedbacks,
//! calcule les seuils score_min et conviction_min optimaux via grid search.
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ── Constantes ────────────────────────────────────────────────────────────────

const SCORE_GRID: &[i64] = &[55, 60, 65, 70, 75, 80];
const CONVICTION_GRID: &[i64] = &[55, 60, 65, 70, 75];
const WIN_RATE_MIN: f64 = 0.50;
const NB_TRADES_MIN: i64 = 20;

// ── Point d'entrée ────────────────────────────────────────────────────────────

/// Démarre le job de calibration Rockets en background (toutes les 6h).
pub fn demarrer_calibration_rockets(db: Arc<Database>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(600)).await;
        loop {
            recalibrer_tous(&db).await;
            sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
    tracing::info!("📐 Job calibration Rockets démarré (toutes les 6h)");
}

// ── Logique principale ────────────────────────────────────────────────────────

async fn recalibrer_tous(db: &Arc<Database>) {
    let paires = match lister_paires_actives(db).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Calibration Rockets: chargement paires: {}", e);
            return;
        }
    };

    let mut recalibrees = 0usize;
    for (phase, session) in &paires {
        match calibrer_paire(db, phase, session).await {
            Ok(row) => {
                if let Err(e) = db::rockets_calibration::sauvegarder(db.pool(), &row).await {
                    tracing::warn!("Calibration Rockets save {}/{}: {}", phase, session, e);
                } else {
                    recalibrees += 1;
                    tracing::debug!(
                        "📐 Rockets {}/{}: score≥{} conviction≥{} WR={:.0}% [{}]{}",
                        phase,
                        session,
                        row.score_min,
                        row.conviction_min,
                        row.win_rate * 100.0,
                        row.fiabilite,
                        if row.invalide { " ⚠️ INVALIDE" } else { "" },
                    );
                }
            }
            Err(e) => tracing::warn!("Calibration Rockets {}/{}: {}", phase, session, e),
        }
    }

    if recalibrees > 0 {
        tracing::info!(
            "📐 Calibration Rockets terminée: {}/{} paires recalibrées",
            recalibrees,
            paires.len()
        );
    } else {
        tracing::debug!("Calibration Rockets: 0/{} paires (aucune donnée suffisante)", paires.len());
    }
}

async fn calibrer_paire(
    db: &Arc<Database>,
    phase: &str,
    session: &str,
) -> common::Result<db::rockets_calibration::RocketsCalibrationRow> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT score_scan, conviction_llm, pnl_r, gagnant
         FROM rockets_feedback
         WHERE phase = ? AND session_active = ? AND verdict IS NOT NULL",
    )
    .bind(phase)
    .bind(session)
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    let nb_total = rows.len() as i64;
    let fiabilite = fiabilite(nb_total);

    // (score_scan, conviction_llm, pnl_r, gagnant)
    let trades: Vec<(i64, i64, f64, i64)> = rows
        .iter()
        .map(|r| {
            (
                r.get::<i64, _>("score_scan"),
                r.get::<i64, _>("conviction_llm"),
                r.get::<Option<f64>, _>("pnl_r").unwrap_or(0.0),
                r.get::<Option<i64>, _>("gagnant").unwrap_or(0),
            )
        })
        .collect();

    let (score_optimal, conviction_optimal, invalide) = optimiser_seuils(&trades);

    let nb_gagnants = trades.iter().filter(|t| t.3 == 1).count() as i64;
    let win_rate = if nb_total > 0 {
        nb_gagnants as f64 / nb_total as f64
    } else {
        0.0
    };
    let pnl_moyen = if nb_total > 0 {
        trades.iter().map(|t| t.2).sum::<f64>() / nb_total as f64
    } else {
        0.0
    };

    Ok(db::rockets_calibration::RocketsCalibrationRow {
        phase: phase.to_string(),
        session: session.to_string(),
        score_min: score_optimal,
        conviction_min: conviction_optimal,
        nb_trades: nb_total,
        win_rate,
        pnl_moyen_r: pnl_moyen,
        fiabilite: fiabilite.to_string(),
        invalide,
        maj_le: chrono::Utc::now().timestamp(),
    })
}

// ── Algorithmes ───────────────────────────────────────────────────────────────

/// Grid search 2D sur (score_min, conviction_min) maximisant win_rate × profit_factor.
fn optimiser_seuils(trades: &[(i64, i64, f64, i64)]) -> (i64, i64, bool) {
    let mut meilleur_score_global = 0.0f64;
    let mut score_optimal = 65i64;
    let mut conviction_optimal = 65i64;
    let mut au_moins_un_valide = false;

    for &score_seuil in SCORE_GRID {
        for &conviction_seuil in CONVICTION_GRID {
            let filtre: Vec<_> = trades
                .iter()
                .filter(|t| t.0 >= score_seuil && t.1 >= conviction_seuil)
                .collect();

            if (filtre.len() as i64) < 3 {
                continue;
            }

            let nb = filtre.len() as f64;
            let gagnants = filtre.iter().filter(|t| t.3 == 1).count() as f64;
            let win_rate = gagnants / nb;

            if win_rate < WIN_RATE_MIN {
                continue;
            }

            au_moins_un_valide = true;

            let gains: f64 = filtre.iter().filter(|t| t.2 > 0.0).map(|t| t.2).sum();
            let pertes: f64 = filtre.iter().filter(|t| t.2 < 0.0).map(|t| t.2.abs()).sum();
            let pf = if pertes > 0.0 { gains / pertes } else { gains };
            let score_combo = win_rate * pf;

            if score_combo > meilleur_score_global {
                meilleur_score_global = score_combo;
                score_optimal = score_seuil;
                conviction_optimal = conviction_seuil;
            }
        }
    }

    (score_optimal, conviction_optimal, !au_moins_un_valide)
}

fn fiabilite(nb: i64) -> &'static str {
    match nb {
        0..=9 => "insuffisant",
        10..=34 => "faible",
        35..=99 => "correct",
        _ => "fort",
    }
}

// ── Requête helper ────────────────────────────────────────────────────────────

async fn lister_paires_actives(db: &Arc<Database>) -> common::Result<Vec<(String, String)>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT DISTINCT phase, session_active FROM rockets_feedback
         WHERE verdict IS NOT NULL
         GROUP BY phase, session_active
         HAVING COUNT(*) >= ?",
    )
    .bind(NB_TRADES_MIN)
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("phase"),
                r.get::<String, _>("session_active"),
            )
        })
        .collect())
}
