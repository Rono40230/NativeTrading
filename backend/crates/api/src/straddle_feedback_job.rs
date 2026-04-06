//! Job de réconciliation des signaux Straddle ouverts.
//!
//! Tourne toutes les 5 minutes. Pour chaque signal Straddle sans verdict :
//!   1. Charge les bougies depuis la création du signal
//!   2. Vérifie bougie par bougie si un TP ou SL a été touché
//!   3. Met à jour `signaux` (verdict) et `straddle_feedback` (pnl_r, gagnant, ...)
//!   4. Expire automatiquement les signaux Straddle ouvert depuis plus de 24h
use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Horizon d'expiration d'un signal Straddle sans verdict (en secondes).
const HORIZON_EXPIRE_SEC: i64 = 24 * 3600;

// ── Signal Straddle ouvert (requête dédiée avec les deux jambes) ──────────────

struct SignalStraddleOuvert {
    id: String,
    asset: String,
    timeframe: String,
    prix_entree: f64,
    score: f64,         // score 0-100 (stocké × 10 dans la boucle)
    stop_loss: f64,     // SL jambe long (< prix_entree)
    tp_long: Vec<f64>,  // [tp1, tp2, tp3] long (> prix_entree)
    sl_short: f64,      // SL jambe short (> prix_entree)
    tp_short: Vec<f64>, // [tp1, tp2, tp3] short (< prix_entree)
    cree_le: i64,
}

// ── Point d'entrée public ─────────────────────────────────────────────────────

/// Démarre le job de réconciliation en background — ne bloque pas.
pub fn demarrer_job_feedback(db: Arc<Database>) {
    tokio::spawn(async move {
        // Délai initial pour laisser la boucle Straddle démarrer d'abord.
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
    let signaux = match charger_signaux_straddle_ouverts(db).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Job feedback: chargement signaux Straddle: {}", e);
            return;
        }
    };

    if signaux.is_empty() {
        return;
    }

    tracing::debug!(
        "Job feedback: {} signaux Straddle ouverts à vérifier",
        signaux.len()
    );

    for s in &signaux {
        traiter_signal(db, s).await;
    }
}

async fn traiter_signal(db: &Arc<Database>, s: &SignalStraddleOuvert) {
    let now = Utc::now().timestamp();

    // Expiration automatique
    if now - s.cree_le > HORIZON_EXPIRE_SEC {
        cloturer(db, s, "expire", s.prix_entree).await;
        return;
    }

    // Déduction de l'asset et timeframe
    let asset = match Asset::try_from(s.asset.as_str()) {
        Ok(a) => a,
        Err(_) => return,
    };
    let tf = match Timeframe::try_from(s.timeframe.as_str()) {
        Ok(t) => t,
        Err(_) => return,
    };

    // Bougies depuis la création du signal (au max 1 jour)
    let bougies = match db.obtenir_bougies_depuis_jours(&asset, &tf, 1).await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!("Job feedback bougies {}/{}: {}", s.asset, s.timeframe, e);
            return;
        }
    };

    // Ne garder que les bougies postérieures à la création du signal
    let bougies_post: Vec<_> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= s.cree_le)
        .collect();

    if bougies_post.is_empty() {
        return;
    }

    // Niveaux à surveiller (TP par ordre croissant pour jambe long)
    let tp_long_labels = ["tp1", "tp2", "tp3"];
    let tp_short_labels = ["tp1", "tp2", "tp3"];

    // Parcours chronologique des bougies
    let mut verdict_trouve: Option<(&str, f64)> = None;

    'boucle: for bougie in &bougies_post {
        // SL long touché
        if !s.tp_long.is_empty() && bougie.low <= s.stop_loss {
            verdict_trouve = Some(("sl", s.stop_loss));
            break 'boucle;
        }
        // SL short touché
        if !s.tp_short.is_empty() && bougie.high >= s.sl_short {
            verdict_trouve = Some(("sl", s.sl_short));
            break 'boucle;
        }
        // TP long du plus élevé au moins élevé (TP3 > TP2 > TP1)
        for (i, &tp) in s.tp_long.iter().enumerate().rev() {
            if bougie.high >= tp {
                verdict_trouve = Some((tp_long_labels[i], tp));
                break 'boucle;
            }
        }
        // TP short du plus bas au plus haut (TP3 < TP2 < TP1)
        for (i, &tp) in s.tp_short.iter().enumerate().rev() {
            if bougie.low <= tp {
                verdict_trouve = Some((tp_short_labels[i], tp));
                break 'boucle;
            }
        }
    }

    if let Some((verdict, prix)) = verdict_trouve {
        cloturer(db, s, verdict, prix).await;
    }
}

async fn cloturer(db: &Arc<Database>, s: &SignalStraddleOuvert, verdict: &str, prix_verdict: f64) {
    // 1. Mettre à jour la table `signaux`
    if let Err(e) = db::signaux::maj_verdict(db.pool(), &s.id, verdict, prix_verdict).await {
        tracing::warn!("Job feedback maj_verdict {}: {}", s.id, e);
    }

    // 2. Mettre à jour `straddle_feedback`
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
        tracing::warn!("Job feedback maj_feedback {}: {}", s.id, e);
    }

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
