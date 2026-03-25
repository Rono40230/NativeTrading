use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::ollama;
use crate::ollama_types::{
    ReponseAnalyse, ReponseChartAnalyse, ReponseChat, ReponseSignalIA, RequeteAnalyse,
    RequeteChartAnalyse, RequeteChat, RequeteSignalIA, StatutIA,
};
use crate::state::AppState;

// ─── POST /api/ia/analyse ─────────────────────────────────────────────────────
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
        "Tu es un trader institutionnel SMC/ICT expert. Analyse ce signal candidat avec rigueur.\n\n\
        ## SIGNAL CANDIDAT : {asset} {tf}\n\
        - Direction : {dir}\n\
        - Score confluence SMC : {score:.1}/100\n\
        - Prix entrée : {entree:.2} | Stop-Loss : {sl:.2} | Take-Profit : {tp:.2}\n\
        - Risk/Reward : {rr}\n\
        - Détail SMC : Tendance={tend:.1}/30 OB={ob:.1}/20 Imbalance={imb:.1}/15 IFVG={ifvg:.1}/10 Fibonacci={fib:.1}/5\n\
        - Confiance ML : {ml:.1}%\n\n\
        ## CRITÈRES À VÉRIFIER\n\
        1. Kill Zone active ? (London 07h-10h / NY 13h30-16h30 UTC) — BLOQUANT si absent\n\
        2. Sweep de liquidité confirmé ? — BLOQUANT si absent\n\
        3. Order Block non mitigé aligné avec la direction ?\n\
        4. R:R ≥ 2:1 minimum ? (calculé : {rr})\n\
        5. Score SMC ≥ 60/100 et ML ≥ 60% ? (actuels : {score:.0} / {ml:.0}%)\n\n\
        ## PHILOSOPHIE : QUALITÉ > QUANTITÉ\n\
        Il vaut mieux passer ce signal que le valider sans confluence suffisante.\n\n\
        Fournis une analyse en 5-7 phrases couvrant dans l'ordre :\n\
        1. Verdict (Valide / À éviter) et raison principale\n\
        2. Points forts du setup (confluences présentes)\n\
        3. Points faibles ou risques (critères manquants, zone dangereuse)\n\
        4. Niveau d'invalidation clé à surveiller\n\
        5. Recommandation finale (trader, patienter, ou rejeter)",
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
/// POST /api/ia/chart
/// Analyse visuelle d'un ou plusieurs screenshots via llama3.2-vision — top-down multi-TF.
pub async fn analyser_chart(body: web::Json<RequeteChartAnalyse>) -> impl Responder {
    if body.images.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Au moins une image requise" }));
    }

    let slices: Vec<(&str, &str)> = body
        .images
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

/// POST /api/ia/signal — Signal SMC structuré (JSON) via LLM Ollama.
pub async fn generer_signal(
    state: web::Data<AppState>,
    body: web::Json<RequeteSignalIA>,
) -> impl Responder {
    use common::{Asset, Direction, Signal, Timeframe};

    // Contexte historique pour nourrir le LLM
    let historique_raw = state.db.obtenir_contexte_llm(&body.asset, 5).await;
    let contexte = crate::ollama::formater_contexte_historique(
        &body.asset,
        "SMC Directionnel",
        &historique_raw,
    );

    let prompt = format!(
        "{contexte}{base}\n\nAsset: {asset} {tf}\nPrix actuel: {prix:.5} | ATR: {atr:.5}\n\
        kill_zone_active: {kz} | sweep_detecte: {sw}\n\
        SMC: Tendance={tend:.1} OB={ob:.1} Imbalance={imb:.1} IFVG={ifvg:.1} Fib={fib:.1}\n\
        ML confiance={ml:.1}% | Score SMC total={score:.1}/100",
        contexte = contexte,
        base = crate::ollama::PROMPT_SIGNAL_SMC,
        asset = body.asset,
        tf = body.timeframe,
        prix = body.prix_actuel,
        atr = body.atr,
        kz = body
            .kill_zone_active
            .unwrap_or_else(|| smc::kill_zone::est_en_kill_zone(chrono::Utc::now())),
        sw = body.sweep_detecte.unwrap_or(false),
        tend = body.tendance,
        ob = body.order_block,
        imb = body.imbalance,
        ifvg = body.ifvg,
        fib = body.fibonacci,
        ml = body.confiance_ml * 100.0,
        score = body.score_smc,
    );
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());
    match ollama::interroger(&prompt).await {
        Err(e) => {
            HttpResponse::ServiceUnavailable().json(serde_json::json!({ "error": format!("{e}") }))
        }
        Ok(texte) => {
            let debut: usize = texte.find('{').unwrap_or(0);
            let fin: usize = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
            let Ok(brut) = serde_json::from_str::<SignalBrut>(&texte[debut..fin]) else {
                return HttpResponse::UnprocessableEntity().json(serde_json::json!({
                    "error": "Réponse LLM non parsable en JSON", "brut": texte
                }));
            };
            let signal = if brut.direction != "Neutre" {
                let asset = crate::utils::parse_asset(&body.asset).unwrap_or(Asset::BTC);
                let tf = match body.timeframe.as_str() {
                    "M1" => Timeframe::M1,
                    "M5" => Timeframe::M5,
                    "H1" => Timeframe::H1,
                    "H4" => Timeframe::H4,
                    "D1" => Timeframe::D1,
                    "W1" => Timeframe::W1,
                    _ => Timeframe::M15,
                };
                let dir = if brut.direction == "Short" {
                    Direction::Short
                } else {
                    Direction::Long
                };
                Some(Signal::nouveau(
                    asset,
                    tf,
                    dir,
                    brut.score_confiance * 10.0,
                    brut.prix_entree,
                    brut.stop_loss,
                    vec![brut.tp1, brut.tp2, brut.tp3],
                    "SMC-IA",
                ))
            } else {
                None
            };
            HttpResponse::Ok().json(ReponseSignalIA {
                signal,
                score_confiance: brut.score_confiance,
                niveau_invalidation: brut.niveau_invalidation,
                confluences: brut.confluences,
                raisonnement: brut.raisonnement,
                modele,
            })
        }
    }
}
