//! Handler HTTP pour l'analyse LLM périodique des performances SMC Directionnel.
//! POST /api/smc/analyse-llm  — déclenche une analyse sur demande.
//! GET  /api/smc/analyse-llm  — retourne la dernière analyse stockée.
use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike, Utc, Weekday};
use sqlx::Row;
use std::sync::Arc;
use tokio::time::Duration;

use llm::smc_analyse::{analyser_strategie, SignalSMCClotl};
use crate::state::AppState;
use db::Database;

const MIN_TRADES: i64 = 5;
const LIMITE_TRADES: i64 = 100;

// ── Handlers HTTP ─────────────────────────────────────────────────────────────

/// POST /api/smc/analyse-llm — déclenche une analyse stratégique immédiate.
pub async fn lancer_analyse(state: web::Data<AppState>) -> impl Responder {
    match executer_analyse(&state.db).await {
        Ok(analyse) => HttpResponse::Ok().json(analyse),
        Err(e) => {
            tracing::error!("Analyse LLM SMC: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/smc/analyse-llm — retourne la dernière analyse stockée.
pub async fn get_derniere_analyse(state: web::Data<AppState>) -> impl Responder {
    match lire_derniere_analyse(state.db.pool()).await {
        Ok(Some(a)) => HttpResponse::Ok().json(a),
        Ok(None) => HttpResponse::NoContent().finish(),
        Err(e) => {
            tracing::error!("Lecture analyse LLM SMC: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Logique métier ────────────────────────────────────────────────────────────

async fn charger_signaux_smc(db: &Database, limite: i64) -> anyhow::Result<Vec<SignalSMCClotl>> {
    let rows = sqlx::query(
        "SELECT asset, timeframe, direction, score, statut, verdict,
                llm_conviction, cree_le
         FROM signaux
         WHERE statut IN ('Fermé', 'Actif')
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(limite)
    .fetch_all(db.pool())
    .await?;

    Ok(rows
        .iter()
        .map(|row| SignalSMCClotl {
            asset: row.get("asset"),
            timeframe: row.get("timeframe"),
            direction: row.get("direction"),
            _score: row.get("score"),
            statut: row.get("statut"),
            verdict: row.get("verdict"),
            llm_conviction: row.get("llm_conviction"),
            _cree_le: row.get("cree_le"),
        })
        .collect())
}

async fn sauvegarder_analyse(
    pool: &sqlx::SqlitePool,
    nb_trades: i64,
    synthese: &str,
    meilleur_setup: Option<&str>,
    pire_setup: Option<&str>,
    recommandations_json: &str,
) -> anyhow::Result<i64> {
    let row = sqlx::query(
        "INSERT INTO smc_analyses_llm
         (nb_trades, synthese, meilleur_setup, pire_setup, recommandations)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(nb_trades)
    .bind(synthese)
    .bind(meilleur_setup)
    .bind(pire_setup)
    .bind(recommandations_json)
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

async fn lire_derniere_analyse(
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        "SELECT id, nb_trades, synthese, meilleur_setup, pire_setup,
                recommandations, cree_le
         FROM smc_analyses_llm ORDER BY cree_le DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        let recomm_raw: String = r.get("recommandations");
        let recommandations: serde_json::Value =
            serde_json::from_str(&recomm_raw).unwrap_or(serde_json::json!([]));
        serde_json::json!({
            "id":              r.get::<i64, _>("id"),
            "nb_trades":       r.get::<i64, _>("nb_trades"),
            "synthese":        r.get::<String, _>("synthese"),
            "meilleur_setup":  r.get::<Option<String>, _>("meilleur_setup"),
            "pire_setup":      r.get::<Option<String>, _>("pire_setup"),
            "recommandations": recommandations,
            "cree_le":         r.get::<String, _>("cree_le"),
        })
    }))
}

async fn executer_analyse(
    db: &Database,
) -> anyhow::Result<serde_json::Value> {
    let signaux = charger_signaux_smc(db, LIMITE_TRADES).await?;
    let fermes = signaux.iter().filter(|s| s.statut == "Fermé").count() as i64;

    if fermes < MIN_TRADES {
        anyhow::bail!(
            "Pas assez de trades SMC clôturés ({} < {})",
            fermes,
            MIN_TRADES
        );
    }

    let reponse = analyser_strategie(&signaux, None).await?;
    let recommandations_json = serde_json::to_string(&reponse.recommandations)?;

    let id = sauvegarder_analyse(
        db.pool(),
        signaux.len() as i64,
        &reponse.synthese,
        reponse.meilleur_setup.as_deref(),
        reponse.pire_setup.as_deref(),
        &recommandations_json,
    )
    .await?;

    tracing::info!(
        "Analyse LLM SMC sauvegardée (id={}, {} signaux)",
        id,
        signaux.len()
    );

    lire_derniere_analyse(db.pool())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Analyse SMC introuvable après insertion"))
}

// ── Worker hebdomadaire planifié ─────────────────────────────────────────────

/// Attend jusqu'au prochain lundi 02h00 UTC.
fn secondes_jusqu_lundi_2h_utc() -> u64 {
    let now = Utc::now();
    // jours jusqu'au lundi (0=lundi dans notre sens)
    let jours_restants = match now.weekday() {
        Weekday::Mon => 7, // déjà lundi → prochain lundi
        Weekday::Tue => 6,
        Weekday::Wed => 5,
        Weekday::Thu => 4,
        Weekday::Fri => 3,
        Weekday::Sat => 2,
        Weekday::Sun => 1,
    };
    let secondes_journee =
        now.hour() as u64 * 3600 + now.minute() as u64 * 60 + now.second() as u64;
    // secondes restantes dans la journée + jours complets + 2h (02:00 UTC)
    let restant_journee = 86400u64.saturating_sub(secondes_journee);
    restant_journee + (jours_restants - 1) * 86400 + 2 * 3600
}

/// Worker planifié : déclenche l'analyse SMC LLM chaque lundi à 02h00 UTC.
pub async fn demarrer_worker_analyse_smc(db: Arc<Database>) {
    let attente = secondes_jusqu_lundi_2h_utc();
    tracing::info!(
        "⏰ Analyse SMC hebdo: prochain déclenchement dans {}h{}m (lundi 02:00 UTC)",
        attente / 3600,
        (attente % 3600) / 60
    );
    tokio::time::sleep(Duration::from_secs(attente)).await;

    loop {
        match executer_analyse(&db).await {
            Ok(_) => tracing::info!("✅ Analyse LLM SMC hebdo terminée"),
            Err(e) => tracing::warn!("❌ Analyse LLM SMC hebdo: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(7 * 24 * 3600)).await;
    }
}
