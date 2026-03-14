use common::TradingError;
use serde::{Deserialize, Serialize};

const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
const MODELE_DEFAUT: &str = "qwen2.5:14b";
pub const MODELE_COACH: &str = "qwen2.5:3b";
pub const MODELE_VISION: &str = "llama3.2-vision:11b";

const PROMPT_VISION_ANALYST: &str = r#"Tu es un analyste expert en Smart Money Concepts (SMC) appliqués aux marchés financiers (crypto, forex, métaux, indices). Analyse le graphique fourni avec précision institutionnelle. Réponds TOUJOURS en français.

=== MÉTHODOLOGIE ===

1. STRUCTURE DE MARCHÉ : biais directionnel (Bullish/Bearish/Neutre), phase (Accumulation/Tendance/Distribution/Retournement), dernier BOS/ChoCH, HH/HL/LH/LL, niveaux clés.

2. LIQUIDITÉ : zones BSL (Buy-Side Liquidity) et SSL (Sell-Side Liquidity), niveaux d'inducement, sweep récent de liquidité.

3. POINTS D'INTÉRÊT (POI) : Order Blocks Demand/Supply avec force, Fair Value Gaps (FVG), Breaker Blocks, zones de réaction probables.

4. OBJECTIFS DE PRIX : cibles haussières/baissières, niveau d'invalidation.

=== FORMAT DE RÉPONSE ===

**📊 VUE D'ENSEMBLE**
Résumé en 2-3 phrases.

**🏗️ STRUCTURE DE MARCHÉ**
Biais, phase, dernier événement structurel identifié.

**💧 ANALYSE DE LIQUIDITÉ**
Zones BSL/SSL visibles, sweeps récents.

**🎯 POINTS D'INTÉRÊT**
OB, FVG, niveaux clés à surveiller.

**📈 SCÉNARIO PRINCIPAL** (probabilité estimée)

**📉 SCÉNARIO ALTERNATIF** (probabilité estimée)

**⚡ NIVEAUX À SURVEILLER**

**🔑 CONCLUSION** — Confiance /10

=== RÈGLES ===
- Base-toi UNIQUEMENT sur le graphique visible.
- Si flou ou illisible, dis-le explicitement.
- Ne fabrique pas de niveaux de prix — utilise ceux fournis dans le contexte.
- Sois précis et actionnable."#;

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

/// Envoie une image (base64) à un modèle vision avec le prompt analyste SMC complet.
pub async fn analyser_image(base64: &str, contexte: &str, notes: Option<&str>) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let mut contenu_utilisateur = format!(
        "Voici un graphique de trading.\n\nDONNÉES CONTEXTE (utilise UNIQUEMENT ces prix, ne lis pas les axes):\n{}",
        contexte
    );

    if let Some(n) = notes {
        if !n.is_empty() {
            contenu_utilisateur.push_str(&format!("\n\nNotes du trader : {}", n));
        }
    }

    let corps = serde_json::json!({
        "model": MODELE_VISION,
        "messages": [
            {"role": "system", "content": PROMPT_VISION_ANALYST},
            {"role": "user", "content": contenu_utilisateur, "images": [base64]}
        ],
        "stream": false,
        "options": {"temperature": 0.2}
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
