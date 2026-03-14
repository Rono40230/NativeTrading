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

const PROMPT_VISION_MULTI_TF: &str = r#"Tu es un analyste expert en Smart Money Concepts (SMC) spécialisé en analyse top-down multi-timeframe. Tu reçois plusieurs graphiques du même asset sur des timeframes différents. Réponds TOUJOURS en français.

=== MÉTHODOLOGIE TOP-DOWN ===

Analyse dans l'ordre chronologique des TF : HTF (biais directionnel) → ITF (structure) → LTF (entrée précise).

Pour CHAQUE graphique reçu :
1. STRUCTURE DE MARCHÉ : biais directionnel, phase, BOS/ChoCH, HH/HL/LH/LL
2. LIQUIDITÉ : zones BSL/SSL, sweeps récents, inducements
3. POI : Order Blocks, FVG, niveaux clés de réaction

Puis une synthèse de confluence.

=== FORMAT DE RÉPONSE ===

**(répété pour chaque TF) 🔭 ANALYSE [TIMEFRAME]**
Structure, liquidité, POI.

**🔗 CONFLUENCE MULTI-TF**
Alignement des biais, zones de confluence, invalidation.

**🎯 SCÉNARIO OPTIMAL**
Entry zone, SL logique, objectifs TP1/TP2/TP3.

**⚡ SCÉNARIO ALTERNATIF**

**🔑 CONCLUSION** — Confiance /10

=== RÈGLES ===
- Chaque image correspond à un timeframe différent du même asset.
- Base-toi UNIQUEMENT sur les graphiques visibles.
- Ne fabrique pas de niveaux — utilise ceux fournis dans le contexte.
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

fn tf_libelle(tf: &str) -> &str {
    match tf {
        "M1"  => "1 minute",
        "M5"  => "5 minutes",
        "M15" => "15 minutes",
        "H1"  => "1 heure",
        "H4"  => "4 heures",
        "D1"  => "journalier",
        "W1"  => "hebdomadaire",
        other => other,
    }
}

/// Envoie une ou plusieurs images (base64, timeframe) à llama3.2-vision — analyse SMC top-down.
pub async fn analyser_images(images: &[(&str, &str)], asset: &str, notes: Option<&str>) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let prompt = if images.len() == 1 { PROMPT_VISION_ANALYST } else { PROMPT_VISION_MULTI_TF };

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

    let corps = serde_json::json!({
        "model": MODELE_VISION,
        "messages": [
            {"role": "system", "content": prompt},
            {"role": "user", "content": contenu, "images": bases64}
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
