//! Handlers HTTP Phase 8 — ML Feedback Loop.
//! GET /api/ml/feedback/stats → statistiques GLOBALES de performance par
//! stratégie, alimentées par les vraies clôtures (`ml_training_samples`).
//! (Écart 25, 13/10 : les tranches par score/kill-zone/confiance/phase et la
//! boucle d'écriture feedback ont été purgées — tables sources vides, aucun
//! écrivain depuis l'audit ML du 06/10. Les « suggestions de paramètres »
//! avaient déjà été purgées le 09/09.)
use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;

use crate::state::AppState;

/// GET /api/ml/feedback/stats
pub async fn stats_feedback(state: web::Data<AppState>) -> HttpResponse {
    let analyse = charger_analyse(state.db.pool()).await;
    HttpResponse::Ok().json(analyse)
}

// ── Helpers privés ────────────────────────────────────────────────────────────

/// Construit l'`AnalyseGlobale` en parallèle depuis les vraies clôtures.
async fn charger_analyse(pool: &SqlitePool) -> ml::feedback_analyser::AnalyseGlobale {
    let (smc_g, rkt_g, str_g) = tokio::join!(
        db::ml_feedback::stats_globales_smc(pool),
        db::ml_feedback::stats_globales_rockets(pool),
        db::ml_feedback::stats_globales_straddle(pool),
    );

    use ml::feedback_analyser::{AnalyseGlobale, RocketsAnalyse, SmcAnalyse, StatsGlobales, StraddleAnalyse};

    let sg = |g: db::ml_feedback::FeedbackGlobal| StatsGlobales {
        nb_trades: g.nb_trades,
        nb_gagnants: g.nb_gagnants,
        win_rate: g.win_rate,
        pnl_r_moyen: g.pnl_r_moyen,
    };

    AnalyseGlobale {
        smc: smc_g.ok().map(|g| SmcAnalyse { global: sg(g) }),
        rockets: rkt_g.ok().map(|g| RocketsAnalyse { global: sg(g) }),
        straddle: str_g.ok().map(|g| StraddleAnalyse { global: sg(g) }),
    }
}
