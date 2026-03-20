use serde::{Deserialize, Serialize};

// ─── /api/ia/analyse ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteAnalyse {
    pub asset: String,
    pub timeframe: String,
    pub direction: String,
    pub score_smc: f64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
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

#[derive(Deserialize)]
pub struct MessageChat {
    pub role: String,
    pub contenu: String,
}

#[derive(Deserialize)]
pub struct RequeteChat {
    pub messages: Vec<MessageChat>,
}

#[derive(Serialize)]
pub struct ReponseChat {
    pub reponse: String,
    pub modele: String,
}

// ─── /api/ia/status ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StatutIA {
    pub ollama_disponible: bool,
    pub modele: String,
    pub url: String,
}

// ─── /api/ia/chart ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ImageAvecTF {
    pub base64: String,
    pub timeframe: String,
}

#[derive(Deserialize)]
pub struct RequeteChartAnalyse {
    pub asset: String,
    pub images: Vec<ImageAvecTF>,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct ReponseChartAnalyse {
    pub analyse: String,
    pub modele: String,
}

// ─── /api/ia/signal ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteSignalIA {
    pub asset: String,
    pub timeframe: String,
    pub score_smc: f64,
    pub prix_actuel: f64,
    pub tendance: f64,
    pub order_block: f64,
    pub imbalance: f64,
    pub ifvg: f64,
    pub fibonacci: f64,
    pub confiance_ml: f64,
    pub atr: f64,
}

#[derive(Serialize)]
pub struct ReponseSignalIA {
    pub signal: Option<common::Signal>,
    pub score_confiance: f64,
    pub niveau_invalidation: f64,
    pub confluences: Vec<String>,
    pub raisonnement: String,
    pub modele: String,
}
