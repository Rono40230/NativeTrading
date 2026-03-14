use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::ollama;

// ─── POST /api/ia/analyse ─────────────────────────────────────────────────────

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

/// POST /api/ia/analyse
/// Génère une analyse narrative SMC via le LLM local Ollama.
pub async fn analyser(body: web::Json<RequeteAnalyse>) -> impl Responder {
    let rr = if body.stop_loss != 0.0 && body.prix_entree != body.stop_loss {
        let risque = (body.prix_entree - body.stop_loss).abs();
        let gain = (body.take_profit - body.prix_entree).abs();
        format!("{:.2}", gain / risque)
    } else {
        "N/A".to_string()
    };

    let prompt = format!(
        "Analyse trading SMC pour {asset} {tf}:\n\
        - Direction signal: {dir}\n\
        - Score confluence SMC: {score:.1}/100\n\
        - Prix d'entrée: {entree:.2} | Stop-Loss: {sl:.2} | Take-Profit: {tp:.2}\n\
        - Risk/Reward: {rr}\n\
        - Détail SMC: Tendance={tend:.1} OrderBlock={ob:.1} Imbalance={imb:.1} IFVG={ifvg:.1} Fibonacci={fib:.1}\n\
        - Confiance ML: {ml:.1}%\n\n\
        Fournis une analyse concise (5-8 phrases) couvrant: \
        validité du signal, points de vigilance, zones clés, et recommandation finale.",
        asset = body.asset,
        tf = body.timeframe,
        dir = body.direction,
        score = body.score_smc,
        entree = body.prix_entree,
        sl = body.stop_loss,
        tp = body.take_profit,
        rr = rr,
        tend = body.tendance,
        ob = body.order_block,
        imb = body.imbalance,
        ifvg = body.ifvg,
        fib = body.fibonacci,
        ml = body.confiance_ml * 100.0,
    );

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());

    match ollama::interroger(&prompt).await {
        Ok(texte) => HttpResponse::Ok().json(ReponseAnalyse {
            analyse: texte,
            modele,
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Lancez Ollama: ollama serve && ollama pull qwen2.5:14b"
        })),
    }
}

// ─── POST /api/ia/chat ────────────────────────────────────────────────────────

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

/// POST /api/ia/chat
/// Coach trading conversationnel — transmet l'historique à Ollama.
pub async fn chat(body: web::Json<RequeteChat>) -> impl Responder {
    if body.messages.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "messages ne peut pas être vide" }));
    }
    if body.messages.len() > 40 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Historique trop long (max 40 messages)" }));
    }

    let historique: Vec<(String, String)> = body
        .messages
        .iter()
        .map(|m| (m.role.clone(), m.contenu.clone()))
        .collect();

    match ollama::interroger_chat_modele(&historique, ollama::MODELE_COACH).await {
        Ok(reponse) => HttpResponse::Ok().json(ReponseChat {
            reponse,
            modele: ollama::MODELE_COACH.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Lancez Ollama: ollama serve && ollama pull qwen2.5:3b"
        })),
    }
}

// ─── GET /api/ia/status ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StatutIA {
    pub ollama_disponible: bool,
    pub modele: String,
    pub url: String,
}

/// GET /api/ia/status — vérifie si Ollama répond.
pub async fn statut() -> impl Responder {
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let disponible = reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    HttpResponse::Ok().json(StatutIA {
        ollama_disponible: disponible,
        modele,
        url,
    })
}

// ─── POST /api/ia/chart ─────────────────────────────────────────────────────────────────────

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

/// POST /api/ia/chart
/// Analyse visuelle d'un ou plusieurs screenshots via llama3.2-vision — top-down multi-TF.
pub async fn analyser_chart(body: web::Json<RequeteChartAnalyse>) -> impl Responder {
    if body.images.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Au moins une image requise" }));
    }

    let slices: Vec<(&str, &str)> = body.images
        .iter()
        .map(|img| (img.base64.as_str(), img.timeframe.as_str()))
        .collect();

    match ollama::analyser_images(&slices, &body.asset, body.notes.as_deref()).await {
        Ok(analyse) => HttpResponse::Ok().json(ReponseChartAnalyse {
            analyse,
            modele: ollama::MODELE_VISION.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Lancez Ollama: ollama serve && ollama pull llama3.2-vision:11b"
        })),
    }
}
