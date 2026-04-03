//! Calcul et mise à jour périodique des seuils calibrés Straddle.
//!
//! Tourne toutes les 6h. Pour chaque (asset, categorie) avec assez de feedbacks,
//! calcule le seuil score LLM optimal et le ratio ATR minimal via grid search.
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ── Constantes ────────────────────────────────────────────────────────────────

const SEUIL_GRID: &[f64] = &[4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 9.0, 9.5];
const ATR_GRID: &[f64] = &[1.2, 1.3, 1.4, 1.5, 1.6, 1.8, 2.0, 2.5];
const WIN_RATE_MIN: f64 = 0.50;  // Catégorie invalide si WR < 50% sur tout seuil

// ── Point d'entrée ────────────────────────────────────────────────────────────

/// Démarre le job de calibration en background (toutes les 6h).
pub fn demarrer_calibration(db: Arc<Database>) {
    tokio::spawn(async move {
        // Délai initial : laisser les jobs de feedback remplir quelques données
        sleep(Duration::from_secs(300)).await;
        loop {
            recalibrer_tous(&db).await;
            sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
    tracing::info!("📐 Job calibration Straddle démarré (toutes les 6h)");
}

// ── Logique principale ────────────────────────────────────────────────────────

async fn recalibrer_tous(db: &Arc<Database>) {
    // Récupérer tous les (asset, categorie) distincts avec au moins 5 feedbacks clôturés
    let paires = match lister_paires_actives(db).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Calibration: chargement paires: {}", e);
            return;
        }
    };

    let mut recalibrees = 0usize;
    for (asset, categorie) in &paires {
        match calibrer_paire(db, asset, categorie).await {
            Ok(row) => {
                if let Err(e) = db::straddle_calibration::sauvegarder(db.pool(), &row).await {
                    tracing::warn!("Calibration save {}/{}: {}", asset, categorie, e);
                } else {
                    recalibrees += 1;
                    tracing::debug!(
                        "📐 Calibration {}/{}: score≥{:.1} atr≥{:.2} WR={:.0}% [{}]{}",
                        asset,
                        categorie,
                        row.score_llm_seuil,
                        row.atr_seuil,
                        row.win_rate * 100.0,
                        row.fiabilite,
                        if row.invalide { " ⚠️ INVALIDE" } else { "" },
                    );
                }
            }
            Err(e) => tracing::warn!("Calibration {}/{}: {}", asset, categorie, e),
        }
    }

    tracing::info!(
        "📐 Calibration terminée: {}/{} paires recalibrées",
        recalibrees,
        paires.len()
    );
}

async fn calibrer_paire(
    db: &Arc<Database>,
    asset: &str,
    categorie: &str,
) -> common::Result<db::straddle_calibration::CalibrationRow> {
    use sqlx::Row;

    // Charger tous les feedbacks clôturés pour cette paire
    let rows = sqlx::query(
        "SELECT score_llm, ratio_atr, pnl_r, gagnant
         FROM straddle_feedback
         WHERE asset = ? AND categorie = ? AND verdict IS NOT NULL",
    )
    .bind(asset)
    .bind(categorie)
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    let nb_total = rows.len() as i64;
    let fiabilite = fiabilite(nb_total);

    // Données brutes
    let trades: Vec<(f64, f64, f64, i64)> = rows
        .iter()
        .map(|r| {
            (
                r.get::<f64, _>("score_llm"),
                r.get::<f64, _>("ratio_atr"),
                r.get::<Option<f64>, _>("pnl_r").unwrap_or(0.0),
                r.get::<Option<i64>, _>("gagnant").unwrap_or(0),
            )
        })
        .collect();

    // Grid search : meilleur seuil score LLM
    let (score_optimal, invalide) = optimiser_score_seuil(&trades);

    // Grid search : meilleur seuil ATR (parmi trades avec score >= score_optimal)
    let atr_optimal = optimiser_atr_seuil(&trades, score_optimal);

    // Statistiques globales
    let nb_gagnants = trades.iter().filter(|t| t.3 == 1).count() as i64;
    let win_rate = if nb_total > 0 { nb_gagnants as f64 / nb_total as f64 } else { 0.0 };
    let pnl_moyen = if nb_total > 0 {
        trades.iter().map(|t| t.2).sum::<f64>() / nb_total as f64
    } else {
        0.0
    };

    Ok(db::straddle_calibration::CalibrationRow {
        asset: asset.to_string(),
        categorie: categorie.to_string(),
        score_llm_seuil: score_optimal,
        atr_seuil: atr_optimal,
        nb_trades: nb_total,
        win_rate,
        pnl_moyen_r: pnl_moyen,
        fiabilite: fiabilite.to_string(),
        invalide,
        maj_le: chrono::Utc::now().timestamp(),
    })
}

// ── Algorithmes ───────────────────────────────────────────────────────────────

/// Cherche le seuil score LLM qui maximise win_rate × profit_factor.
/// Retourne (seuil_optimal, invalide).
fn optimiser_score_seuil(trades: &[(f64, f64, f64, i64)]) -> (f64, bool) {
    let mut meilleur_score = 0.0f64;
    let mut meilleur_seuil = 6.0f64;
    let mut au_moins_un_valide = false;

    for &seuil in SEUIL_GRID {
        let filtre: Vec<_> = trades.iter().filter(|t| t.0 >= seuil).collect();
        if filtre.len() < 3 {
            continue; // pas assez de données pour ce seuil
        }
        let nb = filtre.len() as f64;
        let gagnants = filtre.iter().filter(|t| t.3 == 1).count() as f64;
        let win_rate = gagnants / nb;

        if win_rate < WIN_RATE_MIN {
            continue;
        }

        au_moins_un_valide = true;

        // Profit factor : somme gains / somme pertes (en R)
        let gains: f64 = filtre.iter().filter(|t| t.2 > 0.0).map(|t| t.2).sum();
        let pertes: f64 = filtre.iter().filter(|t| t.2 < 0.0).map(|t| t.2.abs()).sum();
        let pf = if pertes > 0.0 { gains / pertes } else { gains };
        let score_global = win_rate * pf;

        if score_global > meilleur_score {
            meilleur_score = score_global;
            meilleur_seuil = seuil;
        }
    }

    (meilleur_seuil, !au_moins_un_valide)
}

/// Cherche le seuil ATR minimal optimal (parmi les trades filtrés par score).
fn optimiser_atr_seuil(trades: &[(f64, f64, f64, i64)], score_min: f64) -> f64 {
    let trades_filtres: Vec<_> = trades.iter().filter(|t| t.0 >= score_min).collect();
    if trades_filtres.is_empty() {
        return 1.5;
    }

    let mut meilleur_score = 0.0f64;
    let mut meilleur_atr = 1.5f64;

    for &atr_seuil in ATR_GRID {
        let filtre: Vec<_> = trades_filtres.iter().filter(|t| t.1 >= atr_seuil).collect();
        if filtre.len() < 3 {
            continue;
        }
        let nb = filtre.len() as f64;
        let gagnants = filtre.iter().filter(|t| t.3 == 1).count() as f64;
        let win_rate = gagnants / nb;
        if win_rate > meilleur_score {
            meilleur_score = win_rate;
            meilleur_atr = atr_seuil;
        }
    }

    meilleur_atr
}

fn fiabilite(nb: i64) -> &'static str {
    match nb {
        0..=9 => "insuffisant",
        10..=29 => "faible",
        30..=99 => "correct",
        _ => "fort",
    }
}

// ── Requête helper ────────────────────────────────────────────────────────────

async fn lister_paires_actives(db: &Arc<Database>) -> common::Result<Vec<(String, String)>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT DISTINCT asset, categorie FROM straddle_feedback
         WHERE verdict IS NOT NULL
         GROUP BY asset, categorie HAVING COUNT(*) >= 5",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| (r.get::<String, _>("asset"), r.get::<String, _>("categorie")))
        .collect())
}
