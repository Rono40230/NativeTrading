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
pub struct RequeteChartAnalyse {
    pub asset: String,
    pub timeframe: String,
    pub image_base64: String,
    pub prix_actuel: Option<f64>,
    pub plus_haut: Option<f64>,
    pub plus_bas: Option<f64>,
    pub volume_moyen: Option<f64>,
    pub nb_bougies: Option<u32>,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct ReponseChartAnalyse {
    pub analyse: String,
    pub modele: String,
}

/// POST /api/ia/chart
/// Analyse visuelle d'un screenshot de graphique via le modèle vision llava.
pub async fn analyser_chart(body: web::Json<RequeteChartAnalyse>) -> impl Responder {
    if body.image_base64.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "image_base64 ne peut pas être vide" }));
    }

    let timeframe_libelle = match body.timeframe.as_str() {
        "M1"  => "bougies de 1 minute",
        "M5"  => "bougies de 5 minutes",
        "M15" => "bougies de 15 minutes",
        "H1"  => "bougies de 1 heure",
        "H4"  => "bougies de 4 heures",
        "D1"  => "bougies journalières",
        "W1"  => "bougies hebdomadaires",
        tf    => tf,
    };

    let mut contexte = format!(
        "Asset: {} | Timeframe: {} ({}) | Nombre de bougies visibles: {}",
        body.asset,
        body.timeframe,
        timeframe_libelle,
        body.nb_bougies.unwrap_or(0),
    );

    if let (Some(actuel), Some(haut), Some(bas)) =
        (body.prix_actuel, body.plus_haut, body.plus_bas)
    {
        contexte.push_str(&format!(
            " | Prix actuel: ${:.2} | Plus haut session: ${:.2} | Plus bas session: ${:.2}",
            actuel, haut, bas
        ));
    }
    if let Some(vol) = body.volume_moyen {
        contexte.push_str(&format!(" | Volume moyen par bougie: {:.2}", vol));
    }

    match ollama::analyser_image(&body.image_base64, &contexte, body.notes.as_deref()).await {
        Ok(analyse) => HttpResponse::Ok().json(ReponseChartAnalyse {
            analyse,
            modele: ollama::MODELE_VISION.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Lancez Ollama: ollama serve && ollama pull llava"
        })),
    }
}
