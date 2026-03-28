//! Types de requête et réponse pour le signal Straddle.
//! Séparé de straddle_signal_handler.rs pour respecter la limite de 300 lignes.
use serde::Deserialize;

// ── Requête ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteStraddleSignal {
    pub asset: String,
    pub timeframe: String,
    pub prix_actuel: f64,
    pub atr_actuel: f64,
    pub atr_moyen_14: f64,
    pub kill_zone_active: Option<bool>,
    pub positions_actives: Option<i64>,
    pub drawdown_actuel_pct: Option<f64>,
}

// ── Réponse brute LLM ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ReponseLlm {
    pub signal: String, // "STRADDLE" | "WAIT"
    #[serde(alias = "trigger")]
    pub declencheur: Option<String>,
    #[serde(alias = "reason", alias = "reson", alias = "message")]
    pub raison: String,
    #[serde(
        alias = "confidence_score",
        alias = "confidence",
        alias = "score",
        alias = "score_confidence"
    )]
    pub score_confiance: f64,
    #[serde(alias = "expected_amplitude_pct", alias = "amplitude_pct")]
    pub amplitude_attendue_pct: Option<f64>,
    #[serde(
        alias = "estimated_exposure_min",
        alias = "duration_min",
        alias = "duration"
    )]
    pub duree_exposition_estimee_min: Option<i64>,
}

// ── Réponses intermédiaires Ollama ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct OllamaResp {
    pub message: OllamaMsg,
}

#[derive(Deserialize)]
pub struct OllamaMsg {
    pub content: String,
}
