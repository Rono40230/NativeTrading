//! Appel Ollama + création signal Straddle.
//! Extrait de straddle_boucle pour respecter la limite 300 lignes.
use common::{Asset, Direction, Signal, Timeframe};
use db::Database;
use std::sync::Arc;
use std::time::Duration;

use crate::signal_engine::SignalEngine;
use crate::straddle_types::{OllamaResp, ReponseLlm};

pub struct ParamsOllama<'a> {
    pub prix: f64,
    pub atr: f64,
    pub ctx: &'a str,
    pub feedbacks: &'a [db::straddle_feedback::StraddleFeedbackRow],
    pub categorie: &'a crate::straddle_categorisation::CategoriePic,
    pub score_seuil: f64,
}

pub async fn appeler_ollama_et_publier(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    asset: &Asset,
    tf: &Timeframe,
    params: ParamsOllama<'_>,
) -> anyhow::Result<()> {
    let ParamsOllama { prix, atr, ctx, feedbacks, categorie, score_seuil } = params;
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let prompt = crate::straddle_prompt::construire_prompt_few_shot(
        &crate::prompts_handler::prompt_effectif("straddle_signal"),
        ctx,
        feedbacks,
        asset.as_str(),
        categorie,
    );
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

    if brut.signal != "STRADDLE" || brut.score_confiance < score_seuil {
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

    // Lier le pic ATR détecté à ce signal + charger son contexte pour le feedback
    let pic_id = if let Ok(Some(pid)) =
        db::straddle_pics::dernier_pic_asset(db.pool(), asset.as_str(), tf.as_str(), 60).await
    {
        let _ = db::straddle_pics::lier_signal(db.pool(), pid, &signal.id.to_string()).await;
        Some(pid)
    } else {
        None
    };

    // Charger le pic depuis la DB pour récupérer catégorie, session, ratio réels
    let pic = if let Some(pid) = pic_id {
        db::straddle_pics::charger_par_id(db.pool(), pid).await.unwrap_or(None)
    } else {
        None
    };

    let signal_id_str = signal.id.to_string();
    let fb = db::straddle_feedback::NouveauFeedback {
        signal_id: &signal_id_str,
        pic_id,
        asset: asset.as_str(),
        timeframe: tf.as_str(),
        timestamp_signal: signal.cree_le.timestamp(),
        categorie: pic.as_ref().map(|p| p.categorie.as_str()).unwrap_or("choc_isole"),
        evenement_nom: pic.as_ref().and_then(|p| p.evenement_nom.as_deref()),
        session_active: pic.as_ref().map(|p| p.session_active.as_str()),
        ratio_atr: pic.as_ref().map(|p| p.ratio_atr).unwrap_or(0.0),
        score_llm: brut.score_confiance,
    };
    let _ = db::straddle_feedback::inserer_feedback(db.pool(), &fb).await;

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
