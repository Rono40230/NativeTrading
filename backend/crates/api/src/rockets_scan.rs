use crate::rockets_analyse::analyser_symbol;
use crate::signal_engine::SignalEngine;
use db::rockets::{self, NouveauRocket};
use futures_util::future::join_all;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
pub use strategies::rockets_indicateurs::ScanResultat;
use strategies::rockets_indicateurs::{
    est_eligible, phase_priorite, Ticker24h, BATCH_SIZE, MAX_DISPLAY, SCAN_SECS,
};
use tokio::sync::RwLock;

// ── État partagé (lecture depuis le handler HTTP) ────────────────────────────

static SCAN_RESULTS: OnceLock<Arc<RwLock<Vec<ScanResultat>>>> = OnceLock::new();
static TOTAL_CANDIDATS: OnceLock<Arc<RwLock<usize>>> = OnceLock::new();

pub fn get_scan_results() -> Arc<RwLock<Vec<ScanResultat>>> {
    SCAN_RESULTS
        .get_or_init(|| Arc::new(RwLock::new(vec![])))
        .clone()
}

pub fn get_total_candidats() -> Arc<RwLock<usize>> {
    TOTAL_CANDIDATS
        .get_or_init(|| Arc::new(RwLock::new(0)))
        .clone()
}

// ── Worker de scan ───────────────────────────────────────────────────────────

pub async fn demarrer_worker_scan(pool: sqlx::SqlitePool, signal_engine: Arc<SignalEngine>) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Worker scan HTTP: {}", e);
            return;
        }
    };

    loop {
        if let Err(e) = executer_scan(&client, &pool, &signal_engine).await {
            tracing::warn!("Scan rockets erreur: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(SCAN_SECS)).await;
    }
}

async fn executer_scan(
    client: &reqwest::Client,
    pool: &sqlx::SqlitePool,
    signal_engine: &Arc<SignalEngine>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Lire la config depuis la DB (paramètres ajustables par l'utilisateur)
    let cfg = rockets::lire_config(pool).await;
    tracing::info!(
        "Config scan: score_min={} rsi_max={} ratio_vol_min={} phases={:?}",
        cfg.score_min,
        cfg.rsi_max,
        cfg.ratio_volume_min,
        cfg.phases_actives
    );

    let tickers: Vec<Ticker24h> = client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send()
        .await
        .context("fetch ticker/24hr")?
        .json()
        .await
        .context("parse ticker/24hr")?;

    let vol_min = cfg.vol_marche_min;
    let candidats: Vec<String> = tickers
        .into_iter()
        .filter(|t| {
            let vol = t.quote_volume.parse::<f64>().unwrap_or(0.0);
            est_eligible(&t.symbol, vol, vol_min)
        })
        .map(|t| t.symbol[..t.symbol.len() - 4].to_string())
        .collect();

    tracing::info!("Scan rockets: {} candidats", candidats.len());
    *get_total_candidats().write().await = candidats.len();

    let mut resultats: Vec<ScanResultat> = Vec::new();
    for batch in candidats.chunks(BATCH_SIZE) {
        let futs = batch
            .iter()
            .map(|ticker| analyser_symbol(client, ticker.as_str(), &cfg));
        let res = join_all(futs).await;
        resultats.extend(res.into_iter().flatten());
    }

    resultats.sort_by(|a, b| {
        phase_priorite(&b.phase)
            .cmp(&phase_priorite(&a.phase))
            .then(b.score.cmp(&a.score))
    });
    resultats.truncate(MAX_DISPLAY);

    // Conviction LLM minimale pour sauvegarde (qualité > quantité)
    const CONVICTION_MIN: i64 = 65;

    // Auto-save breakout/pré-lancement avec filtre LLM pré-sauvegarde
    for r in resultats.iter().filter(|r| {
        cfg.phases_actives.contains(&r.phase)
            && r.score >= cfg.score_min
            && r.rsi <= cfg.rsi_max
            && r.rsi >= cfg.rsi_min
            && r.ratio_volume >= cfg.ratio_volume_min  // volume confirmé
            && r.ratio_corps >= 0.35 // pas de doji / mèche de rejet
    }) {
        // SL = entrée - ATR14 | TP1 = measured move | TP2 = 2×ATR14 | TP3 = 2×hauteur_base (measured move ×2)
        let sl = r.prix - r.atr14;
        // TP1 : measured move si la hauteur de base dépasse 1×ATR14 (plus fidèle à la stratégie Rockets)
        let tp1 = if r.hauteur_base > r.atr14 {
            r.prix + r.hauteur_base
        } else {
            r.prix + r.atr14
        };
        let tp2 = r.prix + 2.0 * r.atr14;
        // TP3 : measured move ×2 (hauteur_base floored à ATR14 pour éviter un TP3 invalide)
        let tp3 = r.prix + 2.0 * r.hauteur_base.max(r.atr14);

        // Filtre LLM : interroge l'historique du ticker + évalue le setup
        let historique = rockets::historique_ticker(pool, &r.ticker, 10).await;
        let candidat = crate::ollama::rockets_filtre::SignalCandidat {
            ticker: r.ticker.clone(),
            phase: r.phase.clone(),
            score: r.score,
            prix_entree: r.prix,
            stop_loss: sl,
            tp1,
            atr14: r.atr14,
            atr_ratio: r.atr_ratio,
            ratio_volume: r.ratio_volume,
            rsi: r.rsi,
            change1h: r.change1h,
            ratio_corps: r.ratio_corps,
            tendance_haussiere: r.tendance_haussiere,
            nb_bougies_compression: r.nb_bougies_compression,
            hauteur_base: r.hauteur_base,
        };

        let (llm_valide, llm_conviction, llm_raison, llm_sl, llm_tp1) =
            match crate::ollama::rockets_filtre::filtrer_signal(&candidat, &historique).await {
                Ok(rep) => {
                    tracing::info!(
                        "LLM filtre {} {}: valide={} conviction={}",
                        r.ticker,
                        r.phase,
                        rep.valide,
                        rep.conviction
                    );
                    if !rep.valide {
                        tracing::info!("LLM rejette {} {}: {}", r.ticker, r.phase, rep.raison);
                        continue;
                    }
                    if rep.conviction < CONVICTION_MIN {
                        tracing::info!(
                            "LLM conviction insuffisante {} {} ({}/100): {}",
                            r.ticker,
                            r.phase,
                            rep.conviction,
                            rep.raison
                        );
                        continue; // Qualité insuffisante — pas de sauvegarde
                    }
                    let sl_s = rep.ajustements.as_ref().and_then(|a| a.sl_suggere);
                    let tp1_s = rep.ajustements.as_ref().and_then(|a| a.tp1_suggere);
                    (
                        Some(true),
                        Some(rep.conviction),
                        Some(rep.raison),
                        sl_s,
                        tp1_s,
                    )
                }
                Err(e) => {
                    // Fallback : Ollama indisponible → sauvegarder sans filtre
                    tracing::warn!("LLM filtre indisponible pour {}: {}", r.ticker, e);
                    (None, None, None, None, None)
                }
            };

        let nouveau = NouveauRocket {
            ticker: r.ticker.clone(),
            phase: r.phase.clone(),
            score: r.score,
            prix_entree: r.prix,
            stop_loss: llm_sl.unwrap_or(sl),
            target: llm_tp1.unwrap_or(tp1),
            target2: Some(tp2),
            target3: Some(tp3),
            ratio_volume: r.ratio_volume,
            atr_ratio: r.atr_ratio,
            atr14: Some(r.atr14),
            rsi: r.rsi,
            llm_valide,
            llm_conviction,
            llm_raison,
            llm_sl_suggere: llm_sl,
            llm_tp1_suggere: llm_tp1,
        };
        if let Err(e) = rockets::sauvegarder(pool, &nouveau).await {
            tracing::warn!("Auto-save rocket {}: {}", r.ticker, e);
        } else {
            // Pipeline unifié : publier dans WebSocket + Telegram
            use common::{Direction, Signal, Timeframe};
            if let Some(asset) = crate::utils::parse_asset(&r.ticker) {
                let sl_final = nouveau.llm_sl_suggere.unwrap_or(nouveau.stop_loss);
                let tp1_final = nouveau.llm_tp1_suggere.unwrap_or(nouveau.target);
                let signal = Signal::nouveau(
                    asset,
                    Timeframe::M15,
                    Direction::Long,
                    r.score as f64,
                    nouveau.prix_entree,
                    sl_final,
                    vec![
                        tp1_final,
                        nouveau.target2.unwrap_or(tp1_final),
                        nouveau.target3.unwrap_or(tp1_final),
                    ],
                    "Rockets",
                );
                signal_engine.publier(signal.clone());
                crate::telegram::notifier_telegram(signal);
            }
        }
    }

    let n = resultats.len();
    *get_scan_results().write().await = resultats;
    tracing::info!("Scan rockets terminé: {} signaux actifs", n);
    Ok(())
}
