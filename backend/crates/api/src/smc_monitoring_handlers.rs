//! Endpoints API pour le monitoring ML de la stratégie SMC Directionnel.
//!
//! Routes :
//!   GET  /api/smc/monitoring-ml  → stats globales + par catégorie + dérive
//!   GET  /api/smc/calibration    → seuils calibrés par asset/timeframe/catégorie
//!   GET  /api/smc/feedback       → historique des feedbacks SMC filtrés
//!   GET  /api/smc/baremes        → constantes SCORE_MAX_* du moteur SMC

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;


// ── GET /api/smc/monitoring-ml ────────────────────────────────────────────────

pub async fn monitoring_ml(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    // §11-3 (06/09) : la source de vérité est ml_training_samples (boucle v2
    // réalimentée) — smc_feedback est morte depuis le 15/08.
    let globales = match sqlx::query(
        "SELECT COUNT(*) AS nb,
                SUM(CASE WHEN rr_realise > 0 THEN 1 ELSE 0 END) AS gagnants,
                AVG(rr_realise) AS pnl_moyen
         FROM ml_training_samples
         WHERE LOWER(strategie) LIKE '%smc%'
           AND rr_realise IS NOT NULL
           AND LOWER(outcome) NOT IN ('expire','invalide')",
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => {
            use sqlx::Row as _;
            let nb: i64 = r.get("nb");
            let wins: i64 = r.get::<Option<i64>, _>("gagnants").unwrap_or(0);
            serde_json::json!({
                "nb_signals_total":      nb,
                "nb_feedbacks_clotures": nb,
                "nb_gagnants":           wins,
                "nb_perdants":           nb - wins,
                "nb_invalides":          0,
                "win_rate_global":       if nb > 0 { wins as f64 / nb as f64 } else { 0.0 },
                "pnl_moyen_r":           r.get::<Option<f64>, _>("pnl_moyen"),
                "derniere_maj":          chrono::Utc::now().timestamp(),
            })
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    // Stats par catégorie SMC
    // Par verdict (v2 : les catégories vivaient dans smc_feedback morte).
    let rows = sqlx::query(
        "SELECT outcome AS categorie,
                COUNT(*) AS nb_trades,
                SUM(CASE WHEN rr_realise > 0 THEN 1 ELSE 0 END) AS nb_gagnants,
                AVG(rr_realise) AS pnl_r_moyen
         FROM ml_training_samples
         WHERE LOWER(strategie) LIKE '%smc%'
           AND rr_realise IS NOT NULL
           AND LOWER(outcome) NOT IN ('expire','invalide')
         GROUP BY outcome
         ORDER BY nb_trades DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let par_categorie: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            let wr = if nb > 0 { wins as f64 / nb as f64 } else { 0.0 };
            serde_json::json!({
                "categorie":   r.get::<String, _>("categorie"),
                "nb_trades":   nb,
                "win_rate":    wr,
                "pnl_r_moyen": r.get::<Option<f64>, _>("pnl_r_moyen"),
            })
        })
        .collect();

    // Détection de dérive : win rate des 20 derniers trades < 45 %
    let recents = sqlx::query(
        "SELECT CASE WHEN rr_realise > 0 THEN 1 ELSE 0 END AS gagnant
         FROM ml_training_samples
         WHERE LOWER(strategie) LIKE '%smc%'
           AND rr_realise IS NOT NULL
           AND LOWER(outcome) NOT IN ('expire','invalide')
         ORDER BY cree_le DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let nb_rec = recents.len() as f64;
    let derive_detectee = if nb_rec >= 10.0 {
        let wins: f64 = recents
            .iter()
            .filter(|r| r.get::<Option<i64>, _>("gagnant").unwrap_or(0) == 1)
            .count() as f64;
        wins / nb_rec < 0.45
    } else {
        false
    };

    let mut reponse = globales;
    reponse["par_categorie"] = serde_json::Value::Array(par_categorie);
    reponse["derive_detectee"] = serde_json::Value::Bool(derive_detectee);
    // §11-3 (06/09) : le TOP des features (permutation OOS) et le dernier
    // entraînement — la matière que le Dashboard LLM et l'analyste lisent.
    if let Ok(top) = db::ml_feature_importance::lire_top_importances(pool, "smc", 8).await {
        let arr: Vec<serde_json::Value> = top
            .iter()
            .map(|f| serde_json::json!({
                "feature_nom": f.feature_nom,
                "importance":  f.importance,
            }))
            .collect();
        reponse["features_importances"] = serde_json::Value::Array(arr);
    }
    if let Ok(Some(ent)) = sqlx::query(
        "SELECT asset, timeframe, nb_samples, accuracy_val, cree_le AS date
         FROM historique_entrainements ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    {
        use sqlx::Row as _;
        reponse["dernier_entrainement"] = serde_json::json!({
            "asset": ent.get::<String, _>("asset"),
            "timeframe": ent.get::<String, _>("timeframe"),
            "nb_samples": ent.get::<i64, _>("nb_samples"),
            "accuracy": ent.get::<f64, _>("accuracy_val"),
            "date": ent.get::<i64, _>("date"),
        });
    }

    HttpResponse::Ok().json(reponse)
}

// ── GET /api/smc/calibration ──────────────────────────────────────────────────

pub async fn get_calibration(state: web::Data<AppState>) -> impl Responder {
    match db::smc_calibration::lister_toutes(state.db.pool()).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── GET /api/smc/feedback ─────────────────────────────────────────────────────


// ── GET /api/smc/equity ─────────────────────────────────────────────────────


// ── GET /api/smc/baremes ──────────────────────────────────────────────────────

