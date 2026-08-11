//! Client HTTP partagé pour les services externes non-LLM (Anthropic, Telegram,
//! news, prix, IG REST…). Pour Ollama, utiliser `llm::OLLAMA_HTTP_CLIENT`
//! (timeout 300 s + sémaphore). Les timeouts spéciaux se gèrent par-requête via
//! `RequestBuilder::timeout(...)`.
use std::sync::LazyLock;
use std::time::Duration;

pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
