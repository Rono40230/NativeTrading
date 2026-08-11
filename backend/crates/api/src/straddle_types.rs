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
    /// Heure d'entrée suggérée par le LLM (ISO 8601, ex: "2026-04-16T14:30:00Z").
    /// Optionnel — si absent, l'entrée est immédiate.
    #[serde(alias = "entry_time", alias = "entry_at", alias = "heure_entree_utc")]
    pub heure_entree_utc: Option<String>,
}
