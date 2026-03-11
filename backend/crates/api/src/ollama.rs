use common::TradingError;
use serde::{Deserialize, Serialize};

const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
const MODELE_DEFAUT: &str = "qwen2.5:14b";
pub const MODELE_COACH: &str = "qwen2.5:3b";
pub const MODELE_VISION: &str = "llava";

/// Prompt système injecté dans chaque conversation trading
const SYSTEM_PROMPT: &str = "Tu es un expert en trading algorithmique spécialisé \
dans l'analyse SMC (Smart Money Concept). Tu analyses des données de marché \
(crypto et métaux) et fournis des explications claires, concises et actionnables. \
Réponds toujours en français. Sois précis sur les niveaux de prix et les risques.";

#[derive(Serialize)]
struct MessageOllama<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct RequeteOllama<'a> {
    model: &'a str,
    messages: Vec<MessageOllama<'a>>,
    stream: bool,
}

#[derive(Deserialize)]
struct ReponseOllama {
    message: MessageReponse,
}

#[derive(Deserialize)]
struct MessageReponse {
    content: String,
}

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

/// Envoie une image (base64) à un modèle vision (llava) avec un contexte textuel.
pub async fn analyser_image(base64: &str, contexte: &str) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    // On fournit les prix réels au modèle — il ne doit PAS lire les axes lui-même
    let prompt = format!(
        "Voici un graphique de trading en chandeliers japonais.\n\
        DONNÉES RÉELLES (utilise UNIQUEMENT ces valeurs, ne lis pas les axes):\n\
        {}\n\n\
        Analyse uniquement ce que tu vois visuellement dans l'image (forme des bougies, \
        patterns, structure des hauts/bas, tendance générale de gauche à droite).\n\
        Réponds STRICTEMENT avec cette structure:\n\n\
        **TENDANCE**: HAUSSIÈRE / BAISSIÈRE / LATÉRALE — décris le mouvement global visible\n\
        **STRUCTURE**: patterns de bougies visibles (marteaux, étoiles filantes, englobantes, doji, etc.)\n\
        **SUPPORTS/RÉSISTANCES**: zones de prix où le cours a rebondi (utilise les prix réels fournis)\n\
        **MOMENTUM**: le mouvement récent (dernières bougies à droite) accélère ou ralentit ?\n\
        **RECOMMANDATION**: LONG / SHORT / NEUTRE — avec raison précise basée sur les patterns visibles",
        contexte
    );

    // Format natif Ollama vision : images = tableau de base64 brut (sans préfixe data:)
    let corps = serde_json::json!({
        "model": MODELE_VISION,
        "messages": [{
            "role": "user",
            "content": prompt,
            "images": [base64]
        }],
        "stream": false
    });
    appeler_ollama(&url, &corps).await
}

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
