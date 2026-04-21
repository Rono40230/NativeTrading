use db::rockets_feedback::{lister_pool_phase, lister_recents_ticker_phase};
use db::rockets_feedback_stats::taux_reussite_recent;
use strategies::rockets_indicateurs::ScanResultat;

/// Prépare le contexte global (taux réussite 48h) et récupère les exemples few-shot (feedbacks).
pub async fn preparer_contexte_feedbacks(
    pool: &sqlx::SqlitePool,
    r: &ScanResultat,
    phase: &str,
) -> ((i64, f64, f64), Vec<db::rockets_feedback::RocketsFeedbackRow>) {
    // Contexte de marché global : taux de réussite des 48 dernières heures
    let stats_recent = taux_reussite_recent(pool, 48).await;

    // Sélection des feedbacks few-shot par similarité de profil :
    let feedbacks = {
        let propres = lister_recents_ticker_phase(pool, &r.ticker, phase, 5)
            .await
            .unwrap_or_default();
        if propres.len() >= 5 {
            propres
        } else {
            // Compléter avec le pool large trié par similarité
            let pool_large = lister_pool_phase(pool, phase, 60)
                .await
                .unwrap_or_default();
            let rv = r.ratio_volume;
            let ar = r.atr_ratio;
            let rsi = r.rsi;
            let mut scored: Vec<_> = pool_large
                .into_iter()
                .filter(|fb| fb.ticker != r.ticker) // déjà dans propres
                .map(|fb| {
                    let dist = ((fb.ratio_volume - rv) / rv.max(0.1)).powi(2)
                        + ((fb.atr_ratio - ar) / ar.max(0.1)).powi(2)
                        + ((fb.rsi - rsi) / 100.0).powi(2);
                    (dist, fb)
                })
                .collect();
            scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut resultats = propres;
            resultats.extend(scored.into_iter().take(5 - resultats.len()).map(|(_, fb)| fb));
            resultats
        }
    };

    (stats_recent, feedbacks)
}
