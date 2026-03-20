use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, HttpServer};

mod backtest_handlers;
mod calendar_handlers;
mod config_handlers;
mod export_handlers;
mod handlers;
mod indicators_handlers;
mod indicators_types;
mod ml_handlers;
mod ollama;
mod ollama_handlers;
mod ollama_types;
mod smc_handlers;
mod state;
mod tendance_handlers;
mod utils;
mod ws_handlers;

use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🚀 Native Trading AI Backend starting...");

    let app_state = match AppState::new().await {
        Ok(state) => web::Data::new(state),
        Err(e) => {
            tracing::error!("Échec initialisation état applicatif: {}", e);
            return Err(std::io::Error::other(e.to_string()));
        }
    };

    tracing::info!("🌐 Server running on http://0.0.0.0:8080");

    HttpServer::new(move || {
        // CORS limité au dev Tauri uniquement — en production l'app est native (fenêtre Tauri)
        let cors = Cors::default()
            .allowed_origin("tauri://localhost")
            .allowed_origin("http://localhost:1420") // dev Tauri uniquement (port Vite)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .route("/health", web::get().to(handlers::health_check))
            .route("/api/assets", web::get().to(handlers::get_assets))
            .route("/api/candles", web::get().to(handlers::get_candles))
            .route("/api/signaux", web::get().to(handlers::get_signaux))
            .route("/api/ml/predict", web::get().to(handlers::predict_ml))
            .route("/api/ml/train", web::post().to(ml_handlers::entrainer_ml))
            .route("/api/ml/status", web::get().to(ml_handlers::statut_ml))
            .route(
                "/api/backtest",
                web::post().to(backtest_handlers::run_backtest),
            )
            .route(
                "/api/signaux/export",
                web::get().to(export_handlers::exporter_signaux_csv),
            )
            .route("/api/smc/analyse", web::get().to(smc_handlers::analyse_smc))
            .route(
                "/api/indicators",
                web::get().to(indicators_handlers::get_indicators),
            )
            .route(
                "/api/tendance/multi-tf",
                web::get().to(tendance_handlers::tendance_multi_tf),
            )
            .route("/api/ia/analyse", web::post().to(ollama_handlers::analyser))
            .route("/api/ia/chat", web::post().to(ollama_handlers::chat))
            .route(
                "/api/ia/chart",
                web::post().to(ollama_handlers::analyser_chart),
            )
            .route("/api/ia/status", web::get().to(ollama_handlers::statut))
            .route(
                "/api/ia/signal",
                web::post().to(ollama_handlers::generer_signal),
            )
            .route("/api/config", web::get().to(config_handlers::get_config))
            .route("/api/config", web::post().to(config_handlers::post_config))
            .route("/api/ib/status", web::get().to(handlers::ib_status))
            .route(
                "/api/calendar",
                web::get().to(calendar_handlers::get_calendar),
            )
            .route("/api/stream", web::get().to(ws_handlers::stream_market))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
