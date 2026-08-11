//! Job quotidien de mise à jour des valeur_pips dans asset_params.
//! Seules les paires JPY ont une valeur_pips variable (= 1000 / USDJPY).
//! Les autres assets ont une valeur_pips stable et restent inchangés.
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use db::Database;

use crate::prix_utils;

/// Paires dont la valeur_pips dépend du taux USDJPY.
const PAIRES_JPY: &[&str] = &[
    "USDJPY", "GBPJPY", "EURJPY", "CADJPY", "NZDJPY", "CHFJPY", "AUDJPY",
];

/// Démarre le job de mise à jour des valeur_pips.
/// Lance une première exécution immédiate au démarrage, puis toutes les 24h.
pub fn demarrer_pip_updater(
    db: Arc<Database>,
    ig_session: std::sync::Arc<tokio::sync::Mutex<crate::ig_session::IgSession>>,
) {
    tokio::spawn(async move {
        // Première exécution immédiate (au démarrage du serveur)
        executer(&db, &ig_session).await;

        loop {
            sleep(Duration::from_secs(86_400)).await;
            executer(&db, &ig_session).await;
        }
    });
}

async fn executer(
    db: &Arc<Database>,
    ig_session: &std::sync::Arc<tokio::sync::Mutex<crate::ig_session::IgSession>>,
) {
    let client = &*crate::http_client::HTTP_CLIENT;

    // Récupère le taux USDJPY via IG Markets
    let usdjpy = match prix_utils::fetch_prix_asset(&client, "USDJPY", ig_session, db).await {
        Some(p) if p > 0.0 => p,
        _ => {
            tracing::warn!("pip_updater: USDJPY non disponible, mise à jour reportée");
            return;
        }
    };

    let valeur_pip_jpy = 1000.0 / usdjpy;
    tracing::info!(
        "pip_updater: USDJPY={:.3} → valeur_pip JPY pairs = {:.4}",
        usdjpy,
        valeur_pip_jpy
    );

    let pool = db.pool();
    for asset in PAIRES_JPY {
        match sqlx::query(
            "UPDATE asset_params SET valeur_pips = ?, maj_le = datetime('now')
             WHERE asset = ?",
        )
        .bind(valeur_pip_jpy)
        .bind(*asset)
        .execute(pool)
        .await
        {
            Ok(r) if r.rows_affected() > 0 => {
                tracing::debug!("pip_updater: {} → valeur_pips={:.4}", asset, valeur_pip_jpy);
            }
            Ok(_) => {
                // Asset pas encore dans asset_params — ignoré silencieusement
            }
            Err(e) => {
                tracing::error!("pip_updater: UPDATE {} échoué: {}", asset, e);
            }
        }
    }

    tracing::info!(
        "pip_updater: ✅ {} paires JPY mises à jour (valeur_pips={:.4})",
        PAIRES_JPY.len(),
        valeur_pip_jpy
    );
}
