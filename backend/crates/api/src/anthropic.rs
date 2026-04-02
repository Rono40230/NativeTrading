use common::TradingError;

pub const MODELE_CLAUDE: &str = "claude-sonnet-4-5";
pub const MODELE_CLAUDE_VISION: &str = "claude-sonnet-4-5";

/// Analyse visuelle d'un ou plusieurs screenshots via Claude Sonnet (Anthropic API).
///
/// Les images sont encodées en base64 et envoyées directement à l'API Anthropic.
/// La clé API est lue depuis la DB — jamais exposée au frontend.
pub async fn analyser_images_claude(
    images: &[(&str, &str)],
    asset: &str,
    notes: Option<&str>,
    api_key: &str,
) -> Result<String, TradingError> {
    use crate::ollama::tf_libelle;
    use crate::ollama::{PROMPT_VISION_ANALYST, PROMPT_VISION_MULTI_TF};

    let system_prompt = if images.len() == 1 {
        PROMPT_VISION_ANALYST
    } else {
        PROMPT_VISION_MULTI_TF
    };

    // Construction du contenu utilisateur : images + texte contextuel
    let mut content: Vec<serde_json::Value> = Vec::new();

    for (base64, tf) in images {
        // Détecter le media type depuis le préfixe base64 data URL, ou supposer JPEG
        // Le frontend envoie soit un data URL complet (data:image/...;base64,XXX)
        // soit du base64 pur — on gère les deux cas
        let (media_type, data_str);
        if let Some(comma) = base64.find(',') {
            let header = &base64[..comma];
            media_type = if header.contains("image/png") {
                "image/png"
            } else if header.contains("image/webp") {
                "image/webp"
            } else if header.contains("image/gif") {
                "image/gif"
            } else {
                "image/jpeg"
            };
            data_str = base64[comma + 1..].to_string();
        } else {
            media_type = "image/jpeg";
            data_str = base64.to_string();
        }
        let data = data_str.as_str();

        content.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": data
            }
        }));

        content.push(serde_json::json!({
            "type": "text",
            "text": format!("Timeframe de cette image : {} ({})", tf, tf_libelle(tf))
        }));
    }

    let descriptions: Vec<String> = images
        .iter()
        .enumerate()
        .map(|(i, (_, tf))| format!("  • Image {} → {} ({})", i + 1, tf, tf_libelle(tf)))
        .collect();

    let mut texte_contexte = format!(
        "Asset analysé : {}\nNombre de graphiques : {}\n\nTimeframes :\n{}",
        asset,
        images.len(),
        descriptions.join("\n")
    );

    if images.len() > 1 {
        texte_contexte.push_str(
            "\n\nEffectue une analyse top-down : commence par le TF le plus élevé \
            pour établir le biais, puis descends vers les TF inférieurs pour le POI d'entrée.",
        );
    }

    if let Some(n) = notes {
        if !n.is_empty() {
            texte_contexte.push_str(&format!("\n\nNotes du trader : {}", n));
        }
    }

    content.push(serde_json::json!({
        "type": "text",
        "text": texte_contexte
    }));

    let corps = serde_json::json!({
        "model": MODELE_CLAUDE_VISION,
        "max_tokens": 4096,
        "system": [
            {
                "type": "text",
                "text": system_prompt,
                "cache_control": { "type": "ephemeral" }
            }
        ],
        "messages": [
            { "role": "user", "content": content }
        ]
    });

    let debut = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| TradingError::Api(format!("Anthropic client error: {}", e)))?;

    let reponse = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "prompt-caching-2024-07-31")
        .header("content-type", "application/json")
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Anthropic injoignable: {}", e)))?;

    let status = reponse.status();

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(TradingError::Api(
            "Clé API Anthropic invalide — vérifiez dans Paramètres".to_string(),
        ));
    }

    if !status.is_success() {
        let body = reponse
            .text()
            .await
            .unwrap_or_else(|_| "réponse illisible".to_string());
        if body.contains("credit_balance") || body.contains("credit balance") {
            return Err(TradingError::Api(
                "Crédits Anthropic insuffisants — rechargez votre compte sur console.anthropic.com"
                    .to_string(),
            ));
        }
        return Err(TradingError::Api(format!(
            "Anthropic HTTP {}: {}",
            status, body
        )));
    }

    let data: serde_json::Value = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Anthropic invalide: {}", e)))?;

    let duree = debut.elapsed();
    tracing::info!("Anthropic chart analyse: {:?}", duree);
    if duree > std::time::Duration::from_millis(200) {
        tracing::warn!("Anthropic latence élevée: {:?}", duree);
    }

    let texte = data["content"]
        .as_array()
        .and_then(|arr| arr.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str())
        .unwrap_or("")
        .to_string();

    if texte.is_empty() {
        return Err(TradingError::Api(
            "Réponse vide de l'API Anthropic".to_string(),
        ));
    }

    Ok(texte)
}

/// Coach trading conversationnel via Claude Sonnet (Anthropic API).
/// Reçoit un historique de messages (role, contenu) et un system prompt, retourne la réponse.
pub async fn chat_claude(
    messages: &[(String, String)],
    system_prompt: &str,
    api_key: &str,
) -> Result<String, TradingError> {
    let msgs: Vec<serde_json::Value> = messages
        .iter()
        .map(|(role, contenu)| serde_json::json!({ "role": role, "content": contenu }))
        .collect();

    let corps = serde_json::json!({
        "model": MODELE_CLAUDE,
        "max_tokens": 4096,
        "system": [
            {
                "type": "text",
                "text": system_prompt,
                "cache_control": { "type": "ephemeral" }
            }
        ],
        "messages": msgs
    });

    let debut = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| TradingError::Api(format!("Anthropic client error: {}", e)))?;

    let reponse = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "prompt-caching-2024-07-31")
        .header("content-type", "application/json")
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Anthropic injoignable: {}", e)))?;

    let status = reponse.status();

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(TradingError::Api(
            "Clé API Anthropic invalide — vérifiez dans Paramètres".to_string(),
        ));
    }

    if !status.is_success() {
        let body = reponse
            .text()
            .await
            .unwrap_or_else(|_| "réponse illisible".to_string());
        // Détecter crédit insuffisant
        if body.contains("credit_balance") || body.contains("credit balance") {
            return Err(TradingError::Api(
                "Crédits Anthropic insuffisants — rechargez votre compte sur console.anthropic.com"
                    .to_string(),
            ));
        }
        return Err(TradingError::Api(format!(
            "Anthropic HTTP {}: {}",
            status, body
        )));
    }

    let data: serde_json::Value = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Anthropic invalide: {}", e)))?;

    let duree = debut.elapsed();
    tracing::info!("Anthropic coach: {:?}", duree);

    let texte = data["content"]
        .as_array()
        .and_then(|arr| arr.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str())
        .unwrap_or("")
        .to_string();

    if texte.is_empty() {
        return Err(TradingError::Api(
            "Réponse vide de l'API Anthropic".to_string(),
        ));
    }

    Ok(texte)
}
