use chrono::{Datelike, Timelike, Utc};
use common::{Asset, Timeframe};
use db::Database;
use ml::PipelineML;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

type WhipsawDelais = Arc<Mutex<HashMap<String, Instant>>>;

use crate::signal_engine::SignalEngine;
use crate::straddle_ml_gate::{evaluer_ml_straddle, MlContexteStraddle};
use crate::straddle_signal_ollama::{appeler_ollama_et_publier, ParamsOllama};

const ANTI_DOUBLON_MIN: i64 = 30;
const SEUIL_SIGNAL_DEFAUT: f64 = 1.5;

pub fn demarrer_boucle_straddle(
    db: Arc<Database>,
    signal_engine: Arc<SignalEngine>,
    pipeline_ml: Arc<Mutex<PipelineML>>,
) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(180)).await;
        let whipsaw_delais: WhipsawDelais = Arc::new(Mutex::new(HashMap::new()));
        loop {
            let assets = db.lister_assets().await.unwrap_or_default();
            let nb = assets.len();
            let seuil_straddle: f64 = db.lire_config("seuil_confiance_straddle").await
                .ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0.75);
            let atr_ui: f64 = {
                let p = db::strategies_params::lire_straddle_params(db.pool()).await;
                if p.atr_seuil.is_finite() && p.atr_seuil > 0.0 {
                    p.atr_seuil
                } else {
                    SEUIL_SIGNAL_DEFAUT
                }
            };
            tracing::debug!(
                "Straddle auto cycle: atr_ui={:.2}, seuil_ml={:.2}, assets_count={}",
                atr_ui,
                seuil_straddle,
                nb
            );
            let actifs_corr: HashSet<String> = db::straddle_suivi_position::lister_suivi_actifs(db.pool())
                .await.unwrap_or_default().into_iter().map(|s| s.asset).collect();
            for asset_db in &assets {
                let asset = match Asset::try_from(asset_db.id.as_str()) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let tf = Timeframe::M15;
                let asset_key = asset.as_str().to_string();
                let skip_ww = {
                    let mut map = whipsaw_delais.lock().await;
                    if let Some(&echeance) = map.get(&asset_key) {
                        if Instant::now() >= echeance { map.remove(&asset_key); true }
                        else { continue; }
                    } else { false }
                };
                analyser_asset(&db, &signal_engine, &pipeline_ml, seuil_straddle, atr_ui, &asset, &tf, &whipsaw_delais, skip_ww, &actifs_corr).await;
            }
            tracing::debug!("🌪️  Boucle Straddle cycle terminé ({} assets)", nb);
            sleep(Duration::from_secs(15 * 60)).await;
        }
    });
    tracing::info!("🌪️  Boucle Straddle auto démarrée (15 min, assets dynamiques depuis DB)");
}

#[allow(clippy::too_many_arguments)]
async fn analyser_asset(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    pipeline_ml: &Arc<Mutex<PipelineML>>,
    seuil_straddle: f64,
    atr_seuil_ui: f64,
    asset: &Asset,
    tf: &Timeframe,
    whipsaw_delais: &WhipsawDelais,
    skip_whipsaw: bool,
    actifs_corr: &HashSet<String>,
) {
    match db.signal_recent_existe_strategie(asset, tf, "Straddle", ANTI_DOUBLON_MIN).await {
        Ok(true) => return,
        Err(e) => { tracing::warn!("Straddle auto: anti-doublon {}/{}: {}", asset.as_str(), tf.as_str(), e); return; }
        Ok(false) => {}
    }
    let correlation_active = crate::straddle_utils::groupe_correlation(asset.as_str()).map(|g| g.iter().any(|a| *a != asset.as_str() && actifs_corr.contains(*a))).unwrap_or(false);
    let bougies = match db.obtenir_bougies(asset, tf, 100).await {
        Ok(b) if b.len() >= 30 => b,
        Ok(b) => { tracing::debug!("Straddle {}/{}: {} bougies", asset.as_str(), tf.as_str(), b.len()); return; }
        Err(e) => { tracing::warn!("Straddle: bougies {}/{}: {}", asset.as_str(), tf.as_str(), e); return; }
    };

    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_valides: Vec<f64> = atr_vals.iter().copied().filter(|v| !v.is_nan()).collect();
    if atr_valides.len() < 2 {
        return;
    }
    let atr_actuel = match atr_valides.last().copied() {
        Some(v) => v,
        None => return,
    };
    let n_moy = atr_valides.len().min(14);
    let atr_moyen = atr_valides.iter().rev().take(n_moy).sum::<f64>() / n_moy as f64;
    let prix = bougies.last().map(|b| b.close).unwrap_or(0.0);
    if prix <= 0.0 || atr_actuel <= 0.0 {
        return;
    }
    let ratio_atr = atr_actuel / atr_moyen.max(f64::EPSILON);

    if ratio_atr < atr_seuil_ui {
        return;
    }

    let now = Utc::now();
    let maintenant = now.timestamp();
    let dans_90min = maintenant + 5400;

    let annonces_brutes = db.lire_calendrier_cache(3600).await.unwrap_or_default();
    if annonces_brutes.is_empty() {
        let db_refresh = Arc::clone(db);
        tokio::spawn(async move {
            let n = crate::calendar_handlers::rafraichir_calendrier(db_refresh.as_ref()).await.len();
            tracing::debug!("Straddle auto: refresh calendrier fallback lancé ({} événements)", n);
        });
    }

    let annonces: Vec<serde_json::Value> = annonces_brutes
        .into_iter()
        .filter(|a| {
            a["impact"].as_str() == Some("High")
                && a["date_heure"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| {
                        let ts = dt.timestamp();
                        (maintenant..=dans_90min).contains(&ts)
                    })
                    .unwrap_or(false)
        })
        .collect();

    let asset_str = asset.as_str().to_string();
    let creneaux = db::straddle::lister_creneaux_asset(db.pool(), &asset_str)
        .await
        .unwrap_or_default();
    let creneaux_actifs: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .collect();

    let jours = [
        "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
    ];
    let jour = jours[now.weekday().num_days_from_monday() as usize % 7];
    let heure = now.hour();

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
    if correlation_active { ctx.push_str("Corrélation groupe: actif corrélé déjà ouvert (mode soft: prudence renforcée)\n"); }

    if creneaux_actifs.is_empty() {
        ctx.push_str("Créneaux historiques: aucun\n");
    } else {
        ctx.push_str("Créneaux historiques validés (sur 2 ans de données):\n");
        for c in creneaux_actifs.iter().take(3) {
            let jours_label = c.jour_semaine
                .map(|j| ["Lun","Mar","Mer","Jeu","Ven","Sam","Dim"].get(j as usize).copied().unwrap_or("?"))
                .unwrap_or("tous jours");
            let timing = c.timing_optimal.as_deref().unwrap_or("-");
            let fenetre = c.fenetre_entree.as_deref().unwrap_or("-");
            let whipsaw = c.whipsaw_minutes.map(|w| format!("{}min", w)).unwrap_or_else(|| "-".into());
            ctx.push_str(&format!(
                "  {jours_label} {hd}–{hf} UTC | ATR×{atr:.2} | freq {freq:.0}% | wr {wr}% | timing:{timing} | fenêtre:{fenetre} | whipsaw:{whipsaw}\n",
                hd = c.heure_debut,
                hf = c.heure_fin,
                atr = c.atr_moyen.unwrap_or(0.0),
                freq = c.frequence.unwrap_or(0.0) * 100.0,
                wr = c.backtest_winrate
                    .map(|w| format!("{:.0}", w))
                    .unwrap_or_else(|| "?".to_string()),
            ));
        }
    }

    let creneaux_valides_complets: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .cloned()
        .collect();
    let categorie_ctx = crate::straddle_categorisation::categoriser(
        &annonces,
        now,
        &creneaux_valides_complets,
        asset.as_str(),
    );

    let seuils = db::straddle_calibration::charger_seuils(
        db.pool(),
        asset.as_str(),
        categorie_ctx.categorie.as_str(),
    )
    .await;

    if seuils.invalide {
        tracing::debug!("Straddle {}/{}: cat {} invalide, skip", asset.as_str(), tf.as_str(), categorie_ctx.categorie.as_str());
        return;
    }

    let seuil_atr_effectif = seuils.ratio_atr.min(atr_seuil_ui);
    if ratio_atr < seuil_atr_effectif {
        tracing::debug!(
            "Straddle {}/{}: ratio {:.2} < seuil_effectif {:.2} (ui {:.2}, calib {:.2})",
            asset.as_str(),
            tf.as_str(),
            ratio_atr,
            seuil_atr_effectif,
            atr_seuil_ui,
            seuils.ratio_atr
        );
        return;
    }

    if !skip_whipsaw {
        if whipsaw_delais.lock().await.contains_key(asset.as_str()) { return; }
        let hm = heure * 60 + now.minute();
        if let Some(mins) = crate::straddle_utils::whipsaw_pour_heure(&creneaux_valides_complets, hm) {
            let echeance = Instant::now() + Duration::from_secs((mins.max(1) as u64) * 60);
            whipsaw_delais.lock().await.insert(asset.as_str().to_string(), echeance);
            tracing::info!("⏳ Straddle {}: whipsaw {}min — re-check prévu", asset.as_str(), mins);
            return;
        }
    }

    let feedbacks = db::straddle_feedback::lister_recents_asset_categorie(
        db.pool(), asset.as_str(), categorie_ctx.categorie.as_str(), 10,
    ).await.unwrap_or_default();

    let ml_contexte = evaluer_ml_straddle(pipeline_ml, &bougies, asset.as_str(), tf.as_str(), seuil_straddle).await;
    if let MlContexteStraddle::Directionnel(dir) = &ml_contexte {
        tracing::debug!("Straddle {}/{}: ML → {} dir, skip", asset.as_str(), tf.as_str(), dir);
        return;
    }
    match ml_contexte {
        MlContexteStraddle::Indecis(texte) => ctx.push_str(&texte),
        MlContexteStraddle::NonDisponible => {
            let seuil_regle = crate::straddle_score_regle::seuil_pour_feedback(feedbacks.len());
            let mut score = crate::straddle_score_regle::calculer_score(&crate::straddle_score_regle::ContexteScoreRegle {
                categorie: categorie_ctx.categorie.as_str(), ratio_atr, now, creneaux_valides: &creneaux_valides_complets,
            });
            if correlation_active {
                score = score.saturating_sub(10);
            }
            if score < seuil_regle {
                tracing::debug!("Straddle {}: règles {}/100 < seuil, skip", asset.as_str(), score);
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
        score_seuil: if correlation_active { (seuils.score_llm + 0.7).min(9.5) } else { seuils.score_llm },
        annonces: &annonces,
        bougies: &bougies,
        ratio_atr,
    };
    if let Err(e) = appeler_ollama_et_publier(db, signal_engine, asset, tf, params).await {
        tracing::warn!("Straddle auto {}/{}: {}", asset.as_str(), tf.as_str(), e);
    }
}

