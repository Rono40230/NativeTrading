//! Job de réconciliation des signaux SMC Directionnel ouverts.
//!
//! Tourne toutes les 5 minutes. Pour chaque signal SMC Directionnel sans verdict :
//!   1. Charge les bougies depuis la création du signal
//!   2. Vérifie bougie par bougie si un TP ou SL a été touché
//!   3. Met à jour `signaux` (verdict) et `smc_feedback` (pnl_r, gagnant, ...)
//!   4. Expire automatiquement les signaux SMC ouverts depuis plus de 48h
use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Horizon d'expiration d'un signal SMC sans verdict (en secondes).
const HORIZON_EXPIRE_SEC: i64 = 48 * 3600;

// ── Signal SMC ouvert ─────────────────────────────────────────────────────────

struct SignalSmcOuvert {
    id: String,
    asset: String,
    timeframe: String,
    prix_entree: f64,
    stop_loss: f64,
    take_profit: Vec<f64>, // [tp1, tp2, tp3]
    cree_le: i64,
}

// ── Point d'entrée public ─────────────────────────────────────────────────────

/// Démarre le job de réconciliation SMC en background — ne bloque pas.
pub fn demarrer_job_feedback_smc(db: Arc<Database>) {
    tokio::spawn(async move {
        // Délai initial pour laisser la boucle SMC démarrer d'abord.
        sleep(Duration::from_secs(150)).await;
        loop {
            reconcilier_signaux_smc(&db).await;
            sleep(Duration::from_secs(5 * 60)).await;
        }
    });
    tracing::info!("📋 Job feedback SMC démarré (réconciliation toutes les 5 min)");
}

// ── Réconciliation ────────────────────────────────────────────────────────────

async fn reconcilier_signaux_smc(db: &Arc<Database>) {
    let signaux = match charger_signaux_smc_ouverts(db).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Job feedback SMC: chargement signaux: {}", e);
            return;
        }
    };

    if signaux.is_empty() {
        return;
    }

    tracing::debug!(
        "Job feedback SMC: {} signaux ouverts à vérifier",
        signaux.len()
    );

    for s in &signaux {
        traiter_signal_smc(db, s).await;
    }
}

async fn traiter_signal_smc(db: &Arc<Database>, s: &SignalSmcOuvert) {
    let now = Utc::now().timestamp();

    // Expiration automatique
    if now - s.cree_le > HORIZON_EXPIRE_SEC {
        cloturer_smc(db, s, "expire", s.prix_entree).await;
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

    // Bougies depuis la création du signal (au max 2 jours)
    let bougies = match db.obtenir_bougies_depuis_jours(&asset, &tf, 2).await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!(
                "Job feedback SMC bougies {}/{}: {}",
                s.asset,
                s.timeframe,
                e
            );
            return;
        }
    };

    // Filtrer les bougies postérieures à la création du signal
    let bougies_post: Vec<_> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= s.cree_le)
        .collect();

    if bougies_post.is_empty() {
        return;
    }

    let tp_labels = ["tp1", "tp2", "tp3"];
    let mut verdict_trouve: Option<(&str, f64)> = None;

    'boucle: for bougie in &bougies_post {
        // SL touché
        if bougie.low <= s.stop_loss {
            verdict_trouve = Some(("sl", s.stop_loss));
            break 'boucle;
        }
        // TP du plus éloigné au plus proche (TP3 > TP2 > TP1)
        for (i, &tp) in s.take_profit.iter().enumerate().rev() {
            if tp > s.prix_entree && bougie.high >= tp {
                verdict_trouve = Some((tp_labels[i], tp));
                break 'boucle;
            }
            // Direction SHORT : TP < prix_entree
            if tp < s.prix_entree && bougie.low <= tp {
                verdict_trouve = Some((tp_labels[i], tp));
                break 'boucle;
            }
        }
    }

    if let Some((verdict, prix)) = verdict_trouve {
        cloturer_smc(db, s, verdict, prix).await;
    }
}

async fn cloturer_smc(db: &Arc<Database>, s: &SignalSmcOuvert, verdict: &str, prix_verdict: f64) {
    // 1. Mettre à jour la table `signaux`
    if let Err(e) = db::signaux::maj_verdict(db.pool(), &s.id, verdict, prix_verdict).await {
        tracing::warn!("Job feedback SMC maj_verdict {}: {}", s.id, e);
    }

    // 2. Récupérer atr14 depuis smc_feedback pour calculer pnl_r
    let atr14 = match lire_atr14_feedback(db, &s.id).await {
        Ok(v) => v,
        Err(_) => (s.prix_entree - s.stop_loss).abs().max(f64::EPSILON),
    };

    // 3. Mettre à jour `smc_feedback`
    if let Err(e) = db::smc_feedback::reconcilier_feedback(
        db.pool(),
        &s.id,
        verdict,
        s.prix_entree,
        prix_verdict,
        atr14,
        s.cree_le,
    )
    .await
    {
        tracing::warn!("Job feedback SMC reconcilier {}: {}", s.id, e);
    }

    tracing::info!(
        "📋 SMC clôturé {} {}/{} → {} @ {:.5}",
        s.id,
        s.asset,
        s.timeframe,
        verdict,
        prix_verdict,
    );
}

/// Lit l'atr14 stocké dans smc_feedback pour un signal donné.
async fn lire_atr14_feedback(db: &Arc<Database>, signal_id: &str) -> common::Result<f64> {
    use sqlx::Row;
    let row = sqlx::query("SELECT atr14 FROM smc_feedback WHERE signal_id = ? LIMIT 1")
        .bind(signal_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| common::TradingError::Database(e.to_string()))?;

    match row {
        Some(r) => Ok(r.get::<f64, _>("atr14")),
        None => Err(common::TradingError::Data("atr14 introuvable".into())),
    }
}

// ── Requête signaux SMC Directionnel ouverts ──────────────────────────────────

async fn charger_signaux_smc_ouverts(db: &Arc<Database>) -> common::Result<Vec<SignalSmcOuvert>> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT id, asset, timeframe, prix_entree, stop_loss, take_profit, cree_le
         FROM signaux
         WHERE statut = 'Actif' AND strategie = 'SMC Directionnel'
         ORDER BY cree_le ASC",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| common::TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let tp_raw = r.get::<&str, _>("take_profit");
            let take_profit: Vec<f64> = serde_json::from_str(tp_raw).unwrap_or_default();
            SignalSmcOuvert {
                id: r.get("id"),
                asset: r.get("asset"),
                timeframe: r.get("timeframe"),
                prix_entree: r.get("prix_entree"),
                stop_loss: r.get("stop_loss"),
                take_profit,
                cree_le: r.get("cree_le"),
            }
        })
        .collect())
}
