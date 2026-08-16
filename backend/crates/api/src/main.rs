use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer};

mod ab_test_handlers;
mod asset_params_handlers;
mod assets_handlers;
mod calendar_handlers;
mod config_handlers;
mod data_handlers;
mod dukascopy_handlers;
mod engine_handlers;
mod handlers;
mod http_client;
mod indicators_handlers;
mod indicators_types;
mod ml_handlers;
mod ml_insights_handlers;
mod ml_retrain_fine_tuning;
mod ml_retrain_handler;
mod ml_retrain_job;
mod news_context_handler;
mod news_fear_greed;
mod news_handlers;
mod news_lus_handlers;
mod ollama_ajustements_handler;
mod ollama_chart_handler;
mod ollama_chat_handler;
mod ollama_handlers;
mod ollama_signal_ia_handler;
mod ollama_types;
mod patterns_echec_job;
mod pip_updater;
mod prealerte_handlers;
mod presse_handlers;
mod prealerte_worker;
mod prix_handlers;
mod prix_stream;
mod prix_utils;
mod prompts_handler;
mod rockets_analyse;
mod rockets_analyse_handler;
mod rockets_calibration;
mod rockets_handlers;
mod rockets_ml_handlers;
mod rockets_prix;
mod rockets_sauvegarder;
mod rockets_sauvegarder_feedbacks;
mod rockets_scan;
mod rockets_suivi;
mod rockets_suivi_worker;
mod retention_job;
mod runtime_handlers;
mod runtime_tick;
mod scheduler;
mod scheduler_execution;
mod sentiment_composite;
mod sentiment_filter;
mod sentiment_handlers;
mod signal_engine;
mod signal_engine_analyse;
mod signal_engine_asset;
mod signal_filtre;
mod signaux_handlers;
mod smc_analyse_handler;
mod smc_boucle;
mod smc_calibration_job;
mod smc_categorisation;
mod smc_feedback_db;
mod smc_feedback_job;
mod smc_handlers;
mod smc_monitoring_handlers;
mod smc_signal_ollama;
mod smc_v12_collect;
mod smc_v12_handlers;
mod smc_v12_out;
mod state;
mod straddle_boucle;
mod straddle_boucle_analyse;
mod straddle_boucle_signal;
mod straddle_calibration;
mod straddle_categorisation;
mod straddle_dev_handlers;
mod straddle_feedback_job;
mod straddle_handlers;
mod straddle_machine_etats;
mod straddle_ml_gate;
mod straddle_ml_handlers;
mod straddle_moniteur_position;
mod straddle_monitoring_handlers;
mod straddle_precision_handler;
mod straddle_prompt;
mod straddle_scan_pics;
mod straddle_score_regle;
mod straddle_signal_feedback;
mod straddle_signal_handler;
mod straddle_signal_ollama;
mod straddle_types;
mod straddle_utils;
mod strategies_params_handlers;
mod tendance_handlers;
mod utils;
mod volatility_handlers;
mod worker_handlers;
mod ws_handlers;

mod routes;
mod routes_prealerte;
mod routes_ml;
mod routes_rockets;

use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_filename("telegram.env").ok();
    dotenvy::dotenv().ok();

    let env_filter = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    tracing::info!("🚀 Native Trading AI Backend starting...");

    let app_state = match AppState::new().await {
        Ok(state) => web::Data::new(state),
        Err(e) => {
            tracing::error!("Échec initialisation état applicatif: {}", e);
            return Err(std::io::Error::other(e.to_string()));
        }
    };

    tracing::info!("🌐 Server running on http://0.0.0.0:8080");

    // Suivi Rockets : suspendu avec le générateur (aucun signal ouvert).
    // let pool_rockets = app_state.db.pool().clone();
    // tokio::spawn(rockets_suivi::demarrer_worker_suivi(pool_rockets));
    let _ = rockets_suivi::demarrer_worker_suivi;

    // ── ROCKETS SUSPENDU (décision propriétaire 2026-08-15) ──────────────────
    // Générateur de l'ancien système + consommateur des modèles ML purgés.
    // Retour en phase 3 comme plugin du runtime (gate 3).
    // NB : suspension deux fois manquée silencieusement (15/08) — l'instance
    // périmée a généré un signal BOME non sollicité avant correction.
    // let pool_scan = app_state.db.pool().clone();
    // let signal_engine_rockets = app_state.signal_engine.clone();
    // let pipeline_ml_rockets = app_state.pipeline_ml.clone();
    // tokio::spawn(rockets_scan::demarrer_worker_scan(
    //     pool_scan,
    //     signal_engine_rockets,
    //     pipeline_ml_rockets,
    // ));
    tracing::warn!("🛑 Worker Rockets scan SUSPENDU — retour prévu en phase 3 (plugin runtime)");
    let _ = rockets_scan::demarrer_worker_scan;

    // Analyse hebdo LLM Rockets suspendue (consommateur Ollama).
    // tokio::spawn(rockets_analyse_handler::demarrer_worker_analyse(pool_analyse));
    let _ = rockets_analyse_handler::demarrer_worker_analyse;

    // Suivi des signaux de l'ancien système : suspendu avec ses générateurs
    // (plus aucun signal ouvert à suivre).
    // tokio::spawn(signaux_handlers::demarrer_worker_suivi_signaux(pool_signaux));
    let _ = signaux_handlers::demarrer_worker_suivi_signaux;

    // Analyse hebdo LLM SMC suspendue (chemin B éteint — plus de matière).
    // tokio::spawn(smc_analyse_handler::demarrer_worker_analyse_smc(app_state.db.clone()));
    let _ = smc_analyse_handler::demarrer_worker_analyse_smc;

    // ── Boucles automatiques ─────────────────────────────────────────────────
    // Rappel : SMC + Straddle + surveillance ML sont DÉJÀ démarrés par
    // AppState::new() (voir state.rs). Ne pas les relancer ici (sinon
    // double-spawn → charge doublée + races). Garde idempotence dans chaque
    // demarrer_* au cas où.

    let pool_telegram = app_state.db.pool().clone();
    tokio::spawn(notifications::telegram_worker::demarrer_worker_telegram(pool_telegram));

    // ── Runtime tick (Phase 1 ROADMAP — cœur temps réel) ────────────────────
    // Consomme les klines Bybit (formation + confirmations) en mémoire :
    // agrégation bougie par bougie, évaluation intrabar des moteurs (à partir
    // de la phase 2), publication des clôtures. Zéro moteur en phase 1.
    // Démarre lui-même le worker Bybit WS qui l'alimente.
    let _poignees_runtime = runtime_tick::demarrer_runtime_tick(app_state.db.clone());

    // ── Pré-alertes SUSPENDUES (décision propriétaire 2026-08-15) ────────────
    // Elles étaient générées par l'ANCIEN système (scorer SMC + ATR Straddle
    // sur bougies clôturées, cycle 5 min) et envoyées par Telegram — 145 le
    // 15/08 malgré les suspensions. Plus aucune pré-alerte jusqu'à la bascule
    // 2.8 : les seules notifications à venir seront les signaux v12 VALIDÉS.
    // prealerte_worker::demarrer_worker_prealerte(app_state.db.clone());
    tracing::warn!("🛑 Worker pré-alertes SUSPENDU (ancien système — alimentait Telegram)");
    let _ = prealerte_worker::demarrer_worker_prealerte;

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
            .app_data(web::JsonConfig::default().limit(20_971_520)) // 20 MB payload limit for base64 images
            .app_data(app_state.clone())
            .wrap(cors)
            .configure(routes::configurer)
    })
    .keep_alive(std::time::Duration::from_secs(310))
    .client_request_timeout(std::time::Duration::from_secs(310))
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
