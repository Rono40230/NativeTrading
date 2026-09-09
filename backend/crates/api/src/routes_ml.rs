//! Routes ML — séparées de routes.rs pour respecter la limite de 300 lignes.
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/ml/predict",
        web::get().to(crate::handlers::predict_ml),
    )
    // ── Phase 8 : ML Feedback Loop (stats) ──────────────────────────────────────
    .route(
        "/api/ml/feedback/stats",
        web::get().to(crate::ml_insights_handlers::stats_feedback),
    )
    // ── Phase 8.4 : Réentraînement incrémental ─────────────────────────
    .route(
        "/api/ml/retrain",
        web::post().to(crate::ml_retrain_handler::declencher_retrain),
    )
    .route(
        "/api/ml/retrain/last",
        web::get().to(crate::ml_retrain_handler::dernier_statut_retrain),
    )
    .route(
        "/api/ml/retrain/status/{job_id}",
        web::get().to(crate::ml_retrain_handler::statut_retrain),
    )
    // ── P4 : Feature importance ────────────────────────────────────────
    .route(
        "/api/ml/feature-importance/{strategie}",
        web::get().to(crate::ml_retrain_handler::feature_importance),
    );
}
