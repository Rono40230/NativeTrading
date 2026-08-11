use common::TradingError;

use super::types::ReponseOllama;

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
