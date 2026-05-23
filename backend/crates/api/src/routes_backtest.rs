//! Routes dédiées au backtest et aux pré-alertes.
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/pre_alertes",
        web::get().to(crate::prealerte_handlers::get_pre_alertes),
    )
    .route(
        "/api/backtest/lancer",
        web::post().to(crate::backtest_handler::lancer_backtest),
    );
}
