//! Worker centralisé Telegram — remplace les appels inline dispersés.
//!
//! Toutes les stratégies (SMC, Straddle, Rockets) passent par ce worker.
//! Avantages :
//!   • Robuste : retry automatique au redémarrage si un envoi a échoué
//!   • Unifié : un seul point de configuration et de logging
//!   • Zéro perte silencieuse : flag telegram_envoye en DB

use sqlx::{Row, SqlitePool};
use tokio::time::{interval, Duration};

use crate::telegram_formatage::{formater_prealerte, formater_rocket, formater_signal};

/// Lance le worker. À appeler une seule fois dans main.rs.
pub async fn demarrer_worker_telegram(pool: SqlitePool) {
    let mut tick = interval(Duration::from_secs(15));
    loop {
        tick.tick().await;
        let (token, chat_id) = crate::telegram::lire_tokens_pool(&pool).await;
        if token.is_empty() || chat_id.is_empty() {
            tracing::debug!("Telegram worker: tokens non configurés, skip.");
            continue;
        }
        traiter_signaux(&pool, &token, &chat_id).await;
        traiter_rockets(&pool, &token, &chat_id).await;
        traiter_pre_alertes(&pool, &token, &chat_id).await;
    }
}

// ── Signaux SMC / Straddle ────────────────────────────────────────────────────

async fn traiter_signaux(pool: &SqlitePool, token: &str, chat_id: &str) {
    let rows = match sqlx::query(
        "SELECT id, asset, direction, score, prix_entree, stop_loss,
                take_profit, strategie, llm_conviction, llm_raison
         FROM signaux WHERE telegram_envoye = 0 AND statut != 'Fermé'
         ORDER BY cree_le ASC LIMIT 10",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Telegram worker signaux: {}", e);
            return;
        }
    };

    for row in rows {
        let id: String = row.get("id");
        let asset: String = row.get("asset");
        let direction: String = row.get("direction");
        let strategie: String = row.get("strategie");
        let score: f64 = row.get("score");
        let prix_entree: f64 = row.get("prix_entree");
        let stop_loss: f64 = row.get("stop_loss");
        let tp_raw: String = row.get("take_profit");
        let take_profit: Vec<f64> = serde_json::from_str(&tp_raw).unwrap_or_default();
        let llm_conviction: Option<f64> = row.try_get("llm_conviction").ok().flatten();
        let llm_raison: Option<String> = row.try_get("llm_raison").ok().flatten();

        let p_taille: f64 = sqlx::query("SELECT taille_pip FROM asset_params WHERE asset = ?")
            .bind(&asset)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.get::<f64, _>("taille_pip"))
            .unwrap_or(0.0001);

        let p_to_pt: f64 = sqlx::query("SELECT pip_to_points FROM asset_params WHERE asset = ?")
            .bind(&asset)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.get::<f64, _>("pip_to_points"))
            .unwrap_or(10.0);

        let texte = formater_signal(
            &strategie,
            &direction,
            &asset,
            score,
            prix_entree,
            stop_loss,
            &take_profit,
            llm_conviction,
            llm_raison.as_deref(),
            p_taille,
            p_to_pt,
        );

        match crate::telegram::post_message(token, chat_id, &texte).await {
            Ok(_) => {
                tracing::info!("✉️  Telegram → {}/{}", strategie, asset);
                let _ =
                    sqlx::query("UPDATE signaux SET telegram_envoye = 1 WHERE id = ?")
                        .bind(&id)
                        .execute(pool)
                        .await;
            }
            Err(e) => tracing::warn!("Telegram signal {}: {}", id, e),
        }
    }
}

// ── Rockets ───────────────────────────────────────────────────────────────────

async fn traiter_rockets(pool: &SqlitePool, token: &str, chat_id: &str) {
    let rows = match sqlx::query(
        "SELECT id, ticker, phase, score, prix_entree, stop_loss, target, target2,
                trailing_coeff, llm_conviction, llm_raison,
                entree_limite, entree_stop, niveau_invalidation, type_entree_rec
         FROM rockets_signaux WHERE telegram_envoye = 0 AND statut != 'ferme'
         ORDER BY cree_le ASC LIMIT 10",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Telegram worker rockets: {}", e);
            return;
        }
    };

    for row in rows {
        let id: i64 = row.get("id");
        let ticker: String = row.get("ticker");
        let phase: String = row.get("phase");
        let score: i64 = row.get("score");
        let prix_entree: f64 = row.get("prix_entree");
        let stop_loss: f64 = row.get("stop_loss");
        let target: f64 = row.get("target");
        let target2: Option<f64> = row.try_get("target2").ok().flatten();
        let trailing_coeff: f64 = row.try_get("trailing_coeff").ok().flatten().unwrap_or(2.0);
        let llm_conviction: Option<i64> = row.try_get("llm_conviction").ok().flatten();
        let llm_raison: Option<String> = row.try_get("llm_raison").ok().flatten();
        let entree_limite: Option<f64> = row.try_get("entree_limite").ok().flatten();
        let entree_stop: Option<f64> = row.try_get("entree_stop").ok().flatten();
        let niveau_invalidation: Option<f64> = row.try_get("niveau_invalidation").ok().flatten();
        let type_entree_rec: Option<String> = row.try_get("type_entree_rec").ok().flatten();

        let texte = formater_rocket(
            &ticker,
            &phase,
            score,
            prix_entree,
            stop_loss,
            target,
            target2,
            trailing_coeff,
            llm_conviction,
            llm_raison.as_deref(),
            entree_limite,
            entree_stop,
            niveau_invalidation,
            type_entree_rec.as_deref(),
        );

        match crate::telegram::post_message(token, chat_id, &texte).await {
            Ok(_) => {
                tracing::info!("✉️  Telegram Rocket → {}", ticker);
                let _ = sqlx::query(
                    "UPDATE rockets_signaux SET telegram_envoye = 1 WHERE id = ?",
                )
                .bind(id)
                .execute(pool)
                .await;
            }
            Err(e) => tracing::warn!("Telegram rocket {}: {}", ticker, e),
        }
    }
}

// ── Pré-alertes (setups en formation) ────────────────────────────────────────

async fn traiter_pre_alertes(pool: &SqlitePool, token: &str, chat_id: &str) {
    let rows = match sqlx::query(
        "SELECT id, asset, strategie, raison, score_actuel, evenement, minutes_avant
         FROM pre_alertes WHERE telegram_envoye = 0
         ORDER BY cree_le ASC LIMIT 10",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Telegram worker pre_alertes: {}", e);
            return;
        }
    };

    for row in rows {
        let id: String = row.get("id");
        let asset: String = row.get("asset");
        let strategie: String = row.get("strategie");
        let raison: String = row.get("raison");
        let score_actuel: Option<f64> = row.try_get("score_actuel").ok().flatten();
        let evenement: Option<String> = row.try_get("evenement").ok().flatten();
        let minutes_avant: Option<i64> = row.try_get("minutes_avant").ok().flatten();

        let texte = formater_prealerte(
            &strategie,
            &asset,
            &raison,
            score_actuel,
            evenement.as_deref(),
            minutes_avant,
        );

        match crate::telegram::post_message(token, chat_id, &texte).await {
            Ok(_) => {
                tracing::info!("⚠️  Telegram Pré-alerte → {}/{}", strategie, asset);
                let _ = sqlx::query(
                    "UPDATE pre_alertes SET telegram_envoye = 1 WHERE id = ?",
                )
                .bind(&id)
                .execute(pool)
                .await;
            }
            Err(e) => tracing::warn!("Telegram pre_alerte {}: {}", id, e),
        }
    }
}
