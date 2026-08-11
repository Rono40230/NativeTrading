//! Crate `llm` — intégration LLM (Ollama + Anthropic). Extrait du monolithe `api`
//! (phase 1.6b). Cluster fermé : aucune dépendance vers le métier api, aucun cycle.
//!
//! `prompt_effectif` (lecture fichier pure sur `data/prompts_overrides.json`) a été
//! déplacée ici depuis `api::prompts_handler` pour découpler le cycle
//! `anthropic → prompts_handler` : `anthropic` appelle désormais `crate::prompt_effectif`
//! (intra-llm), et `api::prompts_handler` consomme `llm::prompt_effectif`.
pub mod anthropic;
pub mod ollama;
pub mod prompts;

// Re-exports pour une API publique ergonomique : `llm::OLLAMA_HTTP_CLIENT`,
// `llm::filtrer_think`, `llm::ReponseOllama`, `llm::smc_filtre::SignalSMCCandidat`, etc.
// L'item explicite `pub mod prompts` (root) prime sur le glob `ollama::prompts`.
pub use ollama::*;
pub use prompts::{
    charger_overrides, defaults, prompt_effectif, sauvegarder_overrides, PROMPT_SIGNAL_STRADDLE,
};

use std::sync::LazyLock;

/// Client HTTP partagé du crate `llm` pour les appels non-Ollama (Anthropic, …).
/// Recréé localement (timeout 30 s) pour éviter une dépendance `llm → api` vers
/// l'ancien `api::http_client::HTTP_CLIENT`. Les timeouts spéciaux (60 s / 180 s)
/// se gèrent par-requête via `RequestBuilder::timeout(...)`.
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
