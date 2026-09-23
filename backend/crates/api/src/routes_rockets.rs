//! Routes /api/rockets/* — extraites de routes.rs pour respecter la limite de 300 lignes.
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/rockets/monitoring-ml",
        // Migration 23/09 : source = ml_training_samples (l'ancienne table
        // rockets_feedback est morte — la carte affichait des zéros).
        web::get().to(crate::ml_monitoring::rockets),
    )
    .route(
        "/api/kdj/monitoring-ml",
        web::get().to(crate::ml_monitoring::kdj),
    )
    .route(
        "/api/rockets/calibration",
        web::get().to(crate::rockets_ml_handlers::get_calibration),
    )
    .route(
        "/api/rockets/positions",
        web::get().to(crate::rockets_handlers::get_positions),
    )
    .route(
        "/api/rockets/historique",
        web::get().to(crate::rockets_handlers::get_historique),
    )
    .service(
        web::resource("/api/rockets/analyse-llm")
            .route(web::get().to(crate::rockets_analyse_handler::get_derniere_analyse))
            .route(web::post().to(crate::rockets_analyse_handler::lancer_analyse)),
    );
}
