//! Handlers de surveillance temps réel et monitoring ML Straddle.
//! Extrait de straddle_ml_handlers pour respecter la limite 300 lignes.
use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

// ── GET /api/straddle/volatilite-live ────────────────────────────────────────


// ── GET /api/straddle/monitoring-ml ──────────────────────────────────────────

pub async fn monitoring_ml(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    let globales = match db::straddle_feedback::stats_globales(pool).await {
        Ok(g) => g,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let rows = sqlx::query(
        "SELECT categorie,
                COUNT(*) as nb_trades,
                SUM(gagnant) as nb_gagnants,
                AVG(CASE WHEN gagnant = 1 THEN score_llm END) as score_moyen_win,
                AVG(CASE WHEN gagnant = 0 THEN score_llm END) as score_moyen_lose,
                AVG(pnl_r) as pnl_r_moyen
         FROM straddle_feedback
         WHERE verdict IS NOT NULL
         GROUP BY categorie
         ORDER BY nb_trades DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    use sqlx::Row;
    let par_categorie: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            let wr = if nb > 0 { wins as f64 / nb as f64 } else { 0.0 };
            serde_json::json!({
                "categorie":      r.get::<String, _>("categorie"),
                "nb_trades":      nb,
                "win_rate":       wr,
                "score_llm_win":  r.get::<Option<f64>, _>("score_moyen_win"),
                "score_llm_lose": r.get::<Option<f64>, _>("score_moyen_lose"),
                "pnl_r_moyen":    r.get::<Option<f64>, _>("pnl_r_moyen"),
            })
        })
        .collect();

    let recents = sqlx::query(
        "SELECT gagnant FROM straddle_feedback
         WHERE verdict IS NOT NULL ORDER BY ferme_le DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let nb_recents = recents.len() as f64;
    let derive_detectee = if nb_recents >= 10.0 {
        let wins: f64 = recents
            .iter()
            .filter(|r| r.get::<Option<i64>, _>("gagnant").unwrap_or(0) == 1)
            .count() as f64;
        wins / nb_recents < 0.45
    } else {
        false
    };

    let mut reponse = globales;
    reponse["par_categorie"] = serde_json::Value::Array(par_categorie);
    reponse["derive_detectee"] = serde_json::Value::Bool(derive_detectee);

    HttpResponse::Ok().json(reponse)
}
