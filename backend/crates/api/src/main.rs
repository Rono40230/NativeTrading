use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, HttpServer};

mod ab_test_handlers;
mod assets_handlers;
mod backtest_handlers;
mod calendar_handlers;
mod config_handlers;
mod data_handlers;
mod engine_handlers;
mod export_handlers;
mod handlers;
mod indicators_handlers;
mod indicators_types;
mod ml_handlers;
mod news_context_handler;
mod news_fear_greed;
mod news_handlers;
mod news_lus_handlers;
mod news_rss;
mod news_scoring;
mod news_scraper;
mod news_traduction;
mod ollama;
mod ollama_ajustements_handler;
mod ollama_handlers;
mod ollama_types;
mod rockets_analyse;
mod rockets_analyse_handler;
mod rockets_handlers;
mod rockets_scan;
mod rockets_sauvegarder;
mod rockets_suivi;
mod scheduler;
mod sentiment_handlers;
mod signal_engine;
mod signal_engine_analyse;
mod signal_filtre;
mod signaux_handlers;
mod smc_analyse_handler;
mod smc_handlers;
mod state;
mod straddle_backtest_handler;
mod straddle_boucle;
mod straddle_handlers;
mod straddle_prompt;
mod straddle_signal_handler;
mod straddle_slot_backtest;
mod straddle_slot_backtest_fenetre;
mod straddle_types;
mod straddle_utils;
mod strategies_params_handlers;
mod telegram;
mod tendance_handlers;
mod utils;
mod volatility_handlers;
mod ws_handlers;

mod routes;

use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_filename("telegram.env").ok();
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

    let pool_rockets = app_state.db.pool().clone();
    tokio::spawn(rockets_suivi::demarrer_worker_suivi(pool_rockets));

    let pool_scan = app_state.db.pool().clone();
    let signal_engine_rockets = app_state.signal_engine.clone();
    tokio::spawn(rockets_scan::demarrer_worker_scan(
        pool_scan,
        signal_engine_rockets,
    ));

    let pool_analyse = app_state.db.pool().clone();
    tokio::spawn(rockets_analyse_handler::demarrer_worker_analyse(
        pool_analyse,
    ));

    let pool_signaux = app_state.db.pool().clone();
    tokio::spawn(signaux_handlers::demarrer_worker_suivi_signaux(
        pool_signaux,
    ));

    tokio::spawn(smc_analyse_handler::demarrer_worker_analyse_smc(
        app_state.db.clone(),
    ));

    HttpServer::new(move || {
        // CORS limité au dev Tauri uniquement — en production l'app est native (fenêtre Tauri)
        let cors = Cors::default()
            .allowed_origin("tauri://localhost")
            .allowed_origin("http://localhost:1420") // dev Tauri uniquement (port Vite)
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
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
            .configure(routes::configurer)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
