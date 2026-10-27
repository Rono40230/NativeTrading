//! Journaux du runtime (mode shadow) — observation des bougies et
//! émissions live, extraits de runtime_tick.rs (garde-fou 600 lignes).

use std::sync::Arc;

use db::Database;

/// Journal d'observation (mode shadow) : chaque bougie clôturée par le
/// runtime est persistée avec son mode de clôture. La comparaison avec les
/// bougies officielles se fait à la demande (GET /api/runtime/concordance).
pub(crate) async fn journal_observation(db: Arc<Database>, bus: engine::BusBougies) {
    let mut rx = bus.abonner();
    tracing::info!("📓 Journal d'observation du runtime actif (table runtime_observation)");
    loop {
        match rx.recv().await {
            Ok(b) => {
                if let Err(e) = db
                    .inserer_observation_runtime(
                        &b.asset,
                        &b.tf,
                        &b.bougie,
                        b.mode.libelle(),
                        chrono::Utc::now(),
                    ).await
                {
                    tracing::warn!(
                        "Journal observation: erreur DB {} {} ts={} : {}",
                        b.asset.as_str(),
                        b.tf.as_str(),
                        b.bougie.timestamp.timestamp(),
                        e
                    );
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Journal observation: {} bougies sautées (lecteur lent)", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                tracing::error!("Journal observation: bus fermé — writer arrêté");
                return;
            }
        }
    }
}

/// Journal des émissions LIVE du runtime (shadow mode 2.6) : chaque signal
/// et chaque événement lifecycle publiés sur les bus est persisté — c'est
/// la matière brute du test de vérité (Gate 2). Aucune action n'en découle.
pub(crate) async fn journal_emissions(
    db: Arc<Database>,
    bus_signaux: engine::BusSignaux,
    bus_evenements: engine::BusEvenements,
) {
    let mut rx_signaux = bus_signaux.abonner();
    let mut rx_evenements = bus_evenements.abonner();
    tracing::info!("📓 Journal des émissions live actif (table runtime_emissions)");

    loop {
        tokio::select! {
            peut_etre = rx_signaux.recv() => {
                match peut_etre {
                    Ok(s) => {
                        let tps = serde_json::to_string(&s.take_profits).unwrap_or_default();
                        if let Err(e) = db.inserer_emission_signal(
                            &s.moteur, s.asset.as_str(), s.tf.as_str(),
                            format!("{:?}", s.direction), s.prix_entree, s.stop_loss,
                            &tps, s.score, &s.raison, s.debut_barre, s.emis_le,
                        ).await {
                            tracing::warn!("Journal émissions (signal): {}", e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Journal émissions: {} signaux sautés", n);
                    }
                    Err(_) => return,
                }
            }
            peut_etre = rx_evenements.recv() => {
                match peut_etre {
                    Ok(e) => {
                        if let Err(err) = db.inserer_emission_evenement(
                            &e.moteur, e.asset.as_str(), e.tf.as_str(),
                            &e.cle_trade, format!("{:?}", e.evenement), &e.detail,
                            e.prix, e.debut_barre, e.emis_le,
                        ).await {
                            tracing::warn!("Journal émissions (événement): {}", err);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Journal émissions: {} événements sautés", n);
                    }
                    Err(_) => return,
                }
            }
        }
    }
}
