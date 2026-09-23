//! Endpoints API pour le monitoring ML de la stratégie SMC Directionnel.
//!
//! Routes :
//!   GET  /api/smc/monitoring-ml  → stats globales + par catégorie + dérive
//!   GET  /api/smc/calibration    → seuils calibrés par asset/timeframe/catégorie
//!   GET  /api/smc/feedback       → historique des feedbacks SMC filtrés
//!   GET  /api/smc/baremes        → constantes SCORE_MAX_* du moteur SMC

use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;


// ── GET /api/smc/monitoring-ml ────────────────────────────────────────────────

pub async fn monitoring_ml(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    // Miroir de l'historique des trades clôturés (décision propriétaire
    // 23/09) : signaux Fermé+rempli, R encaissé pondéré ventes partielles —
    // le MÊME calcul que get_signaux. (L'ancienne source ml_training_samples
    // portait le r distance du 15/09 : Σ +86 R vs −15,6 R encaissés.)
    let mut reponse = crate::ml_monitoring::stats_smc(&state).await;

    // Le TOP des features (permutation OOS) et le dernier entraînement — la
    // matière que la page ML et l'analyste lisent.
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

