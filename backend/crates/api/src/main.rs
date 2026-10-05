use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer};

mod analyses;
mod journal_bord;
mod analyses_ia;
mod analyses_smc;
mod asset_params_handlers;
mod alertes_prix;
mod creneaux_perimetre;
mod creneaux_job;
mod evenements;
mod sante_moteurs;
mod evenements_matrice;
mod evenements_armement;
mod evenements_armement_http;
#[cfg(test)]
mod kdj_diagnostic;
mod assets_handlers;
mod calendar_handlers;
mod config_handlers;
mod data_handlers;
mod handlers;
mod http_client;
mod indicators_handlers;
mod indicators_types;
mod ml_collecte;
mod ml_insights_handlers;
mod ml_monitoring;
mod ml_retrain_fine_tuning;
mod ml_retrain_handler;
mod ml_retrain_job;
mod news_handlers;
mod ollama_chat_handler;
mod ollama_handlers;
mod ollama_types;
mod pip_updater;
mod presse_handlers;
mod presse_notation;
#[cfg(test)]
mod tests_flux_critiques;
mod whale_watching;
mod whale_labo;
mod prix_handlers;
mod prix_stream;
mod prix_utils;
mod prompts_handler;
mod rockets_handlers;
mod rockets_ml_handlers;
mod retention_job;
mod runtime_handlers;
mod runtime_replay;
mod runtime_perimetre;
mod runtime_amorces;
mod runtime_tick;
mod scheduler_execution;
mod sentiment_bandeau;
mod sentiment_composite;
mod sentiment_filter;
mod sentiment_handlers;
mod signaux_handlers;
mod straddle_agenda;
mod straddle_analyste;
mod straddle_atr;
mod rockets_actions_backfill;
mod rockets_actions_news;
mod rockets_actions_scanner;
mod rockets_verticale;
mod rockets_gestion;
mod rockets_cloture;
mod rockets_ia;
mod rockets_unlocks;
mod mt5_collecteur;
mod mt5_etat_historique;
mod setups_formation;
mod kdj_handlers;
mod latents;
mod routes_kdj;
mod smc_scanner;
mod capital_simule;
mod reglages_smc;
mod smc_pondere;
mod simulation;
mod simulation_balayages;
mod simulation_kdj;
mod rattrapage_smc;
mod smc_rejeu;
mod straddle_rejeu;
mod registre_strategies;
mod signaux_officiels;
mod smc_handlers;
mod smc_monitoring_handlers;
mod smc_v12_collect;
mod smc_v12_handlers;
mod smc_v12_out;
mod state;
mod straddle_ml_handlers;
mod straddle_types;
mod strategies_params_handlers;
mod tendance_handlers;
mod utils;
mod volatility_handlers;
mod worker_handlers;
mod ws_handlers;

mod routes;
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

    // ── ROCKETS SUSPENDU (décision propriétaire 2026-08-15) ──────────────────
    // Générateur de l'ancien système + consommateur des modèles ML purgés.
    // Retour en phase 3 comme plugin du runtime (gate 3).
    // NB : suspension deux fois manquée silencieusement (15/08) — l'instance
    // périmée a généré un signal BOME non sollicité avant correction.
    // let pool_scan = app_state.db.pool().clone();
    // let signal_engine_rockets = app_state.signal_engine.clone();
    // let pipeline_ml_rockets = app_state.pipeline_ml.clone();
    tracing::warn!("🛑 Worker Rockets scan SUSPENDU — retour prévu en phase 3 (plugin runtime)");

    // Analyse hebdo LLM Rockets suspendue (consommateur Ollama).

    // Suivi des signaux de l'ancien système : suspendu avec ses générateurs
    // (plus aucun signal ouvert à suivre).


    // ── Boucles automatiques ─────────────────────────────────────────────────
    // Rappel : SMC + Straddle + surveillance ML sont DÉJÀ démarrés par
    // AppState::new() (voir state.rs). Ne pas les relancer ici (sinon
    // double-spawn → charge doublée + races). Garde idempotence dans chaque
    // demarrer_* au cas où.

    // ── Vieux worker Telegram ÉTEINT (audit étape 2) ────────────────────────
    // Il renvoyait CHAQUE signal de la table dans l'ancien format, en double
    // du writer officiel et sans consulter le registre (état/son). Le seul
    // émetteur Telegram est désormais signaux_officiels (maquettes validées).

    // ── Runtime tick (Phase 1 ROADMAP — cœur temps réel) ────────────────────
    // Consomme les klines Bybit (formation + confirmations) en mémoire :
    // agrégation bougie par bougie, évaluation intrabar des moteurs (à partir
    // de la phase 2), publication des clôtures. Zéro moteur en phase 1.
    // Démarre lui-même le worker Bybit WS qui l'alimente.
    let poignees_runtime = runtime_tick::demarrer_runtime_tick(app_state.db.clone(), app_state.whale.clone());
    // Étape 5 — verticale Rockets : scanner D1 + gestion (bus signaux).
    rockets_verticale::demarrer(app_state.db.clone(), poignees_runtime.bus_signaux.clone());
    // Étape A2 : backfill hiérarchisé actions US (quota Tiingo : 500
    // symboles uniques/mois, file = prioritaires → narratifs → reste).
    tokio::spawn(rockets_actions_backfill::boucle_backfill(app_state.db.clone()));
    // Périmètre actif plafonné (450) par liquidité + narratif — recalcul
    // quotidien : les bougies de la nuit requalifient l'univers.
    tokio::spawn(rockets_actions_backfill::boucle_recalcul_univers(app_state.db.clone()));
    // §11 étape 1 : boucle ML v2 — rattrapage des clôtures passées en
    // samples (la collecte continue vit dans fermer_signal_par_cle).
    tokio::spawn(ml_collecte::boucle_rattrapage(app_state.db.clone()));
    // §16 : agenda intelligent straddle — propositions IA fraîches au matin.
    tokio::spawn(evenements_armement::boucle_validation(app_state.db.clone()));
    // Étape 1-bis (incident 05/10 « tout à 0 ») : préchauffage des caches
    // froids en tâche de fond DÈS le boot — les deux gros calculs (patterns
    // 24 mois, matrice événements) scannaient des dizaines de millions de
    // bougies pendant que le dashboard chargeait : tous les fetchs
    // patientaient derrière. À l'ouverture de la fenêtre : cache chaud.
    {
        let db = app_state.db.clone();
        tokio::spawn(async move {
            let debut = std::time::Instant::now();
            let n = evenements_matrice::prechauffer_matrice(&db).await["evenements"].as_array().map(|a| a.len()).unwrap_or(0);
            tracing::info!("🔥 Préchauffage matrice événements : {n} événements en {:?}", debut.elapsed());
        });
    }
    {
        let db = app_state.db.clone();
        tokio::spawn(async move {
            let debut = std::time::Instant::now();
            let n = volatility_handlers::prechauffer_patterns_jour(&db).await["assets"].as_array().map(|a| a.len()).unwrap_or(0);
            tracing::info!("🔥 Préchauffage patterns-jour : {n} asset(s) en {:?}", debut.elapsed());
        });
    }
    tokio::spawn(rockets_unlocks::boucle(app_state.db.clone()));
    tokio::spawn(straddle_analyste::assurer_cache(app_state.db.clone()));
    // Tâche 6.4 audit : notation LLM des articles restés à 0 (backlog au
    // boot puis balayage toutes les 6 h) — scores bornés à 1 pour ne jamais
    // re-boucler sur un article déjà noté.
    tokio::spawn(presse_notation::boucle(app_state.db.clone()));
    // Sentiment haussier/neutre/baissier automatique (26/09) — le Presse IA
    // du bandeau ne dépend plus des ouvertures d'articles.
    tokio::spawn(presse_notation::boucle_sentiment(app_state.db.clone()));

    // Whale watching (2.2) : rafraîchit les z-scores volume des couples SMC.
    {
        let couples: Vec<(common::Asset, common::Timeframe)> = vec![
            ("BTC", "M5"), ("ETH", "M5"), ("SOL", "M5"), ("XRP", "M5"),
            ("DOGE", "M5"), ("BNB", "M5"), ("ADA", "M5"), ("AVAX", "M5"),
            ("LINK", "M5"), ("LTC", "M5"), ("DOT", "M5"),
            ("XAUUSD", "M5"), ("XAGUSD", "M5"), ("DAX", "M5"),
            ("NAS100", "M5"), ("SP500", "M5"),
            ("BTC", "M15"), ("ETH", "M15"), ("XAUUSD", "M15"), ("DAX", "M15"),
        ]
        .into_iter()
        .filter_map(|(a, t)| {
            let asset = common::Asset::nouveau(a);
            let tf = common::Timeframe::try_from(t).ok()?;
            Some((asset, tf))
        })
        .collect();
        // Whale watching (2.2) : partage l'Arc<RwLock> du state entre
        // la boucle de refresh et l'endpoint /api/whale.
        let whale_db = app_state.db.clone();
        whale_watching::demarrer_boucle_whale(whale_db, app_state.whale.clone(), couples);
    }
    // Entraînement ML automatique : hebdomadaire (dernier > 7 j) — le
    // bouton manuel a été retiré le 23/09 (décision propriétaire).
    tokio::spawn(ml_retrain_handler::boucle_automatique(app_state.clone()));
    // Étape C : scanner actions quotidien — Observation silencieuse.
    tokio::spawn(rockets_actions_scanner::boucle_scanner_actions(app_state.db.clone(), poignees_runtime.bus_signaux.clone()));

    // ── Pré-alertes SUPPRIMÉES (nettoyage code mort, décision 2026-08-15) ──────
    // L'ancien worker (scorer SMC + ATR Straddle sur bougies clôturées) alimentait
    // Telegram en double des signaux officiels. Les seules notifications sont les
    // signaux v12 VALIDÉS (aucun endpoint de lecture ne subsiste).
    tracing::warn!("🛑 Worker pré-alertes SUPPRIMÉ (ancien système — alimentait Telegram en double)");

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
mod yahoo_quotes;
