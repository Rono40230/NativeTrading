mod contexte;
mod prompts;
pub mod rockets_analyse;
pub mod rockets_filtre;
pub mod smc_analyse;
pub mod smc_filtre;
pub mod smc_confirm;
pub mod straddle_analyse;
mod types;
mod vision;

use common::TradingError;
pub use contexte::formater_contexte_backtest;
pub use contexte::formater_contexte_historique;
use prompts::SYSTEM_PROMPT;
pub use prompts::{
    PROMPT_SIGNAL_SMC, PROMPT_VISION_ANALYST, PROMPT_VISION_MULTI_TF, SYSTEM_PROMPT_COACH,
};
use types::{MessageOllama, ReponseOllama, RequeteOllama, MODELE_DEFAUT, OLLAMA_URL};

pub use types::tf_libelle;
pub use vision::{analyser_images, appeler_ollama, MODELE_VISION};
pub use smc_confirm::enrichir_signal_avec_ollama;

pub const MODELE_COACH: &str = "deepseek-r1-14b";

/// Envoie un prompt à Ollama et retourne la réponse textuelle.
pub async fn interroger(prompt: &str) -> Result<String, TradingError> {
    interroger_avec_systeme(prompt, SYSTEM_PROMPT).await
}

/// Envoie un prompt avec historique en spécifiant explicitement le modèle.
pub async fn interroger_chat_modele(
    messages: &[(String, String)],
    modele: &str,
) -> Result<String, TradingError> {
    interroger_chat_modele_avec_systeme(messages, modele, SYSTEM_PROMPT).await
}

/// Variante du chat avec un system prompt personnalisé.
pub async fn interroger_chat_modele_avec_systeme(
    messages: &[(String, String)],
    modele: &str,
    system_prompt: &str,
) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let mut msgs: Vec<serde_json::Value> =
        vec![serde_json::json!({ "role": "system", "content": system_prompt })];
    for (role, contenu) in messages {
        msgs.push(serde_json::json!({ "role": role, "content": contenu }));
    }

    let corps = serde_json::json!({ "model": modele, "messages": msgs, "stream": false });
    let texte = appeler_ollama(&url, &corps).await?;
    Ok(filtrer_think(texte))
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

/// Supprime les balises `<think>...</think>` (raisonnement interne DeepSeek-R1).
fn filtrer_think(texte: String) -> String {
    let mut resultat = texte;
    loop {
        let debut = resultat.find("<think>");
        let fin = resultat.find("</think>");
        match (debut, fin) {
            (Some(d), Some(f)) if d <= f => {
                resultat = format!("{}{}", &resultat[..d], &resultat[f + 8..].trim_start());
            }
            _ => break,
        }
    }
    resultat
}
