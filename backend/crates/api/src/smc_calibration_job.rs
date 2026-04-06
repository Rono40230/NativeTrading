//! Calcul et mise à jour périodique des seuils calibrés SMC Directionnel.
//!
//! Tourne toutes les 6h. Pour chaque (asset, timeframe, categorie) avec assez
//! de feedbacks clôturés, calcule les seuils score_smc et conviction optimaux
//! via grid search 2D maximisant win_rate × profit_factor.
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ── Constantes ────────────────────────────────────────────────────────────────

const SCORE_SMC_GRID: &[f64] = &[60.0, 65.0, 70.0, 75.0, 80.0];
const CONVICTION_GRID: &[i64] = &[55, 60, 65, 70, 75];
const WIN_RATE_MIN: f64 = 0.50;
const NB_TRADES_MIN: i64 = 15;

// ── Point d'entrée ────────────────────────────────────────────────────────────

/// Démarre le job de calibration SMC en background (toutes les 6h).
pub fn demarrer_calibration_smc(db: Arc<Database>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(600)).await;
        loop {
            recalibrer_tous(&db).await;
            sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
    tracing::info!("📐 Job calibration SMC démarré (toutes les 6h)");
}

// ── Logique principale ────────────────────────────────────────────────────────

async fn recalibrer_tous(db: &Arc<Database>) {
    let triplets = match lister_triplets_actifs(db).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Calibration SMC: chargement triplets: {}", e);
            return;
        }
    };

    let mut recalibrees = 0usize;
    for (asset, timeframe, categorie) in &triplets {
        match calibrer_triplet(db, asset, timeframe, categorie).await {
            Ok(row) => {
                if let Err(e) = db::smc_calibration::sauvegarder(db.pool(), &row).await {
                    tracing::warn!(
                        "Calibration SMC save {}/{}/{}: {}",
                        asset,
                        timeframe,
                        categorie,
                        e
                    );
                } else {
                    recalibrees += 1;
                    tracing::debug!(
                        "📐 SMC {}/{}/{}: score≥{:.0} conviction≥{} WR={:.0}% [{}]{}",
                        asset,
                        timeframe,
                        categorie,
                        row.score_smc_seuil,
                        row.conviction_seuil,
                        row.win_rate * 100.0,
                        row.fiabilite,
                        if row.invalide { " ⚠️ INVALIDE" } else { "" },
                    );
                }
            }
            Err(e) => tracing::warn!(
                "Calibration SMC {}/{}/{}: {}",
                asset,
                timeframe,
                categorie,
                e
            ),
        }
    }

    tracing::info!(
        "📐 Calibration SMC terminée: {}/{} triplets recalibrés",
        recalibrees,
        triplets.len()
    );
}

async fn calibrer_triplet(
    db: &Arc<Database>,
    asset: &str,
    timeframe: &str,
    categorie: &str,
) -> common::Result<db::smc_calibration::SmcCalibrationRow> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT score_smc, conviction_llm, pnl_r, gagnant
         FROM smc_feedback
         WHERE asset = ? AND timeframe = ? AND categorie = ? AND verdict IS NOT NULL",
    )
    .bind(asset)
    .bind(timeframe)
    .bind(categorie)
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    let nb_total = rows.len() as i64;
    let fiabilite = fiabilite(nb_total);

    // (score_smc, conviction_llm, pnl_r, gagnant)
    let trades: Vec<(f64, i64, f64, i64)> = rows
        .iter()
        .map(|r| {
            (
                r.get::<f64, _>("score_smc"),
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

    Ok(db::smc_calibration::SmcCalibrationRow {
        asset: asset.to_string(),
        timeframe: timeframe.to_string(),
        categorie: categorie.to_string(),
        score_smc_seuil: score_optimal,
        conviction_seuil: conviction_optimal,
        nb_trades: nb_total,
        win_rate,
        pnl_moyen_r: pnl_moyen,
        fiabilite: fiabilite.to_string(),
        invalide,
        maj_le: chrono::Utc::now().timestamp(),
    })
}

// ── Grid search ───────────────────────────────────────────────────────────────

/// Maximise win_rate × profit_factor sur la grille (score_smc, conviction).
fn optimiser_seuils(trades: &[(f64, i64, f64, i64)]) -> (f64, i64, bool) {
    let mut meilleur_score_global = 0.0f64;
    let mut score_optimal = 70.0f64;
    let mut conviction_optimal = 70i64;
    let mut au_moins_un_valide = false;

    for &score_seuil in SCORE_SMC_GRID {
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

async fn lister_triplets_actifs(
    db: &Arc<Database>,
) -> common::Result<Vec<(String, String, String)>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT DISTINCT asset, timeframe, categorie FROM smc_feedback
         WHERE verdict IS NOT NULL
         GROUP BY asset, timeframe, categorie
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
                r.get::<String, _>("asset"),
                r.get::<String, _>("timeframe"),
                r.get::<String, _>("categorie"),
            )
        })
        .collect())
}
