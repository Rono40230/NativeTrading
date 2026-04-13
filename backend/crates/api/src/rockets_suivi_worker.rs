//! Logique métier du worker de suivi Rockets (extrait de rockets_suivi.rs).
//! Un cycle = expiration + signaux en attente + signaux ouverts.

use db::{rockets, rockets_config};
use strategies::rockets_niveaux::calculer_verdict_rocket;

use crate::rockets_prix::{fetch_prix, reconcilier_feedback};

/// Exécute un cycle complet de suivi : appelé toutes les 3 min par le worker.
pub(crate) async fn executer_cycle_suivi(pool: &sqlx::SqlitePool, client: &reqwest::Client) {
    // 1. Expirer les signaux EN ATTENTE depuis >6h (position jamais ouverte)
    if let Ok(n) = rockets::marquer_expires(pool).await {
        if n > 0 {
            tracing::info!("Rockets: {} signal(s) expirés (jamais entrés)", n);
        }
    }

    // 2. Signaux en attente : SL touché avant entrée → invalide, sinon ouvrir si prix atteint
    let en_attente = match rockets::lister_en_attente(pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Worker rockets attente: {}", e);
            return;
        }
    };
    for s in &en_attente {
        let Some(prix) = fetch_prix(client, &s.ticker).await else {
            continue;
        };
        if prix <= s.stop_loss {
            if let Err(e) = rockets::maj_verdict(pool, s.id, "invalide", prix).await {
                tracing::warn!("Rocket {} SL avant entrée: {}", s.ticker, e);
            } else {
                tracing::info!(
                    "Rocket {} → invalide (SL avant entrée) @ {:.5}",
                    s.ticker,
                    prix
                );
                reconcilier_feedback(
                    pool,
                    &s.ticker,
                    s.id,
                    "invalide",
                    s.prix_entree,
                    prix,
                    s.atr14,
                    &s.cree_le,
                )
                .await;
                db::ml_samples::sauvegarder_sample(
                    pool,
                    &db::ml_samples::MlSample {
                        strategie: "ROCKETS".to_string(),
                        asset: s.ticker.clone(),
                        timeframe: "M5".to_string(),
                        direction: "LONG".to_string(),
                        prix_entree: s.prix_entree,
                        prix_sortie: prix,
                        stop_loss: s.stop_loss,
                        outcome: "invalide".to_string(),
                        rr_realise: Some(-1.0),
                    },
                )
                .await
                .ok();
            }
        } else if prix >= s.prix_entree {
            if let Err(e) = rockets::entrer_position(pool, s.id).await {
                tracing::warn!("Rocket {} entrée position: {}", s.ticker, e);
            } else {
                tracing::info!("Rocket {} → ouvert @ {:.5}", s.ticker, prix);
            }
        }
    }

    // 3. Signaux OUVERTS : TP pyramidal + trailing TP3 + SL
    let config = rockets_config::lire_config(pool).await;
    let signaux = match rockets::lister_ouverts(pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Worker rockets ouverts: {}", e);
            return;
        }
    };
    for s in &signaux {
        let Some(prix) = fetch_prix(client, &s.ticker).await else {
            continue;
        };

        let peak_precedent = s.prix_peak.unwrap_or(s.prix_entree);
        let peak = peak_precedent.max(prix);
        if peak > peak_precedent {
            if let Err(e) = rockets::maj_prix_peak(pool, s.id, peak).await {
                tracing::warn!("Rocket {} maj peak: {}", s.ticker, e);
            }
        }

        match calculer_verdict_rocket(s, prix, peak, peak_precedent) {
            Some(v @ "TP1") | Some(v @ "TP2") => {
                if config.vente_partielle {
                    if let Err(e) = rockets::enregistrer_tp_partiel(pool, s.id, v, prix).await {
                        tracing::warn!("Rocket {} tp partiel: {}", s.ticker, e);
                    } else {
                        tracing::info!("Rocket {} → {} partiel @ {:.5}", s.ticker, v, prix);
                    }
                } else {
                    tracing::info!(
                        "Rocket {} → {} (SL progresse, Option 2) @ {:.5}",
                        s.ticker,
                        v,
                        prix
                    );
                }
            }
            Some(v) => {
                if let Err(e) = rockets::maj_verdict(pool, s.id, v, prix).await {
                    tracing::warn!("Worker rockets verdict: {}", e);
                } else {
                    tracing::info!("Rocket {} → {} @ {:.5}", s.ticker, v, prix);
                    reconcilier_feedback(
                        pool,
                        &s.ticker,
                        s.id,
                        v,
                        s.prix_entree,
                        prix,
                        s.atr14,
                        &s.cree_le,
                    )
                    .await;
                    let risque = (s.prix_entree - s.stop_loss).abs().max(f64::EPSILON);
                    let rr = if v == "sl" {
                        -((s.prix_entree - prix).abs() / risque)
                    } else {
                        (prix - s.prix_entree).abs() / risque
                    };
                    db::ml_samples::sauvegarder_sample(
                        pool,
                        &db::ml_samples::MlSample {
                            strategie: "ROCKETS".to_string(),
                            asset: s.ticker.clone(),
                            timeframe: "M5".to_string(),
                            direction: "LONG".to_string(),
                            prix_entree: s.prix_entree,
                            prix_sortie: prix,
                            stop_loss: s.stop_loss,
                            outcome: v.to_string(),
                            rr_realise: Some(rr),
                        },
                    )
                    .await
                    .ok();
                }
            }
            None => {}
        }
    }
}
