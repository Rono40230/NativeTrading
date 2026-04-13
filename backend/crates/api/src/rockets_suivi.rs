use actix_web::{web, HttpResponse, Responder};
use db::rockets;
use sqlx::Row;
use std::time::Duration;

use crate::rockets_prix::{fetch_prix, reconcilier_feedback};
use crate::state::AppState;

pub use strategies::rockets_niveaux::calculer_verdict_rocket;

// ── POST /api/rockets/sync ───────────────────────────────────────────────────

/// Force un cycle de suivi immédiat (SL/TP attente + ouverts)
pub async fn sync_verdicts(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("HTTP client: {}", e) }))
        }
    };

    let mut fermes = 0u32;
    let mut ouverts_nouveaux = 0u32;

    // Attente : SL avant entrée, ou ouvrir si prix atteint
    if let Ok(en_attente) = rockets::lister_en_attente(pool).await {
        for s in &en_attente {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            if prix <= s.stop_loss
                && rockets::maj_verdict(pool, s.id, "invalide", prix)
                    .await
                    .is_ok()
            {
                fermes += 1;
            } else if prix >= s.prix_entree && rockets::entrer_position(pool, s.id).await.is_ok() {
                ouverts_nouveaux += 1;
            }
        }
    }

    // Ouverts : SL/TP
    if let Ok(signaux) = rockets::lister_ouverts(pool).await {
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            let peak_precedent = s.prix_peak.unwrap_or(s.prix_entree);
            let peak = peak_precedent.max(prix);
            if peak > peak_precedent {
                let _ = rockets::maj_prix_peak(pool, s.id, peak).await;
            }
            match calculer_verdict_rocket(s, prix, peak, peak_precedent) {
                Some(v @ "TP1") | Some(v @ "TP2") => {
                    // Vente partielle ⅓ — position reste ouverte
                    let _ = rockets::enregistrer_tp_partiel(pool, s.id, v, prix).await;
                }
                Some(v) => {
                    if rockets::maj_verdict(pool, s.id, v, prix).await.is_ok() {
                        fermes += 1;
                    }
                }
                None => {}
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "fermes": fermes,
        "ouverts_nouveaux": ouverts_nouveaux
    }))
}

// ── POST /api/rockets/sync-feedback ──────────────────────────────────────────

/// Rétro-synchronise rockets_feedback pour tous les trades déjà clôturés
/// dont le pnl_r est encore NULL (trades fermés avant la mise en place du feedback).
#[allow(dead_code)]
pub async fn sync_feedback_historique(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    let rows = match sqlx::query(
        "SELECT rs.id, rs.ticker, rs.verdict, rs.prix_entree, rs.prix_verdict, rs.atr14, rs.cree_le
         FROM rockets_signaux rs
         LEFT JOIN rockets_feedback rf ON rf.signal_id = rs.id
         WHERE rs.verdict IS NOT NULL
           AND rs.verdict NOT IN ('invalide', 'expire')
           AND rs.prix_verdict IS NOT NULL
           AND (rf.pnl_r IS NULL OR rf.id IS NULL)
         ORDER BY rs.cree_le ASC",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let total = rows.len() as u32;
    let mut synces = 0u32;

    for row in &rows {
        let signal_id: i64 = row.get("id");
        let ticker: String = row.get("ticker");
        let verdict: String = row.get("verdict");
        let prix_entree: f64 = row.get("prix_entree");
        let prix_verdict: f64 = row.get("prix_verdict");
        let atr14: Option<f64> = row.try_get("atr14").ok().flatten();
        let cree_le: String = row.get("cree_le");

        // Créer la ligne feedback si elle n'existe pas
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO rockets_feedback
             (signal_id, ticker, phase, session_active, timestamp_signal,
              score_scan, conviction_llm, ratio_volume, atr_ratio, rsi)
             SELECT rs.id, rs.ticker, rs.phase, 'retrosynced',
                    strftime('%s', rs.cree_le),
                    COALESCE(rs.score, 0), 0,
                    COALESCE(rs.ratio_volume, 1.0),
                    COALESCE(rs.atr_ratio, 1.0),
                    COALESCE(rs.rsi, 50.0)
             FROM rockets_signaux rs WHERE rs.id = ?",
        )
        .bind(signal_id)
        .execute(pool)
        .await;

        reconcilier_feedback(
            pool,
            &ticker,
            signal_id,
            &verdict,
            prix_entree,
            prix_verdict,
            atr14,
            &cree_le,
        )
        .await;

        synces += 1;
        tracing::info!("Rétro-sync feedback Rockets {} id={} verdict={}", ticker, signal_id, verdict);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "total_eligibles": total,
        "synces": synces,
    }))
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

/// Worker lancé au démarrage : toutes les 3min, gère cycle de vie complet.
pub async fn demarrer_worker_suivi(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Worker rockets HTTP: {}", e);
            return;
        }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(3 * 60)).await;
        crate::rockets_suivi_worker::executer_cycle_suivi(&pool, &client).await;
    }
}

#[cfg(test)]
#[path = "rockets_suivi_tests.rs"]
mod tests;
