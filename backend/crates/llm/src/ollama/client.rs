use common::TradingError;

use super::types::{ReponseOllama, MODELE_REPLI};

/// Appel Ollama avec repli automatique de modèle : si le modèle demandé ne
/// charge pas (machine 24/7 — le 32B exige plus de RAM/VRAM que disponible
/// → HTTP 500 « requires more system memory »), l'appel est retenté une
/// fois avec `MODELE_REPLI` (3B, charge toujours). Un appel qui demande
/// déjà le repli n'est pas retenté. Le repli est tracé : la baisse de
/// qualité est visible dans les logs, jamais silencieuse.
pub async fn appeler_ollama(url: &str, corps: &serde_json::Value) -> Result<String, TradingError> {
    match tenter(url, corps).await {
        Ok(texte) => Ok(texte),
        Err(erreur) => {
            let modele_demande = corps.get("model").and_then(|m| m.as_str()).unwrap_or("");
            if modele_demande == MODELE_REPLI {
                return Err(erreur);
            }
            tracing::warn!(
                "Ollama modèle « {modele_demande} » en échec ({erreur}) — repli sur {MODELE_REPLI}"
            );
            let mut corps_repli = corps.clone();
            corps_repli["model"] = serde_json::Value::String(MODELE_REPLI.to_string());
            tenter(url, &corps_repli).await
        }
    }
}

/// Appel brut, sans repli — le corps porte déjà le modèle voulu.
async fn tenter(url: &str, corps: &serde_json::Value) -> Result<String, TradingError> {
    super::compter_appel();
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
