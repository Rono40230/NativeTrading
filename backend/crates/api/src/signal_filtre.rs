//! Filtre LLM pré-sauvegarde des signaux SMC.
//!
//! Appelle `smc_filtre::filtrer_signal_smc` avec timeout 90s et persiste
//! le signal avec les métadonnées LLM ou en fall-back sans filtre.
use common::{Asset, Signal, Timeframe};
use db::Database;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::ollama::smc_filtre::{HistoriqueSMCSignal, SignalSMCCandidat};

pub async fn sauvegarder_signal_avec_filtre(
    db: &Arc<Database>,
    tx: &broadcast::Sender<Signal>,
    signal: &Signal,
    asset: &Asset,
    timeframe: &Timeframe,
    candidat: &SignalSMCCandidat,
    historique: &[HistoriqueSMCSignal],
) -> common::Result<()> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(90),
        crate::ollama::smc_filtre::filtrer_signal_smc(candidat, historique, ""),
    )
    .await
    {
        Ok(Ok(filtre)) => {
            if !filtre.valide {
                tracing::info!(
                    "🚫 SMC filtre LLM rejeté {}/{} conviction={} — {}",
                    asset.as_str(),
                    timeframe.as_str(),
                    filtre.conviction,
                    filtre.raison
                );
                return Ok(());
            }
            let sl = filtre.ajustements.as_ref().and_then(|a| a.sl_suggere);
            let tp1 = filtre.ajustements.as_ref().and_then(|a| a.tp1_suggere);
            db.inserer_signal_avec_llm(signal, 1, filtre.conviction, &filtre.raison, sl, tp1)
                .await?;
            tracing::info!(
                "✅ SMC+LLM {}/{} {:?} conviction={} entry={:.4}",
                asset.as_str(),
                timeframe.as_str(),
                signal.direction,
                filtre.conviction,
                signal.prix_entree
            );
        }
        Ok(Err(e)) => {
            tracing::warn!(
                "SMC filtre LLM erreur {}/{}: {} — signal sauvegardé sans filtre",
                asset.as_str(),
                timeframe.as_str(),
                e
            );
            db.inserer_signal(signal).await?;
        }
        Err(_) => {
            tracing::warn!(
                "SMC filtre LLM timeout {}/{} — signal sauvegardé sans filtre",
                asset.as_str(),
                timeframe.as_str()
            );
            db.inserer_signal(signal).await?;
        }
    }

    let _ = tx.send(signal.clone());
    Ok(())
}
