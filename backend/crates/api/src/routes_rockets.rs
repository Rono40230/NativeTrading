//! Routes /api/rockets/* — extraites de routes.rs pour respecter la limite de 300 lignes.
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/rockets/monitoring-ml",
        web::get().to(crate::rockets_ml_handlers::monitoring_ml),
    )
    .route(
        "/api/rockets/calibration",
        web::get().to(crate::rockets_ml_handlers::get_calibration),
    )
    .route(
        "/api/rockets/feedback",
        web::get().to(crate::rockets_ml_handlers::get_feedback),
    )
    .route(
        "/api/rockets/signal",
        web::post().to(crate::rockets_handlers::sauvegarder_signal),
    )
    .route(
        "/api/rockets/scan",
        web::get().to(crate::rockets_handlers::get_scan),
    )
    .route(
        "/api/rockets/scan/debug",
        web::get().to(crate::rockets_handlers::scan_momentum_debug),
    )
    .route(
        "/api/rockets/historique",
        web::get().to(crate::rockets_handlers::get_historique),
    )
    .route(
        "/api/rockets/sync",
        web::post().to(crate::rockets_suivi::sync_verdicts),
    )
    .service(
        web::resource("/api/rockets/config")
            .route(web::get().to(crate::rockets_handlers::get_config))
            .route(web::put().to(crate::rockets_handlers::put_config)),
    )
    .service(
        web::resource("/api/rockets/analyse-llm")
            .route(web::get().to(crate::rockets_analyse_handler::get_derniere_analyse))
            .route(web::post().to(crate::rockets_analyse_handler::lancer_analyse)),
    );
}
