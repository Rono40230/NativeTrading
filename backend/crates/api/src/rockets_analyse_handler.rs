use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use std::time::Duration;

use crate::state::AppState;
use db::{rockets, rockets_config};

// ── Analyse des opportunités visibles dans l'UI ───────────────────────────────

#[derive(Deserialize)]
pub struct SignalResume {
    pub ticker: String,
    pub phase: String,
    pub change1h: f64,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
    pub score: i64,
    pub entree_limite: f64,
    pub entree_stop: f64,
    pub niveau_invalidation: f64,
    pub type_entree_rec: String,
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub tp3_trigger: f64,
    pub trailing_coeff: f64,
}

/// POST /api/rockets/analyse-opportunites — analyse IA des top signaux visibles dans l'UI.
pub async fn analyser_opportunites(body: web::Json<Vec<SignalResume>>) -> impl Responder {
    if body.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Aucun signal fourni" }));
    }

    let liste = body
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}. {} — Phase: {} | Variation 1h: {}% | Vol×: {:.2} | ATR ratio: {:.2} | RSI: {:.1} | \
                 E.Limite: {}$ | E.Stop: {}$ | Invalidation: {}$ | Entrée idéale: {} | \
                 SL: {}$ | TP1: {}$ | TP2: {}$ | Trailing trigger: {}$ | Coef trailing: {:.1}× | Score: {}/100",
                i + 1,
                s.ticker,
                s.phase,
                if s.change1h >= 0.0 { format!("+{:.2}", s.change1h) } else { format!("{:.2}", s.change1h) },
                s.ratio_volume,
                s.atr_ratio,
                s.rsi,
                s.entree_limite,
                s.entree_stop,
                s.niveau_invalidation,
                s.type_entree_rec,
                s.sl,
                s.tp1,
                s.tp2,
                s.tp3_trigger,
                s.trailing_coeff,
                s.score,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5vl:7b".to_string());
    let system = crate::prompts_handler::prompt_effectif("rockets_opportunites");
    let messages = vec![("user".to_string(), format!("Signaux Rocket à analyser :\n\n{}", liste))];

    match crate::ollama::interroger_chat_modele_avec_systeme(&messages, &modele, &system).await {
        Ok(texte) => HttpResponse::Ok().json(serde_json::json!({ "texte": texte })),
        Err(e) => {
            tracing::warn!("Analyse opportunités LLM: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

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
            Ok(a) => tracing::debug!(
                "Worker analyse LLM rockets terminé ({} trades analysés)",
                a.nb_trades
            ),
            Err(e) => tracing::warn!("Worker analyse LLM rockets: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(INTERVALLE_SEMAINES)).await;
    }
}
