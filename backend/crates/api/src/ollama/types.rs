use serde::{Deserialize, Serialize};

pub const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
pub const MODELE_DEFAUT: &str = "qwen2.5:14b";

#[derive(Serialize)]
pub struct MessageOllama<'a> {
    pub role: &'a str,
    pub content: &'a str,
}

#[derive(Serialize)]
pub struct RequeteOllama<'a> {
    pub model: &'a str,
    pub messages: Vec<MessageOllama<'a>>,
    pub stream: bool,
}

#[derive(Deserialize)]
pub struct ReponseOllama {
    pub message: MessageReponse,
}

#[derive(Deserialize)]
pub struct MessageReponse {
    pub content: String,
}

pub fn tf_libelle(tf: &str) -> &str {
    match tf {
        "M1" => "1 minute",
        "M5" => "5 minutes",
        "M15" => "15 minutes",
        "H1" => "1 heure",
        "H4" => "4 heures",
        "D1" => "journalier",
        "W1" => "hebdomadaire",
        other => other,
    }
}
