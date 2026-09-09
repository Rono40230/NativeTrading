//! Handlers HTTP Phase 8 — ML Feedback Loop.
//! GET  /api/ml/feedback/stats → statistiques de performance par stratégie
//! (alimentées par les vraies clôtures : `ml_training_samples`, `smc_feedback`).
//! Les « suggestions de paramètres » ont été purgées le 09/09 : elles
//! écrivaient dans les tables fantômes `smc_params`/`rockets_config` qu'aucun
//! moteur ne lisait — voir ROADMAP §6.
use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;

use crate::state::AppState;

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/ml/feedback/stats
pub async fn stats_feedback(state: web::Data<AppState>) -> HttpResponse {
    let analyse = charger_analyse(state.db.pool()).await;
    HttpResponse::Ok().json(analyse)
}

// ── Helpers privés ────────────────────────────────────────────────────────────

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
