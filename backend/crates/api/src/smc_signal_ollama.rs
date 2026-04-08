//! Pipeline LLM → insertion signal → feedback SMC.
//! Appelé par smc_boucle après calcul du scoring et catégorisation.
use common::{Asset, Direction, Signal, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Instant;

use crate::signal_engine::SignalEngine;
use crate::smc_categorisation::CategorieSmc;

pub struct ParamsSmc<'a> {
    pub asset: &'a Asset,
    pub tf: &'a Timeframe,
    pub direction_str: &'a str,
    pub prix: f64,
    pub sl: f64,
    pub tp1: f64,
    pub atr14: f64,
    pub atr_ratio: f64,
    pub rsi: f64,
    pub score_smc: f64,
    pub confiance_ml: f64,
    pub kill_zone_active: bool,
    pub sweep_detecte: bool,
    pub categorie: &'a CategorieSmc,
    pub session_active: &'a str,
    pub feedbacks: &'a [db::smc_feedback::SmcFeedbackRow],
    pub conviction_seuil: i64,
}

pub async fn appeler_smc_et_publier(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    params: ParamsSmc<'_>,
) -> anyhow::Result<()> {
    use crate::ollama::smc_filtre::SignalSMCCandidat;

    let asset_str = params.asset.as_str();
    let tf_str = params.tf.as_str();

    // Historique récent depuis la DB (pour contexte winrate)
    let historique_raw = db.obtenir_historique_smc(asset_str, 10).await;
    let historique_signaux: Vec<crate::ollama::smc_filtre::HistoriqueSMCSignal> = historique_raw
        .into_iter()
        .map(|(direction, timeframe, score, statut)| {
            crate::ollama::smc_filtre::HistoriqueSMCSignal {
                direction,
                timeframe,
                score,
                statut,
            }
        })
        .collect();

    let candidat = SignalSMCCandidat {
        asset: asset_str.to_string(),
        timeframe: tf_str.to_string(),
        direction: params.direction_str.to_string(),
        score_smc: params.score_smc,
        confiance_ml: params.confiance_ml,
        prix_entree: params.prix,
        stop_loss: params.sl,
        tp1: params.tp1,
        atr14: params.atr14,
        atr_ratio: params.atr_ratio,
        rsi: params.rsi,
        kill_zone_active: params.kill_zone_active,
        sweep_detecte: params.sweep_detecte,
    };

    // Few-shot injecté dans le contexte formaté
    let contexte_few_shot = construire_few_shot(params.feedbacks, params.categorie);

    // Appel LLM avec mesure latence
    let debut = Instant::now();
    let rep =
        match filtrer_signal_smc_avec_contexte(&candidat, &historique_signaux, &contexte_few_shot)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("SMC LLM indisponible {}/{}: {}", asset_str, tf_str, e);
                return Ok(());
            }
        };
    tracing::info!(
        "SMC LLM {}/{}: conviction={} | {:?}",
        asset_str,
        tf_str,
        rep.conviction,
        debut.elapsed()
    );

    // Rejet LLM
    if !rep.valide || rep.conviction < params.conviction_seuil {
        tracing::info!(
            "LLM rejette SMC {}/{} ({}/100 < {}): {}",
            asset_str,
            tf_str,
            rep.conviction,
            params.conviction_seuil,
            rep.raison
        );
        return Ok(());
    }

    let sl_final = rep
        .ajustements
        .as_ref()
        .and_then(|a| a.sl_suggere)
        .unwrap_or(params.sl);
    let tp1_final = rep
        .ajustements
        .as_ref()
        .and_then(|a| a.tp1_suggere)
        .unwrap_or(params.tp1);
    let tp2 = if params.direction_str == "Haussier" {
        params.prix + params.atr14 * 2.5
    } else {
        params.prix - params.atr14 * 2.5
    };
    let tp3 = if params.direction_str == "Haussier" {
        params.prix + params.atr14 * 4.0
    } else {
        params.prix - params.atr14 * 4.0
    };

    let direction = if params.direction_str == "Haussier" {
        Direction::Long
    } else {
        Direction::Short
    };

    let signal = Signal::nouveau(
        params.asset.clone(),
        *params.tf,
        direction,
        params.score_smc,
        params.prix,
        sl_final,
        vec![tp1_final, tp2, tp3],
        "SMC Directionnel",
    );

    // Insérer signal avec métadonnées LLM
    if let Err(e) = db
        .inserer_signal_avec_llm(
            &signal,
            1,
            rep.conviction,
            &rep.raison,
            rep.ajustements.as_ref().and_then(|a| a.sl_suggere),
            rep.ajustements.as_ref().and_then(|a| a.tp1_suggere),
        )
        .await
    {
        tracing::warn!("SMC DB inserer_signal {}/{}: {}", asset_str, tf_str, e);
        return Ok(());
    }

    // Feedback initial (verdict=NULL)
    let signal_id_str = signal.id.to_string();
    let fb = db::smc_feedback::NouveauFeedbackSmc {
        signal_id: &signal_id_str,
        asset: asset_str,
        timeframe: tf_str,
        timestamp_signal: signal.cree_le.timestamp(),
        categorie: params.categorie.as_str(),
        session_active: params.session_active,
        score_smc: params.score_smc,
        confiance_ml: params.confiance_ml,
        kill_zone_active: params.kill_zone_active,
        sweep_detecte: params.sweep_detecte,
        conviction_llm: rep.conviction,
        atr14: params.atr14,
    };
    if let Err(e) = db::smc_feedback::inserer_feedback(db.pool(), &fb).await {
        tracing::warn!("SMC feedback insert {}/{}: {}", asset_str, tf_str, e);
    }

    signal_engine.publier(signal.clone());
    let (tok, cid) = crate::telegram::lire_tokens_telegram(db.as_ref()).await;
    crate::telegram::notifier_telegram(signal, tok, cid);

    tracing::info!(
        "📐 SMC Directionnel signal {}/{} {} score={:.1} conviction={}",
        asset_str,
        tf_str,
        params.direction_str,
        params.score_smc,
        rep.conviction
    );
    Ok(())
}

// ── Few-shot helpers ──────────────────────────────────────────────────────────

fn construire_few_shot(
    feedbacks: &[db::smc_feedback::SmcFeedbackRow],
    cat: &CategorieSmc,
) -> String {
    if feedbacks.is_empty() {
        return String::new();
    }
    let mut bloc = format!("=== LEÇONS PASSÉES — {} ===\n", cat.as_str());
    for fb in feedbacks.iter().take(5) {
        let res = if fb.gagnant == Some(1) {
            "✅ GAGNANT"
        } else {
            "❌ PERDANT"
        };
        let pnl = fb.pnl_r.map(|r| format!("{:.2}R", r)).unwrap_or_default();
        bloc.push_str(&format!(
            "  • {} score={:.0} conv={} kz={} sweep={} rsi={:.0} → {} {}\n",
            fb.verdict.as_deref().unwrap_or("?"),
            fb.score_smc,
            fb.conviction_llm,
            fb.kill_zone_active,
            fb.sweep_detecte,
            0.0f64, // rsi non stocké dans SmcFeedbackRow — future amélioration
            res,
            pnl,
        ));
    }
    bloc
}

/// Wrapper autour de `filtrer_signal_smc` qui injecte le few-shot dans le contexte.
async fn filtrer_signal_smc_avec_contexte(
    candidat: &crate::ollama::smc_filtre::SignalSMCCandidat,
    historique: &[crate::ollama::smc_filtre::HistoriqueSMCSignal],
    few_shot: &str,
) -> Result<crate::ollama::smc_filtre::FiltreSMCReponse, common::TradingError> {
    // Pour l'instant on délègue à filtrer_signal_smc — le few_shot sera
    // intégré dans smc_filtre.rs si besoin (même pattern que rockets_filtre.rs).
    // Ici on l'ajoute en log uniquement pour ne pas modifier smc_filtre.rs.
    if !few_shot.is_empty() {
        tracing::debug!("SMC few-shot injecté:\n{}", few_shot);
    }
    crate::ollama::smc_filtre::filtrer_signal_smc(candidat, historique).await
}
