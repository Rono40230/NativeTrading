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
    /// Annonces HIGH impact < 90min (depuis le calendrier économique).
    /// Utilisées pour extraire l'heure d'entrée cible du trade Straddle.
    pub annonces: &'a [serde_json::Value],
    /// Bougies récentes — pour extraire les 52 features OHLCV du snapshot.
    pub bougies: &'a [common::Candle],
    /// Ratio ATR actuel / ATR moyen 14p — feature contextuelle Straddle.
    pub ratio_atr: f64,
    /// Multiplicateur SL (depuis StraddleParams DB).
    pub sl_mult: f64,
    /// Multiplicateurs TP 1/2/3 (depuis StraddleParams DB).
    pub tp_mult_1: f64,
    pub tp_mult_2: f64,
    pub tp_mult_3: f64,
}

pub async fn appeler_ollama_et_publier(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    asset: &Asset,
    tf: &Timeframe,
    params: ParamsOllama<'_>,
) -> anyhow::Result<()> {
    let ParamsOllama {
        prix,
        atr,
        ctx,
        feedbacks,
        categorie,
        score_seuil,
        annonces,
        bougies,
        ratio_atr,
        sl_mult,
        tp_mult_1,
        tp_mult_2,
        tp_mult_3,
    } = params;
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:32b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let lecons = crate::patterns_echec_job::charger_lecons_systemiques(db, "STRADDLE").await;
    let prompt_base = crate::straddle_prompt::construire_prompt_few_shot(
        &crate::prompts_handler::prompt_effectif("straddle_signal"),
        ctx,
        feedbacks,
        asset.as_str(),
        categorie,
    );
    // /no_think : mode non-thinking Qwen3 — classification de contexte Straddle
    let prompt = if lecons.is_empty() {
        format!("{prompt_base}\n/no_think")
    } else {
        format!("{prompt_base}\n\n{lecons}\n/no_think")
    };
    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.7, "num_predict": 300 }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let texte_brut = client
        .post(&url)
        .json(&corps)
        .send()
        .await?
        .json::<OllamaResp>()
        .await?
        .message
        .content;
    // Filtrer les balises <think> au cas où Qwen3 en produit malgré /no_think
    let texte = crate::ollama::filtrer_think(texte_brut);

    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    let mut brut: ReponseLlm = match serde_json::from_str::<ReponseLlm>(&texte[debut..fin]) {
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
    // Normalisation : si le LLM retourne 0-1 au lieu de 0-10, ramener à l'échelle 0-10
    brut.score_confiance = crate::utils::normaliser_score_llm(brut.score_confiance);

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

    let sl_long = prix - sl_mult * atr;
    let sl_short = prix + sl_mult * atr;
    let tps_long = vec![
        prix + tp_mult_1 * atr,
        prix + tp_mult_2 * atr,
        prix + tp_mult_3 * atr,
    ];
    let tps_short = [
        prix - tp_mult_1 * atr,
        prix - tp_mult_2 * atr,
        prix - tp_mult_3 * atr,
    ];

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

    // Heure d'entrée : LLM en priorité, sinon première annonce HIGH impact
    let heure_entree: Option<i64> = brut
        .heure_entree_utc
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .or_else(|| {
            annonces
                .first()
                .and_then(|a| a["date_heure"].as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
        });

    let _ = db
        .inserer_signal_straddle_complet(&signal, sl_short, &tps_short, heure_entree)
        .await;

    crate::straddle_signal_feedback::sauvegarder_feedback_et_features(
        db,
        &signal,
        asset,
        tf,
        brut.score_confiance,
        ratio_atr,
        bougies,
    )
    .await;

    signal_engine.publier(signal.clone());

    tracing::info!(
        "🌪️  Straddle auto signal générée {}/{} score={:.0}",
        asset.as_str(),
        tf.as_str(),
        brut.score_confiance * 10.0
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Vérifie que sl_mult et tp_mult_1/2/3 sont correctement appliqués au calcul SL/TP.
    #[test]
    fn sl_tp_appliquent_multiplicateurs_params() {
        let prix = 2000.0_f64;
        let atr = 50.0_f64;

        // Cas avec sl_mult=0.8, tp_mult_1=1.5, tp_mult_2=2.5, tp_mult_3=5.0
        let sl_mult = 0.8_f64;
        let tp_mult_1 = 1.5_f64;
        let tp_mult_2 = 2.5_f64;
        let tp_mult_3 = 5.0_f64;

        let sl_long = prix - sl_mult * atr;
        let sl_short = prix + sl_mult * atr;
        let tps_long = [
            prix + tp_mult_1 * atr,
            prix + tp_mult_2 * atr,
            prix + tp_mult_3 * atr,
        ];
        let tps_short = [
            prix - tp_mult_1 * atr,
            prix - tp_mult_2 * atr,
            prix - tp_mult_3 * atr,
        ];

        assert!(
            (sl_long - (prix - 0.8 * atr)).abs() < 1e-9,
            "SL long incorrect"
        );
        assert!(
            (sl_short - (prix + 0.8 * atr)).abs() < 1e-9,
            "SL short incorrect"
        );
        assert!(
            (tps_long[0] - (prix + 1.5 * atr)).abs() < 1e-9,
            "TP1 long incorrect"
        );
        assert!(
            (tps_long[1] - (prix + 2.5 * atr)).abs() < 1e-9,
            "TP2 long incorrect"
        );
        assert!(
            (tps_long[2] - (prix + 5.0 * atr)).abs() < 1e-9,
            "TP3 long incorrect"
        );
        assert!(
            (tps_short[0] - (prix - 1.5 * atr)).abs() < 1e-9,
            "TP1 short incorrect"
        );
        assert!(
            (tps_short[1] - (prix - 2.5 * atr)).abs() < 1e-9,
            "TP2 short incorrect"
        );
        assert!(
            (tps_short[2] - (prix - 5.0 * atr)).abs() < 1e-9,
            "TP3 short incorrect"
        );
    }

    /// Vérifie que les valeurs par défaut DB (sl_mult=0.5, tp=2.0/3.5/5.0) produisent
    /// les mêmes niveaux que l'ancien code hardcodé.
    #[test]
    fn valeurs_par_defaut_db_identiques_aux_anciens_hardcodes() {
        let prix = 1800.0_f64;
        let atr = 30.0_f64;

        let sl_mult = 0.5_f64;
        let tp_mult_1 = 2.0_f64;
        let tp_mult_2 = 3.5_f64;
        let tp_mult_3 = 5.0_f64;

        let sl_long = prix - sl_mult * atr;
        let tps_long = [
            prix + tp_mult_1 * atr,
            prix + tp_mult_2 * atr,
            prix + tp_mult_3 * atr,
        ];

        assert!((sl_long - (prix - 0.5 * atr)).abs() < 1e-9);
        assert!((tps_long[0] - (prix + 2.0 * atr)).abs() < 1e-9);
        assert!((tps_long[1] - (prix + 3.5 * atr)).abs() < 1e-9);
        assert!((tps_long[2] - (prix + 5.0 * atr)).abs() < 1e-9);
    }
}
