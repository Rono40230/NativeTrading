mod contexte;
mod prompts;
pub mod rockets_analyse;
pub mod rockets_filtre;
pub mod smc_analyse;
pub mod smc_filtre;
pub mod straddle_analyse;
mod types;

use common::TradingError;
pub use contexte::formater_contexte_backtest;
pub use contexte::formater_contexte_historique;
pub use prompts::PROMPT_SIGNAL_SMC;
use prompts::{PROMPT_VISION_ANALYST, PROMPT_VISION_MULTI_TF, SYSTEM_PROMPT};
use types::{MessageOllama, ReponseOllama, RequeteOllama, MODELE_DEFAUT, OLLAMA_URL};

pub use types::tf_libelle;

pub const MODELE_COACH: &str = "qwen2.5:3b";
pub const MODELE_VISION: &str = "llama3.2-vision:11b";

/// Envoie un prompt à Ollama et retourne la réponse textuelle.
pub async fn interroger(prompt: &str) -> Result<String, TradingError> {
    interroger_avec_systeme(prompt, SYSTEM_PROMPT).await
}

/// Envoie un prompt avec historique en spécifiant explicitement le modèle.
pub async fn interroger_chat_modele(
    messages: &[(String, String)],
    modele: &str,
) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let mut msgs: Vec<serde_json::Value> =
        vec![serde_json::json!({ "role": "system", "content": SYSTEM_PROMPT })];
    for (role, contenu) in messages {
        msgs.push(serde_json::json!({ "role": role, "content": contenu }));
    }

    let corps = serde_json::json!({ "model": modele, "messages": msgs, "stream": false });
    appeler_ollama(&url, &corps).await
}

/// Envoie une ou plusieurs images (base64, timeframe) à llama3.2-vision — analyse SMC top-down.
pub async fn analyser_images(
    images: &[(&str, &str)],
    asset: &str,
    notes: Option<&str>,
) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let prompt = if images.len() == 1 {
        PROMPT_VISION_ANALYST
    } else {
        PROMPT_VISION_MULTI_TF
    };

    let descriptions: Vec<String> = images
        .iter()
        .enumerate()
        .map(|(i, (_, tf))| format!("  • Image {} → {} ({})", i + 1, tf, tf_libelle(tf)))
        .collect();

    let mut contenu = format!(
        "Asset analysé : {}\nNombre de graphiques : {}\n\nTimeframes (dans l'ordre d'envoi) :\n{}",
        asset,
        images.len(),
        descriptions.join("\n"),
    );

    if images.len() > 1 {
        contenu.push_str("\n\nEffectue une analyse top-down : commence par le timeframe le plus élevé pour établir le biais directionnel, puis descends vers les TF inférieurs pour identifier le Point d'Intérêt d'entrée précis.");
    }

    if let Some(n) = notes {
        if !n.is_empty() {
            contenu.push_str(&format!("\n\nNotes du trader : {}", n));
        }
    }

    let bases64: Vec<&str> = images.iter().map(|(b, _)| *b).collect();

    // Pour les modèles vision, fusionner les instructions dans le message user
    // (avec l'image) : meilleure compréhension du contexte visuel.
    // num_ctx 32768 : prompt (~3k tokens) + image (~2k) + réponse complète (~4k).
    let contenu_complet = format!("{}\n\n---\n\n{}", prompt, contenu);
    let corps = serde_json::json!({
        "model": MODELE_VISION,
        "messages": [
            {"role": "user", "content": contenu_complet, "images": bases64}
        ],
        "stream": false,
        "options": {
            "temperature": 0.2,
            "num_ctx": 32768,
            "num_predict": 2048,
            "stop": ["— FIN DE L'ANALYSE —", "\n\n---", "<|end|>", "<|im_end|>"]
        }
    });
    appeler_ollama(&url, &corps).await
}

// ─── Fonctions privées ────────────────────────────────────────────────────────

/// Appel HTTP générique vers l'API Ollama — partagé par toutes les fonctions chat/vision.
async fn appeler_ollama(url: &str, corps: &serde_json::Value) -> Result<String, TradingError> {
    let client = reqwest::Client::new();
    let reponse = client
        .post(url)
        .json(corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama injoignable: {}", e)))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {}: vérifier que le serveur est démarré (`ollama serve`)",
            reponse.status()
        )));
    }

    let data: ReponseOllama = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Ollama invalide: {}", e)))?;

    Ok(data.message.content)
}

/// Confirmation LLM d'un signal SMC validé par `SmcDirectionalStrategy`.
///
/// Appelle Ollama avec `PROMPT_SIGNAL_SMC` + contexte chiffré du signal.
/// Retourne le raisonnement si le LLM confirme (score_confiance ≥ 0.5),
/// `None` si neutre/rejeté ou si Ollama est injoignable.
#[allow(clippy::too_many_arguments)]
pub async fn confirmer_signal_smc(
    asset: &str,
    timeframe: &str,
    score_smc: f64,
    direction: &str,
    prix_entree: f64,
    stop_loss: f64,
    take_profit: f64,
    confiance_ml: f64,
    atr: f64,
    kill_zone: bool,
    sweep: bool,
    contexte_historique: &str,
) -> Option<String> {
    let prompt = format!(
        "{contexte}{prompt}\n\nAsset: {asset} {tf}\nPrix actuel: {entree:.5} | ATR: {atr:.5}\n\
        kill_zone_active: {kz} | sweep_detecte: {sw}\n\
        Direction SMC: {dir} | Score SMC: {score:.1}/100\n\
        ML confiance: {ml:.1}% | SL: {sl:.5} | TP1: {tp:.5}",
        contexte = contexte_historique,
        prompt = PROMPT_SIGNAL_SMC,
        asset = asset,
        tf = timeframe,
        entree = prix_entree,
        atr = atr,
        kz = kill_zone,
        sw = sweep,
        dir = direction,
        score = score_smc,
        ml = confiance_ml * 100.0,
        sl = stop_loss,
        tp = take_profit,
    );
    let texte = match interroger(&prompt).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Ollama indisponible pour confirmation SMC: {}", e);
            return None;
        }
    };
    #[derive(serde::Deserialize)]
    struct ConfirmBrut {
        direction: String,
        score_confiance: f64,
        raisonnement: String,
    }
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    let Ok(brut) = serde_json::from_str::<ConfirmBrut>(&texte[debut..fin]) else {
        tracing::debug!(
            "LLM réponse non parsable: {}",
            &texte[..texte.len().min(200)]
        );
        return None;
    };
    if brut.direction == "Neutre" || brut.score_confiance < 0.5 {
        return None;
    }
    Some(brut.raisonnement)
}

async fn interroger_avec_systeme(prompt: &str, system: &str) -> Result<String, TradingError> {
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| MODELE_DEFAUT.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = RequeteOllama {
        model: &modele,
        messages: vec![
            MessageOllama {
                role: "system",
                content: system,
            },
            MessageOllama {
                role: "user",
                content: prompt,
            },
        ],
        stream: false,
    };

    let client = reqwest::Client::new();
    let reponse = client
        .post(&url)
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama injoignable: {}", e)))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {}: vérifier que le serveur est démarré (`ollama serve`)",
            reponse.status()
        )));
    }

    let data: ReponseOllama = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Ollama invalide: {}", e)))?;

    Ok(data.message.content)
}

/// Appelle Ollama pour confirmer/enrichir un signal SMC validé.
/// Timeout 45s — retourne "SMC Directionnel" si Ollama est indisponible.
pub async fn enrichir_signal_avec_ollama(
    asset: &str,
    timeframe: &str,
    signal: &strategies::Signal,
    bougies: &[common::Candle],
    contexte_historique: &str,
) -> &'static str {
    let atr_vals = indicators::calculer_atr(bougies, 14);
    let atr_val = atr_vals.last().copied().unwrap_or(0.0);
    let (score_total, kill_zone, sweep) = match smc::scorer(bougies) {
        Some(s) => (s.total, s.kill_zone_active, s.sweep_detecte),
        None => (signal.confiance * 100.0, false, false),
    };
    let dir = format!("{:?}", signal.direction);

    let confirmation = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        confirmer_signal_smc(
            asset,
            timeframe,
            score_total,
            &dir,
            signal.prix_entree,
            signal.stop_loss,
            signal.take_profit,
            signal.confiance,
            atr_val,
            kill_zone,
            sweep,
            contexte_historique,
        ),
    )
    .await;

    match confirmation {
        Ok(Some(r)) => {
            tracing::info!(
                "🤖 LLM confirmé {}/{}: {}",
                asset,
                timeframe,
                &r[..r.len().min(100)]
            );
            "SMC+IA"
        }
        Ok(None) => "SMC Directionnel",
        Err(_) => {
            tracing::warn!(
                "Timeout Ollama (45s) {}/{} — signal SMC conservé",
                asset,
                timeframe
            );
            "SMC Directionnel"
        }
    }
}
