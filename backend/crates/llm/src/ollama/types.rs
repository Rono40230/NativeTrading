use serde::Deserialize;

pub const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
/// Modèle par défaut pour tous les contextes (Straddle, chat, Rockets) — mode non-thinking.
pub const MODELE_DEFAUT: &str = "qwen3:32b";
/// Modèle dédié aux analyses SMC — même modèle, mode thinking activé (/think).
pub const MODELE_SMC: &str = "qwen3:32b";
/// Repli automatique quand le modèle demandé ne charge pas (machine 24/7 :
/// VRAM/RAM souvent insuffisantes pour le 32B — « requires more system
/// memory » → HTTP 500). Le 3B (1,9 Go) charge toujours. Décision 23/09 :
/// conviction IA et analyses straddle/rockets tombaient en échec silencieux.
pub const MODELE_REPLI: &str = "qwen2.5:3b";

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
