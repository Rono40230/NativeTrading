use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};

mod handlers;
mod state;

use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🚀 Native Trading AI Backend starting...");

    let app_state = web::Data::new(AppState::new().await.expect("Failed to init state"));

    tracing::info!("🌐 Server running on http://0.0.0.0:8080");

    HttpServer::new(move || {
        // CORS limité au dev Tauri uniquement — en production l'app est native (fenêtre Tauri)
        let cors = Cors::default()
            .allowed_origin("tauri://localhost")
            .allowed_origin("http://localhost:1420") // dev Tauri uniquement (port Vite)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
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
            .route("/api/backtest", web::post().to(handlers::run_backtest))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
