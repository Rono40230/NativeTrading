use common::TradingError;

use super::types::tf_libelle;
use super::types::{ReponseOllama, OLLAMA_URL};

pub const MODELE_VISION: &str = "qwen2.5vl:7b";

/// Envoie une ou plusieurs images (base64, timeframe) à llama3.2-vision — analyse SMC top-down.
pub async fn analyser_images(
    images: &[(&str, &str)],
    asset: &str,
    notes: Option<&str>,
    modele_option: Option<&str>,
) -> Result<String, TradingError> {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let prompt = if images.len() == 1 {
        crate::prompts_handler::prompt_effectif("vision_1tf")
    } else {
        crate::prompts_handler::prompt_effectif("vision_multi_tf")
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

    let modele_utilise = modele_option.unwrap_or(MODELE_VISION);
    let modele_lourd = modele_utilise.contains("32b");
    let num_ctx = if modele_lourd { 8192 } else { 32768 };
    let num_predict = if modele_lourd { 768 } else { 2048 };

    let bases64: Vec<&str> = images.iter().map(|(b, _)| *b).collect();
    let contenu_complet = format!("{}\n\n---\n\n{}", prompt, contenu);
    let corps = serde_json::json!({
        "model": modele_utilise,
        "messages": [
            {"role": "user", "content": contenu_complet, "images": bases64}
        ],
        "stream": false,
        "options": {
            "temperature": 0.2,
            "num_ctx": num_ctx,
            "num_predict": num_predict,
            "stop": ["— FIN DE L'ANALYSE —", "<|end|>", "<|im_end|>"]
        }
    });
    if modele_lourd {
        dechargement_ollama().await;
    }
    appeler_ollama(&url, &corps).await
}

/// Libère tous les modèles chargés en VRAM via l'API Ollama (keep_alive=0).
/// À appeler avant un modèle lourd (32b+) pour éviter les 503 par manque de VRAM.
async fn dechargement_ollama() {
    let base = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    // L'endpoint /api/generate avec keep_alive=0 décharge le modèle spécifié.
    // On récupère la liste des modèles actifs et on les décharge un par un.
    let client = &*super::OLLAMA_HTTP_CLIENT;
    if let Ok(r) = client.get(format!("{}/api/ps", base)).send().await {
        if let Ok(v) = r.json::<serde_json::Value>().await {
            if let Some(models) = v["models"].as_array() {
                for m in models {
                    if let Some(name) = m["name"].as_str() {
                        let _ = client
                            .post(format!("{}/api/generate", base))
                            .json(&serde_json::json!({ "model": name, "keep_alive": 0 }))
                            .send()
                            .await;
                        tracing::info!("Modèle déchargé de la VRAM: {}", name);
                    }
                }
            }
        }
    }
    // Pause pour laisser Ollama libérer la mémoire
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}

pub async fn appeler_ollama(url: &str, corps: &serde_json::Value) -> Result<String, TradingError> {
    let _permit = super::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*super::OLLAMA_HTTP_CLIENT;
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
