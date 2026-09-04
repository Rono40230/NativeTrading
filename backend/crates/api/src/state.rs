#![allow(unused_variables, dead_code)]
use anyhow::Result;
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::RwLock;

use smc::v12::sentiment::SentimentScore;

pub struct AppState {
    pub db: Arc<Database>,
    pub pipeline_ml: Arc<RwLock<PipelineML>>,
    /// État du job de réentraînement incrémental (Phase 8.4)
    pub retrain_state: Arc<tokio::sync::RwLock<crate::ml_retrain_handler::RetainState>>,
    /// Moteur de génération automatique de signaux SMC
    /// F&G (ex-SignalEngine — phase 2.8 : atomiques nus).
    pub fg_valeur: std::sync::atomic::AtomicU64,
    /// Cache Fear & Greed Index (TTL 1h) — (Instant du fetch, données JSON)
    pub fear_greed_cache: Arc<tokio::sync::RwLock<Option<(std::time::Instant, serde_json::Value)>>>,
    /// Sentiment composite 0-100 par classe (refresh 30 min par le worker).
    pub sentiment: Arc<RwLock<Option<SentimentScore>>>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let db_path =
            std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());

        // Créer le dossier parent si nécessaire (ex: data/)
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let db = Database::new(&db_path).await?;
        db.run_migrations().await?;
        tracing::info!("Base de données initialisée + migrations: {}", db_path);

        // Pipeline ML — tente de recharger les modèles persistés
        let mut pipeline_ml = PipelineML::new();
        let modele_deja_charge = match pipeline_ml.charger_depuis_disque() {
            Ok(true) => {
                tracing::info!("Pipeline ML rechargé depuis disque");
                true
            }
            Ok(false) => {
                tracing::info!(
                    "Pipeline ML initialisé (pas de modèle persisté — entraînement immédiat prévu)"
                );
                false
            }
            Err(e) => {
                tracing::warn!("Impossible de charger le pipeline ML: {}", e);
                false
            }
        };

        let db = Arc::new(db);
        let pipeline_ml = Arc::new(RwLock::new(pipeline_ml));
        // Le SignalEngine reste instancié : c'est le canal broadcast que
        // Straddle/Rockets utilisent pour publier vers le WS frontend.
        let fg_valeur = std::sync::atomic::AtomicU64::new(0);
        // Cache Fear & Greed (TTL 1h) — partagé entre l'endpoint et le worker sentiment.
        let fear_greed_cache: Arc<
            tokio::sync::RwLock<Option<(std::time::Instant, serde_json::Value)>>,
        > = Arc::new(tokio::sync::RwLock::new(None));

        // ── Générateurs SMC timer ÉTEINTS (décision Gate 0 n°2, 2026-08-15) ──
        // Les chemins A (Signal Engine 5 min) et B (boucle SMC 15 min) lisaient
        // des bougies fermées en DB avec 15-45 min de retard — architecture
        // remplacée par le runtime tick + moteur v12 en shadow (runtime_tick).
        // ⚠️ Ne pas réactiver : deux générateurs SMC simultanés pollueraient
        // DB et Telegram pendant le test de vérité (Gate 2).
        tracing::info!("🛑 Ancien chemin SMC timer SUPPRIMÉ (phase 2.8) — signaux officiels = runtime v12");

        // ── ML SUSPENDU (décision propriétaire 2026-08-15) ─────────────────────
        // Les modèles avaient été entraînés sur les signaux de l'ancien système
        // (clôture-bougie, 15-45 min de retard) : données invalides. Purge
        // effectuée (tables d'apprentissage vides, modèles archivés dans
        // data/backups/modeles_2026-08-15/). Réactivation après les gates 2-3,
        // réentraînement sur signaux validés uniquement.
        // (scheduler + surveillance ML supprimés — nettoyage code mort)
        tracing::warn!("🛑 ML SUSPENDU — modèles purgés (entraînés sur l'ancien système), aucun réentraînement avant les gates 2-3");
        let _ = (pipeline_ml.clone(), modele_deja_charge);

        // ── STRADDLE SUSPENDU (décision propriétaire 2026-08-15) ────────────────
        // Générateur de l'ancien système (timer + bougies fermées + gates ML
        // pollués). Redévient actif en phase 3 comme plugin du runtime, après
        // validation de fidélité (gate 3).
        // crate::straddle_boucle::demarrer_boucle_straddle(...);
        // crate::straddle_scan_pics::demarrer_scan_pics(...);
        tracing::warn!("🛑 Boucles Straddle SUSPENDUES — retour prévu en phase 3 (plugin runtime)");

        // Refresh calendrier économique en arrière-plan (toutes les 30 min)
        crate::calendar_handlers::demarrer_refresh_calendrier_job(db.clone());

        // tables purgées, recalibration après gates 2-3.
        // (jobs straddle_feedback_job / straddle_moniteur_position / calibrations
        //  supprimés — nettoyage code mort, l'ancien système ne reviendra pas)

        // Boucle analyse SMC Directionnel (toutes les 15 min) — ÉTEINTE,
        // voir décision Gate 0 n°2 ci-dessus (remplacée par le runtime tick).
        let sentiment_slot: Arc<RwLock<Option<SentimentScore>>> =
            Arc::new(RwLock::new(None));

        // Worker sentiment composite (cycle 30 min) — alimente le post-filtre directionnel
        crate::sentiment_composite::demarrer_worker_sentiment(
            db.clone(),
            sentiment_slot.clone(),
            fear_greed_cache.clone(),
        );
        tracing::info!("📊 Sentiment composite activé (worker 30 min)");

        // Étape 2 — registre des stratégies : enregistre les manifestes
        // code absents de la table (INSERT OR IGNORE, réglages DB préservés).
        crate::registre_strategies::amorcer_registre(&db).await;

        // (workers relics smc_feedback_job / smc_calibration_job supprimés —
        // le moteur v12 lit sa calibration Pine figée)

        // Job quotidien de rétention des données (piloté par la config utilisateur)
        crate::retention_job::demarrer_job_retention(db.clone());

        // Job quotidien de mise à jour des valeur_pips (paires JPY)
        crate::pip_updater::demarrer_pip_updater(db.clone());

        // Créneaux de volatilité par asset (boot + cycle 24h — dashboard)
        crate::creneaux_job::demarrer(db.clone());

        Ok(Self {
            db,
            pipeline_ml,
            retrain_state: Arc::new(tokio::sync::RwLock::new(
                crate::ml_retrain_handler::RetainState::default(),
            )),
            fg_valeur,
            fear_greed_cache,
            sentiment: sentiment_slot,
        })
    }
}
