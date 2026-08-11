//! Routes dédiées aux pré-alertes (feature live).
//! (Le backtest a été supprimé — décision D1 ; la route /api/backtest/lancer est retirée.)
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/pre_alertes",
        web::get().to(crate::prealerte_handlers::get_pre_alertes),
    );
}
