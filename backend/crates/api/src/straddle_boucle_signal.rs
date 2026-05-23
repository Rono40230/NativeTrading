//! Évaluation Straddle : contexte, seuils, ML, publication Ollama.
use chrono::{Datelike, Timelike, Utc};
use common::{Asset, Timeframe};
use db::Database;
use ml::PipelineML;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::signal_engine::SignalEngine;
use crate::straddle_boucle_analyse::{ContexteSignalStraddle, WhipsawDelais};
use crate::straddle_ml_gate::{evaluer_ml_straddle, MlContexteStraddle};
use crate::straddle_signal_ollama::{appeler_ollama_et_publier, ParamsOllama};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn evaluer_et_publier_straddle(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    pipeline_ml: &Arc<RwLock<PipelineML>>,
    seuil_straddle: f64,
    asset: &Asset,
    tf: &Timeframe,
    whipsaw_delais: &WhipsawDelais,
    skip_whipsaw: bool,
    ctx_data: ContexteSignalStraddle,
) {
    let ContexteSignalStraddle {
        bougies, atr_actuel, atr_moyen, prix, ratio_atr, annonces,
        correlation_active, atr_seuil_ui, sl_mult, tp_mult_1, tp_mult_2, tp_mult_3,
    } = ctx_data;

    let asset_str = asset.as_str().to_string();
    let creneaux = db::straddle::lister_creneaux_asset(db.pool(), &asset_str)
        .await
        .unwrap_or_default();
    let creneaux_actifs: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .collect();

    let now = Utc::now();
    let maintenant = now.timestamp();
    let heure = now.hour();
    let jours = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];
    let jour = jours[now.weekday().num_days_from_monday() as usize % 7];

    let mut ctx = format!(
        "=== CONTEXTE STRADDLE TEMPS RÉEL ===\n\
        Asset: {asset_str} | Timeframe: {tf_str} | {jour} {heure:02}h UTC\n\
        Prix: {prix:.5} | ATR actuel: {atr:.5} | ATR moyen 14p: {moy:.5} | Ratio ATR: {ratio:.2}×\n\
        Session active: {session} | Positions ouvertes: 0 | Drawdown: 0.0%\n",
        tf_str = tf.as_str(),
        atr = atr_actuel,
        moy = atr_moyen,
        ratio = ratio_atr,
        session = smc::kill_zone::nom_kill_zone(now).unwrap_or("Hors session"),
    );
    ctx.push_str(&crate::straddle_utils::formater_annonces_contexte(&annonces, maintenant));
    if correlation_active {
        ctx.push_str(
            "Corrélation groupe: actif corrélé déjà ouvert (mode soft: prudence renforcée)\n",
        );
    }
    if creneaux_actifs.is_empty() {
        ctx.push_str("Créneaux historiques: aucun\n");
    } else {
        ctx.push_str("Créneaux historiques validés (sur 2 ans de données):\n");
        for c in creneaux_actifs.iter().take(3) {
            let jours_label = c
                .jour_semaine
                .map(|j| {
                    ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"]
                        .get(j as usize)
                        .copied()
                        .unwrap_or("?")
                })
                .unwrap_or("tous jours");
            let timing = c.timing_optimal.as_deref().unwrap_or("-");
            let fenetre = c.fenetre_entree.as_deref().unwrap_or("-");
            let whipsaw_lbl = c
                .whipsaw_minutes
                .map(|w| format!("{}min", w))
                .unwrap_or_else(|| "-".into());
            ctx.push_str(&format!(
                "  {jours_label} {hd}–{hf} UTC | ATR×{atr:.2} | freq {freq:.0}% | wr {wr}% | timing:{timing} | fenêtre:{fenetre} | whipsaw:{whipsaw_lbl}\n",
                hd = c.heure_debut, hf = c.heure_fin,
                atr = c.atr_moyen.unwrap_or(0.0),
                freq = c.frequence.unwrap_or(0.0) * 100.0,
                wr = c.backtest_winrate.map(|w| format!("{:.0}", w)).unwrap_or_else(|| "?".into()),
            ));
        }
    }
    let creneaux_valides_complets: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .cloned()
        .collect();
    let categorie_ctx = crate::straddle_categorisation::categoriser(
        &annonces, now, &creneaux_valides_complets, asset.as_str(),
    );
    let seuils = db::straddle_calibration::charger_seuils(
        db.pool(), asset.as_str(), categorie_ctx.categorie.as_str(),
    )
    .await;
    if seuils.invalide {
        tracing::debug!(
            "Straddle {}/{}: cat {} invalide, skip",
            asset.as_str(), tf.as_str(), categorie_ctx.categorie.as_str()
        );
        return;
    }
    let seuil_atr_effectif = seuils.ratio_atr.min(atr_seuil_ui);
    if ratio_atr < seuil_atr_effectif {
        tracing::debug!(
            "Straddle {}/{}: ratio {:.2} < seuil_effectif {:.2} (ui {:.2}, calib {:.2})",
            asset.as_str(), tf.as_str(), ratio_atr, seuil_atr_effectif, atr_seuil_ui, seuils.ratio_atr
        );
        return;
    }
    if !skip_whipsaw {
        if whipsaw_delais.lock().await.contains_key(asset.as_str()) {
            return;
        }
        let hm = heure * 60 + now.minute();
        if let Some(mins) =
            crate::straddle_utils::whipsaw_pour_heure(&creneaux_valides_complets, hm)
        {
            let echeance = Instant::now() + Duration::from_secs((mins.max(1) as u64) * 60);
            whipsaw_delais
                .lock()
                .await
                .insert(asset.as_str().to_string(), echeance);
            tracing::info!("⏳ Straddle {}: whipsaw {}min — re-check prévu", asset.as_str(), mins);
            return;
        }
    }
    let feedbacks = db::straddle_feedback::lister_recents_asset_categorie(
        db.pool(), asset.as_str(), categorie_ctx.categorie.as_str(), 10,
    )
    .await
    .unwrap_or_default();
    let ml_contexte = evaluer_ml_straddle(
        pipeline_ml, &bougies, asset.as_str(), tf.as_str(), seuil_straddle,
    )
    .await;
    if let MlContexteStraddle::Directionnel(dir) = &ml_contexte {
        tracing::debug!(
            "Straddle {}/{}: ML → {} dir, skip",
            asset.as_str(), tf.as_str(), dir
        );
        return;
    }
    match ml_contexte {
        MlContexteStraddle::Indecis(texte) => ctx.push_str(&texte),
        MlContexteStraddle::NonDisponible => {
            let seuil_regle = crate::straddle_score_regle::seuil_pour_feedback(feedbacks.len());
            let mut score = crate::straddle_score_regle::calculer_score(
                &crate::straddle_score_regle::ContexteScoreRegle {
                    categorie: categorie_ctx.categorie.as_str(),
                    ratio_atr,
                    now,
                    creneaux_valides: &creneaux_valides_complets,
                },
            );
            if correlation_active {
                score = score.saturating_sub(10);
            }
            if score < seuil_regle {
                tracing::debug!(
                    "Straddle {}: règles {}/100 < seuil, skip",
                    asset.as_str(), score
                );
                return;
            }
            ctx.push_str(&crate::straddle_score_regle::texte_contexte(score));
        }
        MlContexteStraddle::Directionnel(_) => unreachable!(),
    }
    let params = ParamsOllama {
        prix,
        atr: atr_actuel,
        ctx: &ctx,
        feedbacks: &feedbacks,
        categorie: &categorie_ctx.categorie,
        score_seuil: if correlation_active {
            (seuils.score_llm + 0.7).min(9.5)
        } else {
            seuils.score_llm
        },
        annonces: &annonces,
        bougies: &bougies,
        ratio_atr,
        sl_mult,
        tp_mult_1,
        tp_mult_2,
        tp_mult_3,
    };
    if let Err(e) = appeler_ollama_et_publier(db, signal_engine, asset, tf, params).await {
        tracing::warn!("Straddle auto {}/{}: {}", asset.as_str(), tf.as_str(), e);
    }
}
