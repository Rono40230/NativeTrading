use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use llm::ollama;
use crate::ollama_types::{ReponseSignalIA, RequeteSignalIA};
use crate::state::AppState;

// ─── POST /api/ia/signal ─────────────────────────────────────────────────────
#[derive(Deserialize)]
struct SignalBrut {
    direction: String,
    prix_entree: f64,
    stop_loss: f64,
    tp1: f64,
    tp2: f64,
    tp3: f64,
    score_confiance: f64,
    niveau_invalidation: f64,
    confluences: Vec<String>,
    raisonnement: String,
}

