//! Worker de pré-alertes (cycle 5 min).
//!
//! Détecte les setups en formation avant qu'ils n'atteignent le seuil signal :
//! - SMC  : score entre seuil_prealerte (défaut 55) et seuil_signal (70)
//! - Straddle : ATR > X% du seuil OU événement macro High dans l'horizon configuré
//!
//! Les seuils sont lus depuis la table `configuration` et peuvent être mis
//! à jour automatiquement par les jobs de calibration LLM.

use common::{Asset, Timeframe};
use db::Database;
use std::sync::Arc;
use tokio::time::{interval, Duration};

const CYCLE_SEC: u64 = 300; // 5 min

// ── Point d'entrée ───────────────────────────────────────────────────────────

pub fn demarrer_worker_prealerte(db: Arc<Database>) {
    tokio::spawn(async move {
        // Délai initial pour laisser le backend se stabiliser
        tokio::time::sleep(Duration::from_secs(240)).await;
        let mut tick = interval(Duration::from_secs(CYCLE_SEC));
        loop {
            tick.tick().await;
            let params = lire_params(&db).await;
            let assets = db.lister_assets().await.unwrap_or_default();
            for asset_db in &assets {
                let asset = match Asset::try_from(asset_db.id.as_str()) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let tf_smc = if asset_db.type_asset == "crypto" {
                    Timeframe::M5
                } else {
                    Timeframe::M15
                };
                analyser_smc(&db, &asset, tf_smc, &params).await;
                analyser_straddle(&db, &asset, &params).await;
            }
        }
    });
    tracing::info!("⚠️  Worker pré-alertes démarré (cycle 5 min)");
}

// ── Paramètres lus dynamiquement ─────────────────────────────────────────────

struct Params {
    smc_seuil_prealerte: f64,
    smc_seuil_signal: f64,
    straddle_atr_pct: f64,
    straddle_horizon_min: i64,
    cooldown_straddle_min: i64,
    cooldown_smc_min: i64,
}

async fn lire_params(db: &Database) -> Params {
    let parse_f64 = |v: Option<String>, defaut: f64| -> f64 {
        v.and_then(|s| s.parse().ok()).unwrap_or(defaut)
    };

    let smc_seuil_prealerte = parse_f64(
        db.lire_config("smc_seuil_prealerte").await.ok().flatten(), 55.0,
    );
    let smc_seuil_signal = parse_f64(
        db.lire_config("smc_seuil_signal").await.ok().flatten(), 70.0,
    );
    let straddle_atr_pct = parse_f64(
        db.lire_config("straddle_atr_pct_prealerte").await.ok().flatten(), 80.0,
    );
    let straddle_horizon_min = parse_f64(
        db.lire_config("straddle_horizon_macro_min").await.ok().flatten(), 90.0,
    ) as i64;
    let cooldown_straddle_min = parse_f64(
        db.lire_config("prealerte_cooldown_straddle_min").await.ok().flatten(), 30.0,
    ) as i64;
    let cooldown_smc_min = parse_f64(
        db.lire_config("prealerte_cooldown_smc_min").await.ok().flatten(), 240.0,
    ) as i64;

    Params {
        smc_seuil_prealerte,
        smc_seuil_signal,
        straddle_atr_pct,
        straddle_horizon_min,
        cooldown_straddle_min,
        cooldown_smc_min,
    }
}

// ── Analyse SMC ──────────────────────────────────────────────────────────────

async fn analyser_smc(db: &Database, asset: &Asset, tf: Timeframe, params: &Params) {
    let bougies = match db.obtenir_bougies(asset, &tf, 200).await {
        Ok(b) if b.len() >= 30 => b,
        _ => return,
    };

    let score = match smc::scorer(&bougies) {
        Some(s) => s.total,
        None => return,
    };

    if score < params.smc_seuil_prealerte || score >= params.smc_seuil_signal {
        return; // trop bas ou déjà signal
    }

    if !cooldown_ok(db, asset.as_str(), "smc", params.cooldown_smc_min).await {
        return;
    }

    let raison = format!(
        "Setup SMC en formation — score {:.0}/100 (seuil signal : {:.0})",
        score, params.smc_seuil_signal
    );
    if let Err(e) = inserer_prealerte(db, asset.as_str(), "smc", &raison, Some(score), None, None).await {
        tracing::warn!("Pré-alerte SMC {}: {}", asset.as_str(), e);
    }
}

// ── Analyse Straddle ─────────────────────────────────────────────────────────

async fn analyser_straddle(db: &Database, asset: &Asset, params: &Params) {
    if !cooldown_ok(db, asset.as_str(), "straddle", params.cooldown_straddle_min).await {
        return;
    }

    let bougies = match db.obtenir_bougies(asset, &Timeframe::M15, 60).await {
        Ok(b) if b.len() >= 15 => b,
        _ => return,
    };

    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_valides: Vec<f64> = atr_vals.iter().copied().filter(|v| !v.is_nan()).collect();
    if atr_valides.len() < 2 {
        return;
    }
    let atr_actuel = match atr_valides.last().copied() {
        Some(v) => v,
        None => return,
    };
    let n_moy = atr_valides.len().min(14);
    let atr_moyen = atr_valides.iter().rev().take(n_moy).sum::<f64>() / n_moy as f64;
    let ratio_atr = atr_actuel / atr_moyen.max(f64::EPSILON);

    let atr_seuil: f64 = db::strategies_params::lire_straddle_params(db.pool())
        .await
        .atr_seuil;

    let seuil_prealerte = atr_seuil * (params.straddle_atr_pct / 100.0);

    // Déclencheur 1 : ATR approche du seuil
    if ratio_atr >= seuil_prealerte {
        let raison = format!(
            "Volatilité Straddle en formation — ATR ratio {:.2} ({:.0}% du seuil {:.2})",
            ratio_atr,
            params.straddle_atr_pct,
            atr_seuil
        );
        if let Err(e) =
            inserer_prealerte(db, asset.as_str(), "straddle", &raison, None, None, None).await
        {
            tracing::warn!("Pré-alerte Straddle ATR {}: {}", asset.as_str(), e);
        }
        return; // cooldown consommé, pas besoin de vérifier l'événement aussi
    }

    // Déclencheur 2 : événement macro High imminent
    let evenement = match db.prochain_evenement_macro_high(0, params.straddle_horizon_min).await {
        Ok(Some(ev)) => ev,
        _ => return,
    };

    let raison = format!(
        "Événement macro imminent dans {} min — opportunité Straddle possible",
        evenement.1
    );
    if let Err(e) = inserer_prealerte(
        db,
        asset.as_str(),
        "straddle",
        &raison,
        None,
        Some(&evenement.0),
        Some(evenement.1),
    )
    .await
    {
        tracing::warn!("Pré-alerte Straddle macro {}: {}", asset.as_str(), e);
    }
}

// ── Helpers DB ───────────────────────────────────────────────────────────────

/// Vérifie qu'aucune pré-alerte n'a été insérée pour cet asset+stratégie
/// dans la fenêtre de cooldown.
async fn cooldown_ok(db: &Database, asset: &str, strategie: &str, cooldown_min: i64) -> bool {
    let seuil = chrono::Utc::now().timestamp() - cooldown_min * 60;
    let seuil_str = chrono::DateTime::from_timestamp(seuil, 0)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pre_alertes WHERE asset = ? AND strategie = ? AND cree_le >= ?",
    )
    .bind(asset)
    .bind(strategie)
    .bind(&seuil_str)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    count == 0
}

async fn inserer_prealerte(
    db: &Database,
    asset: &str,
    strategie: &str,
    raison: &str,
    score_actuel: Option<f64>,
    evenement: Option<&str>,
    minutes_avant: Option<i64>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO pre_alertes (asset, strategie, raison, score_actuel, evenement, minutes_avant)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(asset)
    .bind(strategie)
    .bind(raison)
    .bind(score_actuel)
    .bind(evenement)
    .bind(minutes_avant)
    .execute(db.pool())
    .await?;

    tracing::info!(
        "⚠️  Pré-alerte {} {} insérée: {}",
        strategie.to_uppercase(),
        asset,
        raison
    );
    Ok(())
}
