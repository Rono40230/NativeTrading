use actix_web::{web, HttpResponse, Responder};
use std::time::Duration;

use crate::state::AppState;
use db::{rockets, rockets_config};

const MIN_TRADES_ANALYSE: i64 = 5;
const LIMITE_TRADES: i64 = 30;
const INTERVALLE_SEMAINES: u64 = 7 * 24 * 3600;

// ── Handler HTTP ─────────────────────────────────────────────────────────────

/// POST /api/rockets/analyse-llm — déclenche une analyse stratégique immédiate.
pub async fn lancer_analyse(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    match executer_analyse(pool).await {
        Ok(analyse) => HttpResponse::Ok().json(analyse),
        Err(e) => {
            tracing::error!("Analyse LLM rockets: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/rockets/analyse-llm — retourne la dernière analyse stockée.
pub async fn get_derniere_analyse(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    match rockets::derniere_analyse(pool).await {
        Ok(Some(a)) => HttpResponse::Ok().json(a),
        Ok(None) => HttpResponse::NoContent().finish(),
        Err(e) => {
            tracing::error!("Lecture analyse LLM: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Logique métier ────────────────────────────────────────────────────────────

async fn executer_analyse(pool: &sqlx::SqlitePool) -> anyhow::Result<db::rockets::AnalyseLlm> {
    let signaux = rockets::signaux_pour_analyse(pool, LIMITE_TRADES).await?;

    if (signaux.len() as i64) < MIN_TRADES_ANALYSE {
        anyhow::bail!(
            "Pas assez de trades clôturés ({} < {})",
            signaux.len(),
            MIN_TRADES_ANALYSE
        );
    }

    let cfg = rockets_config::lire_config(pool).await;
    let reponse = crate::ollama::rockets_analyse::analyser_strategie(&signaux, &cfg).await?;

    let recommandations_json = serde_json::to_string(&reponse.recommandations)?;
    let id = rockets::sauvegarder_analyse(
        pool,
        signaux.len() as i64,
        &reponse.synthese,
        reponse.meilleur_setup.as_deref(),
        reponse.pire_setup.as_deref(),
        &recommandations_json,
    )
    .await?;

    tracing::info!(
        "Analyse LLM rockets sauvegardée (id={}, {} trades)",
        id,
        signaux.len()
    );

    // Recharger depuis DB pour retourner l'objet complet avec cree_le
    rockets::derniere_analyse(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Analyse introuvable après insertion"))
}

// ── Worker hebdomadaire ───────────────────────────────────────────────────────

pub async fn demarrer_worker_analyse(pool: sqlx::SqlitePool) {
    // Attendre 2 minutes au démarrage pour laisser le reste s'initialiser
    tokio::time::sleep(Duration::from_secs(120)).await;

    loop {
        match executer_analyse(&pool).await {
            Ok(a) => tracing::info!(
                "Worker analyse LLM rockets terminé ({} trades analysés)",
                a.nb_trades
            ),
            Err(e) => tracing::warn!("Worker analyse LLM rockets: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(INTERVALLE_SEMAINES)).await;
    }
}
