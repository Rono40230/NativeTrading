//! Routes ML — séparées de routes.rs pour respecter la limite de 300 lignes.
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg
        .route(
            "/api/ml/predict",
            web::get().to(crate::handlers::predict_ml),
        )
        .route(
            "/api/ml/train",
            web::post().to(crate::ml_handlers::entrainer_ml),
        )
        .route(
            "/api/ml/status",
            web::get().to(crate::ml_handlers::statut_ml),
        )
        .route(
            "/api/ml/history",
            web::get().to(crate::ml_handlers::historique_ml),
        )
        .route(
            "/api/backtest/raffiner-ml",
            web::post().to(crate::backtest_handlers::raffiner_ml),
        )
        // ── Phase 8 : ML Feedback Loop ──────────────────────────────────────
        .route(
            "/api/ml/feedback/stats",
            web::get().to(crate::ml_insights_handlers::stats_feedback),
        )
        .route(
            "/api/ml/suggestions",
            web::get().to(crate::ml_insights_handlers::suggestions),
        )
        .route(
            "/api/ml/suggestions/appliquer",
            web::post().to(crate::ml_insights_handlers::appliquer_suggestion),
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
        );
}
