//! Routes de la verticale KDJ/Halftrend (7.E).

use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/kdj/params", web::get().to(crate::kdj_handlers::get_params))
        .route("/api/kdj/params", web::put().to(crate::kdj_handlers::put_params))
        .route("/api/kdj/scanner", web::get().to(crate::kdj_handlers::get_scanner));
}
