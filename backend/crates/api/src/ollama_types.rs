use serde::{Deserialize, Serialize};

// ─── /api/ia/analyse ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteAnalyse {
    pub asset: String,
    pub timeframe: String,
    pub direction: String,
    pub score_smc: f64,
    pub score_min: Option<f64>,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: Option<f64>,
    pub take_profit_3: Option<f64>,
    pub tendance: f64,
    pub order_block: f64,
    pub imbalance: f64,
    pub ifvg: f64,
    pub fibonacci: f64,
    pub confiance_ml: f64,
}

#[derive(Serialize)]
pub struct ReponseAnalyse {
    pub analyse: String,
    pub modele: String,
}

// ─── /api/ia/chat ─────────────────────────────────────────────────────────────

// ─── /api/ia/diagram ─────────────────────────────────────────────────────────
// ─── /api/ia/status ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StatutIA {
    pub ollama_disponible: bool,
    pub modele: String,
    pub url: String,
    /// Appels LLM passés aujourd'hui (jauge de vie du bloc Data & IA Engine).
    pub appels_jour: i64,
}

// ─── /api/ia/chart ────────────────────────────────────────────────────────────

// ─── /api/ia/signal ───────────────────────────────────────────────────────────


