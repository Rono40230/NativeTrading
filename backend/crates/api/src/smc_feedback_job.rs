//! Job de réconciliation des signaux SMC Directionnel ouverts.
//!
//! Tourne toutes les 5 minutes. Pour chaque signal SMC Directionnel sans verdict :
//!   1. Charge les bougies depuis la création du signal
//!   2. Rejoue toutes les bougies via machine à états (SL progressif + TPs partiels)
//!   3. Sauvegarde l'état intermédiaire (sl_effectif, tps_atteints) pour le frontend
//!   4. Clôture uniquement sur SL final ou TP3
//!   5. Expire automatiquement les signaux ouverts depuis plus de 48h
use chrono::Utc;
use common::{Asset, Timeframe};
use db::{strategies_params, Database};
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
    direction: String,     // "Long" | "Short"
    prix_entree: f64,
    stop_loss: f64,        // SL d'origine (immuable)
    take_profit: Vec<f64>, // [tp1, tp2, tp3]
    cree_le: i64,
}

// ── Point d'entrée public ─────────────────────────────────────────────────────

/// Démarre le job de réconciliation SMC en background — ne bloque pas.
pub fn demarrer_job_feedback_smc(db: Arc<Database>) {
    tokio::spawn(async move {
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
    let params = strategies_params::lire_smc_params(db.pool()).await;
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
    tracing::debug!("Job feedback SMC: {} signaux ouverts", signaux.len());
    for s in &signaux {
        traiter_signal_smc(db, s, params.vente_partielle).await;
    }
}

async fn traiter_signal_smc(db: &Arc<Database>, s: &SignalSmcOuvert, vente_partielle: bool) {
    // Expiration automatique
    if Utc::now().timestamp() - s.cree_le > HORIZON_EXPIRE_SEC {
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

    let bougies = match db.obtenir_bougies_depuis_jours(&asset, &tf, 2).await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!("Job SMC bougies {}/{}: {}", s.asset, s.timeframe, e);
            return;
        }
    };

    let bougies_post: Vec<_> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= s.cree_le)
        .collect();

    if bougies_post.is_empty() {
        return;
    }

    let (tp1, tp2, tp3) = match s.take_profit.as_slice() {
        [a, b, c, ..] => (*a, Some(*b), Some(*c)),
        [a, b] => (*a, Some(*b), None),
        [a] => (*a, None, None),
        _ => return,
    };

    let is_long = s.direction != "Short";

    // Machine à états — rejoue depuis le début pour garantir la cohérence
    let mut sl_courant = s.stop_loss;
    let mut tps_done: Vec<&str> = Vec::with_capacity(3);
    let mut verdict_final: Option<(&str, f64)> = None;
    let mut etat_change = false;

    'boucle: for bougie in &bougies_post {
        // SL touché (direction-aware)
        let sl_touche = if is_long {
            bougie.low <= sl_courant
        } else {
            bougie.high >= sl_courant
        };
        if sl_touche {
            verdict_final = Some(("sl", sl_courant));
            break 'boucle;
        }

        // TP1 : SL → Break-Even
        if !tps_done.contains(&"tp1") {
            let tp1_touche = if is_long { bougie.high >= tp1 } else { bougie.low <= tp1 };
            if tp1_touche {
                tps_done.push("tp1");
                sl_courant = s.prix_entree;
                etat_change = true;
                if vente_partielle {
                    tracing::info!("📋 SMC {} TP1 partiel ⅓ @ {:.5}", s.id, tp1);
                } else {
                    tracing::info!("📋 SMC {} TP1 atteint, SL → BE (Option 2) @ {:.5}", s.id, tp1);
                }
            }
        }

        // TP2 : SL → TP1 (seulement après TP1)
        if let Some(tp2_val) = tp2 {
            if tps_done.contains(&"tp1") && !tps_done.contains(&"tp2") {
                let tp2_touche = if is_long { bougie.high >= tp2_val } else { bougie.low <= tp2_val };
                if tp2_touche {
                    tps_done.push("tp2");
                    sl_courant = tp1;
                    etat_change = true;
                    if vente_partielle {
                        tracing::info!("📋 SMC {} TP2 partiel ⅓ @ {:.5}", s.id, tp2_val);
                    } else {
                        tracing::info!("📋 SMC {} TP2 atteint, SL → TP1 (Option 2) @ {:.5}", s.id, tp2_val);
                    }
                }
            }
        }

        // TP3 : clôture finale (seulement après TP2)
        if let Some(tp3_val) = tp3 {
            if tps_done.contains(&"tp2") {
                let tp3_touche = if is_long { bougie.high >= tp3_val } else { bougie.low <= tp3_val };
                if tp3_touche {
                    verdict_final = Some(("tp3", tp3_val));
                    break 'boucle;
                }
            }
        }
    }

    // Sauvegarder l'état intermédiaire si des TPs ont été touchés
    if etat_change && verdict_final.is_none() {
        if let Err(e) = db::signaux::maj_suivi_progressif_smc(
            db.pool(),
            &s.id,
            sl_courant,
            &tps_done,
        )
        .await
        {
            tracing::warn!("Job SMC maj suivi {}: {}", s.id, e);
        }
    }

    if let Some((verdict, prix)) = verdict_final {
        cloturer_smc(db, s, verdict, prix).await;
    }
}

async fn cloturer_smc(db: &Arc<Database>, s: &SignalSmcOuvert, verdict: &str, prix_verdict: f64) {
    if let Err(e) = db::signaux::maj_verdict(db.pool(), &s.id, verdict, prix_verdict).await {
        tracing::warn!("Job feedback SMC maj_verdict {}: {}", s.id, e);
    }

    let atr14 = match lire_atr14_feedback(db, &s.id).await {
        Ok(v) => v,
        Err(_) => (s.prix_entree - s.stop_loss).abs().max(f64::EPSILON),
    };

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
        s.id, s.asset, s.timeframe, verdict, prix_verdict,
    );
}

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
        "SELECT id, asset, timeframe, direction, prix_entree, stop_loss, take_profit, cree_le
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
            let take_profit: Vec<f64> =
                serde_json::from_str(r.get::<&str, _>("take_profit")).unwrap_or_default();
            SignalSmcOuvert {
                id: r.get("id"),
                asset: r.get("asset"),
                timeframe: r.get("timeframe"),
                direction: r.get("direction"),
                prix_entree: r.get("prix_entree"),
                stop_loss: r.get("stop_loss"),
                take_profit,
                cree_le: r.get("cree_le"),
            }
        })
        .collect())
}
