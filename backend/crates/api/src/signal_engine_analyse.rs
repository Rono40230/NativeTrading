//! Logique d'analyse des assets pour le Signal Engine.
//! Séparé de signal_engine.rs pour respecter la limite de 300 lignes.
use crate::signal_filtre::sauvegarder_signal_avec_filtre;
use common::{Asset, Signal, Timeframe};
use db::{strategies_params::lire_smc_params, Database};
use ml::PipelineML;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use strategies::smc_directional::SmcDirectionalStrategy;
use strategies::Strategy;
use tokio::sync::{broadcast, Mutex};

use super::signal_engine::{ASSETS_FALLBACK, DOUBLON_MINUTES, INTERVALLE_SECS, TIMEFRAMES};

struct ContexteSignal<'a> {
    score_news: &'a Arc<AtomicI32>,
    fg_valeur: &'a Arc<AtomicI32>,
}

pub(crate) async fn boucle_detection(
    running: Arc<AtomicBool>,
    prochain: Arc<std::sync::Mutex<i64>>,
    db: Arc<Database>,
    pipeline_ml: Arc<Mutex<PipelineML>>,
    tx: broadcast::Sender<Signal>,
    score_news: Arc<AtomicI32>,
    fg_valeur: Arc<AtomicI32>,
) {
    tracing::info!(
        "🤖 Signal Engine démarré — cycle {}s | assets dynamiques × {} TF",
        INTERVALLE_SECS,
        TIMEFRAMES.len()
    );

    while running.load(Ordering::SeqCst) {
        let ts_debut = chrono::Utc::now().timestamp();
        {
            if let Ok(mut guard) = prochain.lock() {
                *guard = ts_debut + INTERVALLE_SECS as i64;
            }
        }

        // Rechargement des paramètres SMC depuis la DB à chaque cycle
        let smc_params = lire_smc_params(db.pool()).await;
        let strategie = SmcDirectionalStrategy { params: smc_params };

        analyser_tous_assets(&strategie, &db, &pipeline_ml, &tx, &score_news, &fg_valeur).await;

        let steps = INTERVALLE_SECS / 5;
        for _ in 0..steps {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    tracing::info!("🛑 Signal Engine arrêté");
}

async fn analyser_tous_assets(
    strategie: &SmcDirectionalStrategy,
    db: &Arc<Database>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
    tx: &broadcast::Sender<Signal>,
    score_news: &Arc<AtomicI32>,
    fg_valeur: &Arc<AtomicI32>,
) {
    // E.1 — Suspension SMC Directionnel si événement macro High-impact dans ≤30 min
    match db.fenetre_macro_smc_dans_minutes().await {
        Ok(Some((titre, minutes))) => {
            tracing::info!(
                "⏸ SMC Directionnel suspendu — '{}' dans {} min",
                titre,
                minutes
            );
            return;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("Vérification fenêtre macro SMC: {}", e),
    }
    let assets_actifs = match db.lister_assets().await {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|r| crate::utils::parse_asset(&r.id))
            .collect::<Vec<Asset>>(),
        Err(e) => {
            tracing::warn!(
                "Signal Engine — impossible de charger les assets DB: {} — fallback",
                e
            );
            ASSETS_FALLBACK.to_vec()
        }
    };

    tracing::debug!("Signal Engine — analyse {} assets", assets_actifs.len());

    let ctx = ContexteSignal { score_news, fg_valeur };
    for asset in &assets_actifs {
        for timeframe in TIMEFRAMES {
            if let Err(e) = analyser_asset(
                strategie,
                db,
                pipeline_ml,
                tx,
                asset,
                timeframe,
                &ctx,
            )
            .await
            {
                tracing::warn!(
                    "Signal Engine — {}/{}: {}",
                    asset.as_str(),
                    timeframe.as_str(),
                    e
                );
            }
        }
    }
}

async fn analyser_asset(
    strategie: &SmcDirectionalStrategy,
    db: &Arc<Database>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
    tx: &broadcast::Sender<Signal>,
    asset: &Asset,
    timeframe: &Timeframe,
    ctx: &ContexteSignal<'_>,
) -> common::Result<()> {
    let score_news = ctx.score_news;
    let fg_valeur = ctx.fg_valeur;
    let bougies = db.obtenir_bougies(asset, timeframe, 200).await?;
    if bougies.len() < 30 {
        return Ok(());
    }

    let signal_strat = match strategie.analyze(&bougies)? {
        Some(s) => s,
        None => return Ok(()),
    };

    // Gate ML : filtre si ML confiant et direction opposée à SMC
    let confiance_ml = {
        let ml = pipeline_ml.lock().await;
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
                Ok(pred) => pred.confiance,
                Err(_) => signal_strat.confiance,
            }
        } else {
            signal_strat.confiance
        }
    };

    // E.2 — Pénalité confiance SMC si environnement macro hostile (SMC uniquement, pas Straddle)
    let sn = score_news.load(Ordering::Relaxed);
    let fg = fg_valeur.load(Ordering::Relaxed);
    let penalite: f64 = match sn {
        s if s >= 80 => 0.15,
        s if s >= 60 => 0.10,
        _ => 0.0,
    } + if (0..25).contains(&fg) { 0.10 } else { 0.0 };
    let confiance_ml = if penalite > 0.0 {
        tracing::debug!(
            "E.2 pénalité SMC {}/{}: -{:.0}% (news={}, fg={})",
            asset.as_str(),
            timeframe.as_str(),
            penalite * 100.0,
            sn,
            fg
        );
        (confiance_ml - penalite).max(0.0)
    } else {
        confiance_ml
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
        Some(s) => (s.total, s.kill_zone_active, s.sweep_detecte),
        None => (signal_strat.confiance * 100.0, false, false),
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
