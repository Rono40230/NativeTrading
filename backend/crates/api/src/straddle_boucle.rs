//! Boucle automatique d'analyse Straddle au démarrage.
//!
//! Tourne toutes les 15 minutes pour un ensemble d'assets/timeframes.
//! Reproduit la logique de `straddle_signal_handler` sans passer par HTTP.
//! Pipeline unifié : DB → signal_engine.publier() → Telegram.
use chrono::{Datelike, Timelike, Utc};
use common::{Asset, Direction, Signal, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::signal_engine::SignalEngine;
use crate::straddle_prompt::PROMPT_SIGNAL_STRADDLE;
use crate::straddle_types::{OllamaResp, ReponseLlm};

/// Assets/timeframes analysés automatiquement par la boucle Straddle.
const ASSETS_STRADDLE: &[(Asset, Timeframe)] = &[
    (Asset::BTC, Timeframe::M5),
    (Asset::ETH, Timeframe::M5),
    (Asset::XAUUSD, Timeframe::M15),
];

/// Anti-doublon : pas de second signal Straddle sur le même asset/TF avant N minutes.
const ANTI_DOUBLON_MIN: i64 = 60;

/// Démarre la boucle en background — ne bloque pas.
pub fn demarrer_boucle_straddle(db: Arc<Database>, signal_engine: Arc<SignalEngine>) {
    tokio::spawn(async move {
        // Délai initial : laisser la DB et les bougies se charger
        sleep(Duration::from_secs(180)).await;
        loop {
            for (asset, tf) in ASSETS_STRADDLE {
                analyser_asset(&db, &signal_engine, asset, tf).await;
            }
            sleep(Duration::from_secs(15 * 60)).await;
        }
    });
    tracing::info!(
        "🌪️  Boucle Straddle auto démarrée (15 min, {} assets)",
        ASSETS_STRADDLE.len()
    );
}

async fn analyser_asset(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    asset: &Asset,
    tf: &Timeframe,
) {
    // Anti-doublon
    match db.signal_recent_existe(asset, tf, ANTI_DOUBLON_MIN).await {
        Ok(true) => return,
        Err(e) => {
            tracing::warn!(
                "Straddle auto: erreur anti-doublon {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
        Ok(false) => {}
    }

    // Bougies et indicateurs
    let bougies = match db.obtenir_bougies(asset, tf, 100).await {
        Ok(b) if b.len() >= 30 => b,
        Ok(b) => {
            tracing::debug!(
                "Straddle auto {}/{}: {} bougies insuffisantes",
                asset.as_str(),
                tf.as_str(),
                b.len()
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                "Straddle auto: DB bougies {}/{}: {}",
                asset.as_str(),
                tf.as_str(),
                e
            );
            return;
        }
    };

    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_valides: Vec<f64> = atr_vals.iter().copied().filter(|v| !v.is_nan()).collect();
    if atr_valides.len() < 2 {
        return;
    }
    let atr_actuel = *atr_valides.last().unwrap();
    let n_moy = atr_valides.len().min(14);
    let atr_moyen = atr_valides.iter().rev().take(n_moy).sum::<f64>() / n_moy as f64;
    let prix = bougies.last().map(|b| b.close).unwrap_or(0.0);
    if prix <= 0.0 || atr_actuel <= 0.0 {
        return;
    }
    let ratio_atr = atr_actuel / atr_moyen.max(f64::EPSILON);

    let now = Utc::now();
    let kill_zone = smc::kill_zone::est_en_kill_zone(now);
    let maintenant = now.timestamp();
    let dans_90min = maintenant + 5400;

    let annonces: Vec<serde_json::Value> = db
        .lire_calendrier_cache(3600)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|a| {
            a["impact"].as_str() == Some("High")
                && a["date_heure"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| {
                        let ts = dt.timestamp();
                        ts >= maintenant && ts <= dans_90min
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
        Kill Zone active: {kz} | Positions ouvertes: 0 | Drawdown: 0.0%\n",
        tf_str = tf.as_str(),
        atr = atr_actuel,
        moy = atr_moyen,
        ratio = ratio_atr,
        kz = kill_zone,
    );

    if annonces.is_empty() {
        ctx.push_str("Annonces HIGH impact < 90min: aucune\n");
    } else {
        ctx.push_str("Annonces HIGH impact < 90min:\n");
        for a in &annonces {
            let dans = a["date_heure"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| (dt.timestamp() - maintenant) / 60)
                .unwrap_or(0);
            ctx.push_str(&format!(
                "  - {} | {} | dans {}min\n",
                a["titre"].as_str().unwrap_or("?"),
                a["devise"].as_str().unwrap_or("?"),
                dans
            ));
        }
    }

    if creneaux_actifs.is_empty() {
        ctx.push_str("Créneaux historiques: aucun\n");
    } else {
        ctx.push_str("Créneaux validés:\n");
        for c in creneaux_actifs.iter().take(3) {
            ctx.push_str(&format!(
                "  {}h–{}h | ATR×{:.2} | freq {:.0}% | wr {}%\n",
                c.heure_debut,
                c.heure_fin,
                c.atr_moyen.unwrap_or(0.0),
                c.frequence.unwrap_or(0.0) * 100.0,
                c.backtest_winrate
                    .map(|w| format!("{:.0}", w))
                    .unwrap_or_else(|| "?".to_string())
            ));
        }
    }

    if let Err(e) =
        appeler_ollama_et_publier(db, signal_engine, asset, tf, prix, atr_actuel, &ctx).await
    {
        tracing::warn!("Straddle auto {}/{}: {}", asset.as_str(), tf.as_str(), e);
    }
}

async fn appeler_ollama_et_publier(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    asset: &Asset,
    tf: &Timeframe,
    prix: f64,
    atr: f64,
    ctx: &str,
) -> anyhow::Result<()> {
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let prompt = format!("{PROMPT_SIGNAL_STRADDLE}\n\n{ctx}");
    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.1, "num_predict": 300 }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let texte = client
        .post(&url)
        .json(&corps)
        .send()
        .await?
        .json::<OllamaResp>()
        .await?
        .message
        .content;

    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    let brut: ReponseLlm = match serde_json::from_str::<ReponseLlm>(&texte[debut..fin]) {
        Ok(b) => b,
        Err(_) => {
            tracing::debug!(
                "Straddle auto {}/{}: WAIT (JSON non parsable)",
                asset.as_str(),
                tf.as_str()
            );
            return Ok(());
        }
    };

    if brut.signal != "STRADDLE" || brut.score_confiance < 6.0 {
        tracing::debug!(
            "Straddle auto {}/{}: {} (score {:.1})",
            asset.as_str(),
            tf.as_str(),
            brut.signal,
            brut.score_confiance
        );
        return Ok(());
    }

    let sl_long = prix - 0.5 * atr;
    let sl_short = prix + 0.5 * atr;
    let tps_long = vec![prix + 2.0 * atr, prix + 3.5 * atr, prix + 5.0 * atr];
    let tps_short = [prix - 2.0 * atr, prix - 3.5 * atr, prix - 5.0 * atr];

    let signal = Signal::nouveau(
        asset.clone(),
        *tf,
        Direction::Both,
        brut.score_confiance * 10.0,
        prix,
        sl_long,
        tps_long,
        "Straddle",
    );

    let _ = db
        .inserer_signal_straddle_complet(&signal, sl_short, &tps_short)
        .await;
    signal_engine.publier(signal.clone());
    crate::telegram::notifier_telegram(signal);

    tracing::info!(
        "🌪️  Straddle auto signal générée {}/{} score={:.0}",
        asset.as_str(),
        tf.as_str(),
        brut.score_confiance * 10.0
    );
    Ok(())
}
