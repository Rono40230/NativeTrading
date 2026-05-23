//! Analyse d'un asset individuel pour le Signal Engine.
//! Séparé de signal_engine_analyse.rs pour respecter la limite de 300 lignes.
use crate::signal_filtre::sauvegarder_signal_avec_filtre;
use common::{Asset, Signal, Timeframe};
use db::Database;
use ml::PipelineML;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use strategies::smc_directional::SmcDirectionalStrategy;
use strategies::Strategy;
use tokio::sync::{broadcast, RwLock};

use crate::signal_engine::DOUBLON_MINUTES;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn analyser_asset(
    strategie: &SmcDirectionalStrategy,
    db: &Arc<Database>,
    pipeline_ml: &Arc<RwLock<PipelineML>>,
    tx: &broadcast::Sender<Signal>,
    asset: &Asset,
    timeframe: &Timeframe,
    _score_news: &Arc<AtomicI32>,
    _fg_valeur: &Arc<AtomicI32>,
) -> common::Result<()> {
    let bougies = db.obtenir_bougies(asset, timeframe, 200).await?;
    if bougies.len() < 30 {
        return Ok(());
    }

    let signal_strat = match strategie.analyze(&bougies)? {
        Some(s) => s,
        None => return Ok(()),
    };

    // Gate ML : filtre si ML confiant et direction opposée à SMC.
    // Bonus scoring : +15 pts si ML confirme la direction, +20 pts si confiance >70%.
    let (confiance_ml, bonus_ml) = {
        let ml = pipeline_ml.read().await;
        if ml.est_pret() {
            match ml.predire(&bougies) {
                Ok(pred) if pred.est_confiant && pred.direction != signal_strat.direction => {
                    tracing::debug!(
                        "Signal {}/{} rejeté par ML (ML={:?} vs SMC={:?}, conf={:.0}%)",
                        asset.as_str(),
                        timeframe.as_str(),
                        pred.direction,
                        signal_strat.direction,
                        pred.confiance * 100.0,
                    );
                    return Ok(());
                }
                Ok(pred) => {
                    let bonus = if pred.direction == signal_strat.direction {
                        if pred.confiance > 0.7 {
                            20.0
                        } else {
                            15.0
                        }
                    } else {
                        0.0
                    };
                    if bonus > 0.0 {
                        tracing::debug!(
                            "ML confirme SMC {}/{} ({:?}, conf={:.0}%) → +{:.0} pts",
                            asset.as_str(),
                            timeframe.as_str(),
                            pred.direction,
                            pred.confiance * 100.0,
                            bonus
                        );
                    }
                    (pred.confiance, bonus)
                }
                Err(_) => (signal_strat.confiance, 0.0),
            }
        } else {
            (signal_strat.confiance, 0.0)
        }
    };

    if db
        .signal_recent_existe(asset, timeframe, DOUBLON_MINUTES)
        .await?
    {
        tracing::debug!("Doublon ignoré {}/{}", asset.as_str(), timeframe.as_str());
        return Ok(());
    }

    let mut tp_list = vec![signal_strat.take_profit];
    if let Some(tp2) = signal_strat.take_profit_2 {
        tp_list.push(tp2);
    }
    if let Some(tp3) = signal_strat.take_profit_3 {
        tp_list.push(tp3);
    }

    let historique_raw = db.obtenir_contexte_llm(asset.as_str(), 5).await;
    let contexte = crate::ollama::formater_contexte_historique(
        asset.as_str(),
        "SMC Directionnel",
        &historique_raw,
    );

    let strategie_nom = crate::ollama::enrichir_signal_avec_ollama(
        asset.as_str(),
        timeframe.as_str(),
        &signal_strat,
        &bougies,
        &contexte,
    )
    .await;

    let signal = Signal::nouveau(
        asset.clone(),
        *timeframe,
        signal_strat.direction,
        confiance_ml * 100.0,
        signal_strat.prix_entree,
        signal_strat.stop_loss,
        tp_list,
        strategie_nom,
    );

    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_now = atr_vals.last().copied().unwrap_or(0.0);
    let atr_moyen = if atr_vals.len() >= 14 {
        atr_vals[atr_vals.len().saturating_sub(14)..]
            .iter()
            .sum::<f64>()
            / 14.0
    } else {
        atr_now
    };
    let atr_ratio = if atr_moyen > 0.0 {
        atr_now / atr_moyen
    } else {
        1.0
    };

    let rsi_vals = indicators::calculer_rsi(&bougies, 14);
    let rsi = rsi_vals.last().copied().unwrap_or(50.0);

    let (score_smc, kill_zone, sweep) = match smc::scorer(&bougies) {
        Some(s) => (s.total + bonus_ml, s.kill_zone_active, s.sweep_detecte),
        None => (signal_strat.confiance * 100.0 + bonus_ml, false, false),
    };

    let historique_smc = db.obtenir_historique_smc(asset.as_str(), 10).await;
    let historique_filtre: Vec<crate::ollama::smc_filtre::HistoriqueSMCSignal> = historique_smc
        .into_iter()
        .map(
            |(direction, tf, score, statut)| crate::ollama::smc_filtre::HistoriqueSMCSignal {
                direction,
                timeframe: tf,
                score,
                statut,
            },
        )
        .collect();

    let candidat = crate::ollama::smc_filtre::SignalSMCCandidat {
        asset: asset.as_str().to_string(),
        timeframe: timeframe.as_str().to_string(),
        direction: format!("{:?}", signal_strat.direction),
        score_smc,
        confiance_ml,
        prix_entree: signal_strat.prix_entree,
        stop_loss: signal_strat.stop_loss,
        tp1: signal_strat.take_profit,
        atr14: atr_now,
        atr_ratio,
        rsi,
        kill_zone_active: kill_zone,
        sweep_detecte: sweep,
    };

    sauvegarder_signal_avec_filtre(
        db,
        tx,
        &signal,
        asset,
        timeframe,
        &candidat,
        &historique_filtre,
    )
    .await
}
