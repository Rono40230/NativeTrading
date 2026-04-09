//! Job de réconciliation des signaux Straddle ouverts.
//!
//! Tourne toutes les 5 minutes. Pour chaque signal Straddle sans verdict :
//!   1. Charge les bougies depuis la création du signal
//!   2. Rejoue toutes les bougies via machine à états par jambe (LONG + SHORT indépendants)
//!   3. Sauvegarde l'état intermédiaire (sl_*_effectif, tps_*_atteints) pour le frontend
//!   4. Clôture uniquement sur SL final ou TP3 de l'une ou l'autre jambe
//!   5. Expire automatiquement les signaux ouverts depuis plus de 24h
use chrono::Utc;
use common::{Asset, Timeframe};
use db::{strategies_params, Database};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub(crate) use crate::straddle_machine_etats::jouer_machine_etats;

/// Horizon d'expiration d'un signal Straddle sans verdict (en secondes).
const HORIZON_EXPIRE_SEC: i64 = 24 * 3600;

// ── Signal Straddle ouvert ────────────────────────────────────────────────────

struct SignalStraddleOuvert {
    id: String,
    asset: String,
    timeframe: String,
    prix_entree: f64,
    score: f64,
    stop_loss: f64,     // SL jambe LONG d'origine (< prix_entree)
    tp_long: Vec<f64>,  // [tp1, tp2, tp3] long (> prix_entree)
    sl_short: f64,      // SL jambe SHORT d'origine (> prix_entree)
    tp_short: Vec<f64>, // [tp1, tp2, tp3] short (< prix_entree)
    cree_le: i64,
}

// ── Point d'entrée public ─────────────────────────────────────────────────────

/// Démarre le job de réconciliation en background — ne bloque pas.
pub fn demarrer_job_feedback(db: Arc<Database>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(120)).await;
        loop {
            reconcilier_signaux_ouverts(&db).await;
            sleep(Duration::from_secs(5 * 60)).await;
        }
    });
    tracing::info!("📋 Job feedback Straddle démarré (réconciliation toutes les 5 min)");
}

// ── Réconciliation ────────────────────────────────────────────────────────────

async fn reconcilier_signaux_ouverts(db: &Arc<Database>) {
    let params = strategies_params::lire_straddle_params(db.pool()).await;
    let signaux = match charger_signaux_straddle_ouverts(db).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Job feedback Straddle: chargement signaux: {}", e);
            return;
        }
    };
    if signaux.is_empty() {
        return;
    }
    tracing::debug!("Job feedback Straddle: {} signaux ouverts", signaux.len());
    for s in &signaux {
        traiter_signal(db, s, params.vente_partielle).await;
    }
}

async fn traiter_signal(db: &Arc<Database>, s: &SignalStraddleOuvert, vente_partielle: bool) {
    if Utc::now().timestamp() - s.cree_le > HORIZON_EXPIRE_SEC {
        cloturer(db, s, "expire", s.prix_entree).await;
        return;
    }

    let asset = match Asset::try_from(s.asset.as_str()) {
        Ok(a) => a,
        Err(_) => return,
    };
    let tf = match Timeframe::try_from(s.timeframe.as_str()) {
        Ok(t) => t,
        Err(_) => return,
    };

    let bougies = match db.obtenir_bougies_depuis_jours(&asset, &tf, 1).await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!("Job Straddle bougies {}/{}: {}", s.asset, s.timeframe, e);
            return;
        }
    };

    let bougies_post: Vec<_> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= s.cree_le)
        .collect();

    if bougies_post.is_empty() || (s.tp_long.is_empty() && s.tp_short.is_empty()) {
        return;
    }

    // Machine à états LONG
    let long = if !s.tp_long.is_empty() {
        jouer_machine_etats(
            &bougies_post,
            s.stop_loss,
            s.prix_entree,
            &s.tp_long,
            true,
            vente_partielle,
            &s.id,
            "LONG",
        )
    } else {
        None
    };

    // Machine à états SHORT
    let short = if !s.tp_short.is_empty() {
        jouer_machine_etats(
            &bougies_post,
            s.sl_short,
            s.prix_entree,
            &s.tp_short,
            false,
            vente_partielle,
            &s.id,
            "SHORT",
        )
    } else {
        None
    };

    // Sauvegarder l'état intermédiaire si une transition a eu lieu
    let sl_long_save = long.as_ref().map(|e| e.sl_courant).unwrap_or(s.stop_loss);
    let sl_short_save = short.as_ref().map(|e| e.sl_courant).unwrap_or(s.sl_short);
    let tps_long_save: Vec<&str> = long
        .as_ref()
        .map(|e| e.tps_done.clone())
        .unwrap_or_default();
    let tps_short_save: Vec<&str> = short
        .as_ref()
        .map(|e| e.tps_done.clone())
        .unwrap_or_default();
    let any_change = long.as_ref().map(|e| e.etat_change).unwrap_or(false)
        || short.as_ref().map(|e| e.etat_change).unwrap_or(false);

    // Chercher un verdict terminal (SL ou TP3 d'une jambe)
    let verdict_long = long.as_ref().and_then(|e| e.verdict);
    let verdict_short = short.as_ref().and_then(|e| e.verdict);

    let verdict_final = verdict_long.or(verdict_short);

    if verdict_final.is_none() && any_change {
        if let Err(e) = db::signaux::maj_suivi_progressif_straddle(
            db.pool(),
            &s.id,
            sl_long_save,
            sl_short_save,
            &tps_long_save,
            &tps_short_save,
        )
        .await
        {
            tracing::warn!("Job Straddle maj suivi {}: {}", s.id, e);
        }
    }

    if let Some((verdict, prix)) = verdict_final {
        cloturer(db, s, verdict, prix).await;
    }
}

async fn cloturer(db: &Arc<Database>, s: &SignalStraddleOuvert, verdict: &str, prix_verdict: f64) {
    if let Err(e) = db::signaux::maj_verdict(db.pool(), &s.id, verdict, prix_verdict).await {
        tracing::warn!("Job feedback Straddle maj_verdict {}: {}", s.id, e);
    }
    let risque = (s.prix_entree - s.stop_loss).abs().max(f64::EPSILON);
    if let Err(e) = db::straddle_feedback::maj_feedback_verdict(
        db.pool(),
        &s.id,
        verdict,
        s.prix_entree,
        prix_verdict,
        risque,
        s.cree_le,
    )
    .await
    {
        tracing::warn!("Job feedback Straddle maj_feedback {}: {}", s.id, e);
    }
    let rr = if matches!(verdict, "sl" | "invalide" | "expire") {
        -1.0_f64
    } else {
        (prix_verdict - s.prix_entree).abs() / risque
    };
    db::ml_samples::sauvegarder_sample(db.pool(), &db::ml_samples::MlSample {
        strategie:   "STRADDLE".to_string(),
        asset:       s.asset.clone(),
        timeframe:   s.timeframe.clone(),
        direction:   "STRADDLE".to_string(),
        prix_entree: s.prix_entree,
        prix_sortie: prix_verdict,
        stop_loss:   s.stop_loss,
        outcome:     verdict.to_string(),
        rr_realise:  Some(rr),
    }).await.ok();
    tracing::info!(
        "📋 Straddle clôturé {} {}/{} → {} @ {:.5} (score {:.0})",
        s.id,
        s.asset,
        s.timeframe,
        verdict,
        prix_verdict,
        s.score,
    );
}

// ── Requête dédiée signaux Straddle ouverts ───────────────────────────────────

async fn charger_signaux_straddle_ouverts(
    db: &Arc<Database>,
) -> common::Result<Vec<SignalStraddleOuvert>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, asset, timeframe, prix_entree, score,
                stop_loss, take_profit, sl_short, take_profit_short, cree_le
         FROM signaux
         WHERE statut = 'Actif' AND strategie = 'Straddle'
         ORDER BY cree_le ASC",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .filter_map(|r| {
            let tp_long: Vec<f64> =
                serde_json::from_str(r.get::<&str, _>("take_profit")).unwrap_or_default();
            let tp_short: Vec<f64> =
                serde_json::from_str(r.get::<&str, _>("take_profit_short")).unwrap_or_default();
            let sl_short: f64 = r.get::<Option<f64>, _>("sl_short").unwrap_or(0.0);
            if tp_long.is_empty() && tp_short.is_empty() {
                return None;
            }
            Some(SignalStraddleOuvert {
                id: r.get("id"),
                asset: r.get("asset"),
                timeframe: r.get("timeframe"),
                prix_entree: r.get("prix_entree"),
                score: r.get("score"),
                stop_loss: r.get("stop_loss"),
                tp_long,
                sl_short,
                tp_short,
                cree_le: r.get("cree_le"),
            })
        })
        .collect())
}

#[cfg(test)]
#[path = "straddle_feedback_job_tests.rs"]
mod tests;
