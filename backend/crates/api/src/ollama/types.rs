use serde::Deserialize;

pub const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
pub const MODELE_DEFAUT: &str = "qwen2.5vl:7b";

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
