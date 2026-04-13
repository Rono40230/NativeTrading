//! Logique partagée de filtrage LLM, sauvegarde et publication d'un signal Rocket.
//! Séparé de rockets_scan.rs pour respecter la limite de 300 lignes.

use crate::signal_engine::SignalEngine;
use crate::straddle_categorisation::session_active;
use db::rockets::{self, NouveauRocket};
use db::rockets_feedback::{inserer_feedback, lister_pool_phase, lister_recents_ticker_phase, NouveauFeedbackRocket};
use db::rockets_feedback_stats::taux_reussite_recent;
use std::sync::Arc;
use strategies::rockets_indicateurs::ScanResultat;
use strategies::rockets_position::calculer_split_vente;

/// Niveaux TP/SL pré-calculés pour un candidat.
pub struct NiveauxRocket {
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: f64,
    pub trailing_coeff: f64,
}

/// Calcule les niveaux TP/SL depuis un ScanResultat en respectant la R:R configurée.
/// SL  = entrée − ATR × sl_mult            (R−1)
/// TP1 = entrée + ATR × tp1_mult()         (R+1 = sl_mult+1)
/// TP2 = entrée + ATR × tp2_mult()         (R+2 = sl_mult+2)
/// TP3 = entrée + ATR × trailing_trigger   (R+3 = déclencheur SL→TP2)
pub fn calculer_niveaux(
    r: &ScanResultat,
    cfg: &db::rockets_config::RocketsConfig,
) -> NiveauxRocket {
    let sl = r.prix - r.atr14 * cfg.sl_mult;
    let tp1 = r.prix + r.atr14 * cfg.tp1_mult();
    let tp2 = r.prix + r.atr14 * cfg.tp2_mult();
    let tp3 = r.prix + r.atr14 * cfg.trailing_trigger_mult();
    NiveauxRocket { sl, tp1, tp2, tp3, trailing_coeff: r.trailing_coeff }
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
    cfg: &db::rockets_config::RocketsConfig,
) {
    let session = session_active(chrono::Utc::now());
    let seuils = db::rockets_calibration::charger_seuils(pool, phase_sauvegardee, &session).await;

    // Contexte de marché global : taux de réussite des 48 dernières heures
    let (nb_recent, wr_recent, pnl_recent) = taux_reussite_recent(pool, 48).await;

    // Sélection des feedbacks few-shot par similarité de profil :
    // On charge un pool large sur la phase (tous tickers), puis on trie par distance
    // sur les 3 axes les plus discriminants (ratio_volume, atr_ratio, rsi).
    // Si le ticker a lui-même suffisamment d'historique, ses trades propres sont prioritaires.
    let feedbacks = {
        let propres = lister_recents_ticker_phase(pool, &r.ticker, phase_sauvegardee, 5)
            .await
            .unwrap_or_default();
        if propres.len() >= 5 {
            propres
        } else {
            // Compléter avec le pool large trié par similarité
            let pool_large = lister_pool_phase(pool, phase_sauvegardee, 60)
                .await
                .unwrap_or_default();
            let rv = r.ratio_volume;
            let ar = r.atr_ratio;
            let rsi = r.rsi;
            let mut scored: Vec<_> = pool_large
                .into_iter()
                .filter(|fb| fb.ticker != r.ticker) // déjà dans propres
                .map(|fb| {
                    let dist = ((fb.ratio_volume - rv) / rv.max(0.1)).powi(2)
                        + ((fb.atr_ratio - ar) / ar.max(0.1)).powi(2)
                        + ((fb.rsi - rsi) / 100.0).powi(2);
                    (dist, fb)
                })
                .collect();
            scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut resultats = propres;
            resultats.extend(scored.into_iter().take(5 - resultats.len()).map(|(_, fb)| fb));
            resultats
        }
    };

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
        entree_limite: r.entree_limite,
        entree_stop: r.entree_stop,
        niveau_invalidation: r.niveau_invalidation,
        type_entree_rec_algo: r.type_entree_rec.clone(),
        volume_seche: r.volume_seche,
        contraction_qualite: r.contraction_qualite,
        atr50: r.atr50,
        swing_amplitudes: r.swing_amplitudes.clone(),
        session: session.clone(),
        tendance_marche_48h: (nb_recent, wr_recent, pnl_recent),
    };

    let (llm_valide, llm_conviction, llm_raison, llm_sl, llm_tp1, llm_trailing_coeff) =
        match crate::ollama::rockets_filtre::filtrer_signal(&candidat, &historique, &feedbacks)
            .await
        {
            Ok(rep) => {
                tracing::info!(
                    "LLM {} {} : valide={} conviction={}",
                    label_signal,
                    r.ticker,
                    rep.valide,
                    rep.conviction
                );
                if rep.conviction < seuils.conviction_min {
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
                // Clamp strict pour éviter les hallucinations LLM
                let tc_s = rep.ajustements.as_ref()
                    .and_then(|a| a.trailing_coeff_suggere)
                    .map(|v| v.clamp(cfg.trailing_coeff_min, cfg.trailing_coeff_max));
                // Annoter entry_type si le LLM diverge de l'algo
                let raison_avec_entree = {
                    let et_llm = rep.ajustements.as_ref().and_then(|a| a.entry_type_suggere.as_deref());
                    let et_algo = r.type_entree_rec.as_str();
                    match et_llm {
                        Some(et) if et != et_algo => {
                            rep.raison + &format!(" [entrée: {}→{}]", et_algo, et)
                        }
                        _ => rep.raison,
                    }
                };
                (
                    Some(true),
                    Some(rep.conviction),
                    Some(raison_avec_entree),
                    sl_s,
                    tp1_s,
                    tc_s,
                )
            }
            Err(e) => {
                tracing::warn!(
                    "LLM {} indisponible pour {} — signal abandonné: {}",
                    label_signal,
                    r.ticker,
                    e
                );
                return;
            }
        };

    let (pct_tp1, pct_tp2, pct_trailing) = calculer_split_vente(r.score, cfg);

    // Enrichir llm_raison si le LLM a ajusté le trailing_coeff
    let llm_raison_finale = match (llm_trailing_coeff, &llm_raison) {
        (Some(tc_llm), Some(raison)) => {
            let tc_algo = niveaux.trailing_coeff;
            if (tc_llm - tc_algo).abs() > 0.1 {
                tracing::info!(
                    "LLM ajuste trailing_coeff {} : {:.1} → {:.1}",
                    r.ticker, tc_algo, tc_llm
                );
                Some(format!("{} [trail {:.1}×→{:.1}×]", raison, tc_algo, tc_llm))
            } else {
                Some(raison.clone())
            }
        }
        _ => llm_raison,
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
        llm_raison: llm_raison_finale,
        llm_sl_suggere: llm_sl,
        llm_tp1_suggere: llm_tp1,
        trailing_coeff: llm_trailing_coeff.unwrap_or(niveaux.trailing_coeff),
        pct_tp1,
        pct_tp2,
        pct_trailing,
        entree_limite:       Some(r.entree_limite),
        entree_stop:         Some(r.entree_stop),
        niveau_invalidation: Some(r.niveau_invalidation),
        type_entree_rec:     Some(r.type_entree_rec.clone()),
    };

    let signal_id = match rockets::sauvegarder(pool, &nouveau).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!("Doublon {} {} — signal ignoré", label_signal, r.ticker);
            return;
        }
        Err(e) => {
            tracing::warn!("Auto-save {} {}: {}", label_signal, r.ticker, e);
            return;
        }
    };

    use common::{Direction, Signal, Timeframe};
    // Les tickers Rockets sont au format "BTCUSDT" — on retire le quote asset pour parse_asset
    let ticker_base = r.ticker
        .trim_end_matches("USDT")
        .trim_end_matches("BUSD")
        .trim_end_matches("BTC");
    if let Some(asset) = crate::utils::parse_asset(ticker_base) {
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
    }

    // Enregistrer le feedback initial (verdict=NULL, sera réconcilié par rockets_suivi)
    let fb = NouveauFeedbackRocket {
        signal_id,
        ticker: r.ticker.clone(),
        phase: phase_sauvegardee.to_string(),
        session_active: session,
        timestamp_signal: chrono::Utc::now().timestamp(),
        score_scan: r.score,
        conviction_llm: llm_conviction.unwrap_or(0),
        ratio_volume: r.ratio_volume,
        atr_ratio: r.atr_ratio,
        rsi: r.rsi,
    };
    if let Err(e) = inserer_feedback(pool, &fb).await {
        tracing::warn!("Feedback Rockets {} {}: {}", label_signal, r.ticker, e);
    }
}
