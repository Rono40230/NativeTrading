use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, HttpServer};

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
mod news_handlers;
mod news_rss;
mod news_scraper;
mod news_traduction;
mod ollama;
mod ollama_handlers;
mod ollama_types;
mod rockets_analyse_handler;
mod rockets_handlers;
mod rockets_indicateurs;
mod rockets_scan;
mod rockets_suivi;
mod scheduler;
mod sentiment_handlers;
mod signal_engine;
mod signaux_handlers;
mod smc_handlers;
mod state;
mod straddle_handlers;
mod straddle_precision;
mod tendance_handlers;
mod utils;
mod volatility_handlers;
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

    let pool_rockets = app_state.db.pool().clone();
    tokio::spawn(rockets_suivi::demarrer_worker_suivi(pool_rockets));

    let pool_scan = app_state.db.pool().clone();
    tokio::spawn(rockets_scan::demarrer_worker_scan(pool_scan));

    let pool_analyse = app_state.db.pool().clone();
    tokio::spawn(rockets_analyse_handler::demarrer_worker_analyse(
        pool_analyse,
    ));

    let pool_signaux = app_state.db.pool().clone();
    tokio::spawn(signaux_handlers::demarrer_worker_suivi_signaux(
        pool_signaux,
    ));

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
            .route("/api/assets", web::get().to(assets_handlers::lister_assets))
            .route(
                "/api/assets",
                web::post().to(assets_handlers::ajouter_asset),
            )
            .route(
                "/api/assets/{id}",
                web::delete().to(assets_handlers::supprimer_asset),
            )
            .route("/api/candles", web::get().to(handlers::get_candles))
            .route("/api/prix-actuel", web::get().to(handlers::get_prix_actuel))
            .route("/api/signaux", web::get().to(signaux_handlers::get_signaux))
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
            .route(
                "/api/ia/signal/straddle",
                web::post().to(straddle_handlers::analyser),
            )
            .route(
                "/api/straddle/analyser",
                web::post().to(straddle_handlers::analyser),
            )
            .route(
                "/api/straddle/creneaux",
                web::get().to(straddle_handlers::lister_creneaux),
            )
            .route(
                "/api/straddle/creneaux/{id}",
                web::patch().to(straddle_handlers::mettre_a_jour_creneau),
            )
            .route(
                "/api/straddle/creneaux/{id}/precision",
                web::post().to(straddle_handlers::analyser_precision),
            )
            .route("/api/config", web::get().to(config_handlers::get_config))
            .route("/api/config", web::post().to(config_handlers::post_config))
            .route("/api/ib/status", web::get().to(handlers::ib_status))
            .route(
                "/api/calendar",
                web::get().to(calendar_handlers::get_calendar),
            )
            .route(
                "/api/sentiment/marche",
                web::get().to(sentiment_handlers::get_sentiment_marche),
            )
            .route(
                "/api/news/alertes",
                web::get().to(news_handlers::get_news_alertes),
            )
            .route(
                "/api/news/contenu",
                web::get().to(news_handlers::get_contenu_article),
            )
            .route(
                "/api/news/traduire",
                web::get().to(news_handlers::get_traduire),
            )
            .route("/api/stream", web::get().to(ws_handlers::stream_market))
            .route(
                "/api/signal-engine/start",
                web::post().to(engine_handlers::demarrer_engine),
            )
            .route(
                "/api/signal-engine/stop",
                web::post().to(engine_handlers::arreter_engine),
            )
            .route(
                "/api/signal-engine/status",
                web::get().to(engine_handlers::statut_engine),
            )
            .route(
                "/api/signal-engine/stream",
                web::get().to(engine_handlers::stream_signaux),
            )
            .route(
                "/api/data/coverage",
                web::get().to(data_handlers::get_coverage),
            )
            .route(
                "/api/data/collect",
                web::post().to(data_handlers::post_collect),
            )
            .route("/api/ml/history", web::get().to(ml_handlers::historique_ml))
            .route(
                "/api/rockets/signal",
                web::post().to(rockets_handlers::sauvegarder_signal),
            )
            .route(
                "/api/rockets/scan",
                web::get().to(rockets_handlers::get_scan),
            )
            .route(
                "/api/rockets/historique",
                web::get().to(rockets_handlers::get_historique),
            )
            .route(
                "/api/rockets/sync",
                web::post().to(rockets_suivi::sync_verdicts),
            )
            .service(
                web::resource("/api/rockets/config")
                    .route(web::get().to(rockets_handlers::get_config))
                    .route(web::put().to(rockets_handlers::put_config)),
            )
            .service(
                web::resource("/api/rockets/analyse-llm")
                    .route(web::get().to(rockets_analyse_handler::get_derniere_analyse))
                    .route(web::post().to(rockets_analyse_handler::lancer_analyse)),
            )
            .route(
                "/api/volatility/patterns",
                web::get().to(volatility_handlers::get_patterns),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
