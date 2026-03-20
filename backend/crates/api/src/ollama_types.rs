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
    /// Si fourni, remplace le calcul serveur de la Kill Zone.
    pub kill_zone_active: Option<bool>,
    /// Indique si un sweep de liquidité a été détecté par le frontend.
    pub sweep_detecte: Option<bool>,
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

// ─── /api/ia/signal/straddle ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteStraddleIA {
    pub asset: String,
    pub timeframe: String,
    pub prix_actuel: f64,
    pub atr_actuel: f64,
    pub atr_moyen: f64,
    pub kill_zone_active: Option<bool>,
    pub sessions_actives: Option<Vec<String>>,
    pub annonces_imminentes: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ReponseStraddleIA {
    /// Signal stockable (Direction::Both) — None si signal = WAIT.
    pub signal: Option<common::Signal>,
    pub sl_long: f64,
    pub sl_short: f64,
    pub tp1_long: f64,
    pub tp1_short: f64,
    pub tp2_long: f64,
    pub tp2_short: f64,
    pub score_confiance: f64,
    pub declencheur: String,
    pub raisonnement: String,
    pub modele: String,
}
