//! Logique partagée de filtrage LLM, sauvegarde et publication d'un signal Rocket.
//! Séparé de rockets_scan.rs pour respecter la limite de 300 lignes.

use crate::signal_engine::SignalEngine;
use db::rockets::{self, NouveauRocket};
use std::sync::Arc;
use strategies::rockets_indicateurs::ScanResultat;

pub const CONVICTION_MIN: i64 = 65;

/// Niveaux TP/SL pré-calculés pour un candidat.
pub struct NiveauxRocket {
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: f64,
}

/// Calcule les niveaux TP/SL depuis un ScanResultat.
pub fn calculer_niveaux(r: &ScanResultat) -> NiveauxRocket {
    let sl = r.prix - r.atr14;
    let tp1 = if r.hauteur_base > r.atr14 {
        r.prix + r.hauteur_base
    } else {
        r.prix + r.atr14
    };
    let tp2 = r.prix + 2.0 * r.atr14;
    let tp3 = r.prix + 2.0 * r.hauteur_base.max(r.atr14);
    NiveauxRocket { sl, tp1, tp2, tp3 }
}

/// Filtre LLM → sauvegarde → publication WebSocket + Telegram.
/// `phase_sauvegardee` : phase à écrire en DB (peut différer de `r.phase`).
/// `label_signal`      : étiquette du signal (`"Rockets"` ou `"Rockets-Momentum"`).
pub async fn filtrer_sauvegarder_publier(
    r: &ScanResultat,
    niveaux: &NiveauxRocket,
    phase_sauvegardee: &str,
    label_signal: &str,
    pool: &sqlx::SqlitePool,
    signal_engine: &Arc<SignalEngine>,
) {
    let historique = rockets::historique_ticker(pool, &r.ticker, 10).await;
    let candidat = crate::ollama::rockets_filtre::SignalCandidat {
        ticker: r.ticker.clone(),
        phase: r.phase.clone(),
        score: r.score,
        prix_entree: r.prix,
        stop_loss: niveaux.sl,
        tp1: niveaux.tp1,
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
                    "LLM {} {} : valide={} conviction={}",
                    label_signal,
                    r.ticker,
                    rep.valide,
                    rep.conviction
                );
                if !rep.valide || rep.conviction < CONVICTION_MIN {
                    tracing::info!(
                        "LLM rejette {} {} ({}/100): {}",
                        label_signal,
                        r.ticker,
                        rep.conviction,
                        rep.raison
                    );
                    return;
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
                tracing::warn!("LLM {} indisponible pour {}: {}", label_signal, r.ticker, e);
                (None, None, None, None, None)
            }
        };

    let nouveau = NouveauRocket {
        ticker: r.ticker.clone(),
        phase: phase_sauvegardee.to_string(),
        score: r.score,
        prix_entree: r.prix,
        stop_loss: llm_sl.unwrap_or(niveaux.sl),
        target: llm_tp1.unwrap_or(niveaux.tp1),
        target2: Some(niveaux.tp2),
        target3: Some(niveaux.tp3),
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
        tracing::warn!("Auto-save {} {}: {}", label_signal, r.ticker, e);
        return;
    }

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
            label_signal,
        );
        signal_engine.publier(signal.clone());
        crate::telegram::notifier_telegram(signal);
    }
}
