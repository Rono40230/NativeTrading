//! Handlers HTTP Phase 8 — ML Feedback Loop.
//! GET  /api/ml/feedback/stats       → statistiques de performance par stratégie
//! GET  /api/ml/suggestions          → suggestions de paramètres + historique
//! POST /api/ml/suggestions/appliquer → applique une suggestion validée par l'utilisateur
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::state::AppState;

// ── Types requête ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AppliquerRequest {
    pub strategie: String,
    pub param_name: String,
    pub valeur_actuelle: f64,
    pub valeur_suggeree: f64,
    pub gain_winrate_estime: f64,
    pub confiance: f64,
    pub nb_samples_base: i64,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/ml/feedback/stats
pub async fn stats_feedback(state: web::Data<AppState>) -> HttpResponse {
    let analyse = charger_analyse(state.db.pool()).await;
    HttpResponse::Ok().json(analyse)
}

/// GET /api/ml/suggestions
pub async fn suggestions(state: web::Data<AppState>) -> HttpResponse {
    let pool = state.db.pool();
    let analyse = charger_analyse(pool).await;
    let params_smc = db::strategies_params::lire_smc_params(pool).await;

    let suggestions = ml::params_suggester::generer_suggestions(
        &analyse,
        params_smc.score_min,
        params_smc.kill_zone_filtre,
        params_smc.atr_sl,
    );
    let historique = db::ml_feedback::lister_suggestions(pool, 10)
        .await
        .unwrap_or_default();

    HttpResponse::Ok().json(serde_json::json!({
        "suggestions": suggestions,
        "historique":  historique,
    }))
}

/// POST /api/ml/suggestions/appliquer
pub async fn appliquer_suggestion(
    state: web::Data<AppState>,
    body: web::Json<AppliquerRequest>,
) -> HttpResponse {
    let pool = state.db.pool();
    let req = &body.0;

    // Validation basique
    if req.confiance < 0.0 || req.confiance > 1.0 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "confiance doit être entre 0.0 et 1.0" }));
    }

    // Appliquer le changement de paramètre sur la table correspondante
    let result = appliquer_param(pool, req).await;
    if let Err(e) = result {
        tracing::error!(
            "Erreur application suggestion ML {}/{}: {}",
            req.strategie,
            req.param_name,
            e
        );
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() }));
    }

    // Logger la suggestion appliquée (non-bloquant sur erreur)
    let log = db::ml_feedback::NouvelleSuggestionLog {
        strategie: &req.strategie,
        param_name: &req.param_name,
        valeur_avant: req.valeur_actuelle,
        valeur_apres: req.valeur_suggeree,
        gain_winrate_estime: req.gain_winrate_estime,
        confiance: req.confiance,
        nb_samples_base: req.nb_samples_base,
    };
    if let Err(e) = db::ml_feedback::sauvegarder_suggestion(pool, &log).await {
        tracing::warn!(
            "Log suggestion ML échoué (suggestion quand même appliquée): {}",
            e
        );
    }

    tracing::info!(
        "✅ Suggestion ML appliquée : {} {} {} → {}",
        req.strategie,
        req.param_name,
        req.valeur_actuelle,
        req.valeur_suggeree
    );
    HttpResponse::Ok().json(serde_json::json!({
        "ok":          true,
        "strategie":   req.strategie,
        "param_name":  req.param_name,
        "valeur_apres": req.valeur_suggeree,
    }))
}

// ── Helpers privés ────────────────────────────────────────────────────────────

/// Applique la modification de paramètre sur la table DB correspondante.
async fn appliquer_param(pool: &SqlitePool, req: &AppliquerRequest) -> common::Result<()> {
    match (req.strategie.as_str(), req.param_name.as_str()) {
        ("SMC", "score_min") => {
            let mut p = db::strategies_params::lire_smc_params(pool).await;
            p.score_min = req.valeur_suggeree as i64;
            db::strategies_params::sauvegarder_smc_params(pool, &p).await
        }
        ("SMC", "kill_zone_filtre") => {
            let mut p = db::strategies_params::lire_smc_params(pool).await;
            p.kill_zone_filtre = req.valeur_suggeree != 0.0;
            db::strategies_params::sauvegarder_smc_params(pool, &p).await
        }
        ("SMC", "atr_sl") => {
            let mut p = db::strategies_params::lire_smc_params(pool).await;
            p.atr_sl = req.valeur_suggeree;
            db::strategies_params::sauvegarder_smc_params(pool, &p).await
        }
        ("ROCKETS", "score_min") => {
            let mut p = db::rockets_config::lire_config(pool).await;
            p.score_min = req.valeur_suggeree as i64;
            db::rockets_config::sauvegarder_config(pool, &p).await
        }
        ("ROCKETS", "tp_multiplier") => {
            let mut p = db::rockets_config::lire_config(pool).await;
            p.sl_mult = req.valeur_suggeree - 0.5;
            db::rockets_config::sauvegarder_config(pool, &p).await
        }
        ("ROCKETS", "conviction_llm_min") => {
            tracing::info!("Application logique de conviction_llm_min bypassée techniquement (géré par phase/session via auto-ml)");
            Ok(())
        }
        ("STRADDLE", "atr_seuil") => {
            let mut p = db::strategies_params::lire_straddle_params(pool).await;
            p.atr_seuil = req.valeur_suggeree;
            db::strategies_params::sauvegarder_straddle_params(pool, &p).await
        }
        ("STRADDLE", "tp_mult_1") => {
            let mut p = db::strategies_params::lire_straddle_params(pool).await;
            p.tp_mult_1 = req.valeur_suggeree;
            db::strategies_params::sauvegarder_straddle_params(pool, &p).await
        }
        _ => Err(common::TradingError::Data(format!(
            "Paramètre {}/{} non supporté",
            req.strategie, req.param_name
        ))),
    }
}

/// Construit l'`AnalyseGlobale` en parallèle depuis les tables feedback.
async fn charger_analyse(pool: &SqlitePool) -> ml::feedback_analyser::AnalyseGlobale {
    let (smc_g, rkt_g, str_g, smc_s, smc_kz, smc_ml, rkt_phase, rkt_conv, str_cat, str_llm) =
        tokio::join!(
            db::ml_feedback::stats_globales_smc(pool),
            db::ml_feedback::stats_globales_rockets(pool),
            db::ml_feedback::stats_globales_straddle(pool),
            db::ml_feedback::stats_smc_par_score(pool),
            db::ml_feedback::stats_smc_par_kill_zone(pool),
            db::ml_feedback::stats_smc_ml_correlation(pool),
            db::ml_feedback_rockets::stats_par_phase(pool),
            db::ml_feedback_rockets::stats_conviction_llm(pool),
            db::ml_feedback_straddle::stats_par_categorie(pool),
            db::ml_feedback_straddle::stats_score_llm(pool),
        );

    use ml::feedback_analyser::{
        AnalyseGlobale, RocketsAnalyse, SmcAnalyse, StatsGlobales, StraddleAnalyse, TrancheStat,
    };

    let to_tranche = |tranche: String, nb_trades: i64, win_rate: f64| TrancheStat {
        tranche,
        nb_trades,
        win_rate,
    };
    let score_trs = |rows: Vec<db::ml_feedback::SmcScoreStats>| -> Vec<TrancheStat> {
        rows.into_iter()
            .map(|r| to_tranche(r.tranche, r.nb_trades, r.win_rate))
            .collect()
    };
    let kz_trs = |rows: Vec<db::ml_feedback::SmcSessionStats>| -> Vec<TrancheStat> {
        rows.into_iter()
            .map(|r| {
                to_tranche(
                    if r.en_kill_zone { "Kill Zone".into() } else { "Hors Kill Zone".into() },
                    r.nb_trades,
                    r.win_rate,
                )
            })
            .collect()
    };
    let ml_trs = |rows: Vec<db::ml_feedback::MlCorrelationStats>| -> Vec<TrancheStat> {
        rows.into_iter()
            .map(|r| to_tranche(r.tranche, r.nb_trades, r.win_rate))
            .collect()
    };
    let sg = |g: db::ml_feedback::FeedbackGlobal| StatsGlobales {
        nb_trades: g.nb_trades,
        nb_gagnants: g.nb_gagnants,
        win_rate: g.win_rate,
        pnl_r_moyen: g.pnl_r_moyen,
    };

    let smc = smc_g.ok().map(|g| SmcAnalyse {
        global: sg(g),
        par_score: score_trs(smc_s.unwrap_or_default()),
        par_kill_zone: kz_trs(smc_kz.unwrap_or_default()),
        ml_correlation: ml_trs(smc_ml.unwrap_or_default()),
    });

    let rockets = rkt_g.ok().map(|g| RocketsAnalyse {
        global: sg(g),
        par_phase: rkt_phase
            .unwrap_or_default()
            .into_iter()
            .map(|r| to_tranche(r.phase, r.nb_trades, r.win_rate))
            .collect(),
        conviction_llm: rkt_conv
            .unwrap_or_default()
            .into_iter()
            .map(|r| to_tranche(r.tranche, r.nb_trades, r.win_rate))
            .collect(),
    });

    let straddle = str_g.ok().map(|g| StraddleAnalyse {
        global: sg(g),
        par_categorie: str_cat
            .unwrap_or_default()
            .into_iter()
            .map(|r| to_tranche(r.categorie, r.nb_trades, r.win_rate))
            .collect(),
        score_llm: str_llm
            .unwrap_or_default()
            .into_iter()
            .map(|r| to_tranche(r.tranche, r.nb_trades, r.win_rate))
            .collect(),
    });

    AnalyseGlobale { smc, rockets, straddle }
}
