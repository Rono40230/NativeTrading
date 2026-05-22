mod contexte;
pub mod diagram_templates;
pub mod prompts;
pub mod rockets_analyse;
pub mod rockets_contexte;
pub mod rockets_filtre;
pub mod smc_analyse;
pub mod smc_confirm;
pub mod smc_filtre;
pub mod straddle_analyse;
mod types;
mod vision;

use common::TradingError;
use std::sync::LazyLock;
pub use contexte::formater_contexte_historique;
use prompts::SYSTEM_PROMPT;
pub use prompts::{
    PROMPT_FILTRE_ROCKET, PROMPT_SIGNAL_SMC, SYSTEM_PROMPT_COACH, SYSTEM_PROMPT_COACH_DIAGRAM,
    SYSTEM_PROMPT_COACH_OLLAMA,
};
use types::{ReponseOllama, MODELE_DEFAUT, OLLAMA_URL};

/// Sémaphore global Ollama : max 2 appels LLM concurrents (évite la saturation VRAM/swap modèle).
pub static OLLAMA_SEMAPHORE: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::new(2));

/// Client HTTP partagé pour tous les appels Ollama (timeout 300s, pas de reconstruction par appel).
pub static OLLAMA_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

pub use smc_confirm::enrichir_signal_avec_ollama;
pub use types::tf_libelle;
pub use vision::appeler_ollama;

pub const MODELE_COACH: &str = "qwen2.5vl:7b";
pub const MODELE_COACH_DIAGRAM: &str = "qwen2.5-coder:14b";
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

    let corps = serde_json::json!({
        "model": modele,
        "messages": msgs,
        "stream": false,
        "options": { "num_gpu": 99, "num_ctx": 16384 }
    });
    let texte = appeler_ollama(&url, &corps).await?;
    Ok(filtrer_think(texte))
}

async fn interroger_avec_systeme(prompt: &str, system: &str) -> Result<String, TradingError> {
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| MODELE_DEFAUT.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": prompt }
        ],
        "stream": false,
        "options": { "num_gpu": 99, "num_ctx": 8192 }
    });

    let _permit = OLLAMA_SEMAPHORE.acquire().await.ok();
    let reponse = OLLAMA_HTTP_CLIENT
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
