use actix_web::{web, HttpResponse, Responder};
use smc::{
    SCORE_MAX_FIBONACCI, SCORE_MAX_IFVG, SCORE_MAX_IMBALANCE, SCORE_MAX_ORDER_BLOCK,
    SCORE_MAX_TENDANCE,
};

use crate::ollama;
use crate::ollama_types::{
    ReponseAnalyse, ReponseChat, RequeteAnalyse, RequeteChat, RequeteDiagram, StatutIA,
};
use crate::state::AppState;

// ─── POST /api/ia/analyse ─────────────────────────────────────────────────────
pub async fn analyser(body: web::Json<RequeteAnalyse>) -> impl Responder {
    let rr = if body.stop_loss != 0.0 && body.prix_entree != body.stop_loss {
        let risque = (body.prix_entree - body.stop_loss).abs();
        let gain = (body.take_profit_1 - body.prix_entree).abs();
        format!("{:.2}", gain / risque)
    } else {
        "N/A".to_string()
    };
    let tp2_str = body
        .take_profit_2
        .filter(|&v| v > 0.0)
        .map(|v| format!("{v:.2}"))
        .unwrap_or_else(|| "—".to_string());
    let tp3_str = body
        .take_profit_3
        .filter(|&v| v > 0.0)
        .map(|v| format!("{v:.2}"))
        .unwrap_or_else(|| "—".to_string());

    let prompt = format!(
        "Tu es un trader institutionnel SMC/ICT expert. Analyse ce signal candidat avec rigueur.\n\n\
        ## SIGNAL CANDIDAT : {asset} {tf}\n\
        - Direction : {dir}\n\
        - Score confluence SMC : {score:.1}/100\n\
        - Prix entrée : {entree:.2} | Stop-Loss : {sl:.2}\n\
        - TP1 : {tp1:.2} (R:R={rr}) | TP2 : {tp2} | TP3 : {tp3}\n\
        - Détail SMC : Tendance={tend:.1}/{smax_tend} OB={ob:.1}/{smax_ob} Imbalance={imb:.1}/{smax_imb} IFVG={ifvg:.1}/{smax_ifvg} Fibonacci={fib:.1}/{smax_fib}\n\
        - Confiance ML : {ml:.1}%\n\n\
        ## CRITÈRES À VÉRIFIER\n\
        1. Kill Zone active ? (London 07h-10h / NY 13h30-16h30 UTC) — BLOQUANT si absent\n\
        2. Sweep de liquidité confirmé ? — BLOQUANT si absent\n\
        3. Order Block non mitigé aligné avec la direction ?\n\
        4. R:R ≥ 2:1 minimum ? (calculé : {rr})\n\
        5. Score SMC ≥ {seuil_score:.0}/100 et ML ≥ 60% ? (actuels : {score:.0} / {ml:.0}%)\n\n\
        ## PHILOSOPHIE : QUALITÉ > QUANTITÉ\n\
        Il vaut mieux passer ce signal que le valider sans confluence suffisante.\n\n\
        Fournis une analyse en 5-7 phrases couvrant dans l'ordre :\n\
        1. Verdict (Valide / À éviter) et raison principale\n\
        2. Points forts du setup (confluences présentes)\n\
        3. Points faibles ou risques (critères manquants, zone dangereuse)\n\
        4. Niveau d'invalidation clé à surveiller\n\
        5. Recommandation finale (trader, patienter, ou rejeter)",
        asset = body.asset, tf = body.timeframe, dir = body.direction,
        score = body.score_smc, entree = body.prix_entree, sl = body.stop_loss,
        tp1 = body.take_profit_1, tp2 = tp2_str, tp3 = tp3_str, rr = rr,
        tend = body.tendance, ob = body.order_block, imb = body.imbalance,
        ifvg = body.ifvg, fib = body.fibonacci, ml = body.confiance_ml * 100.0,
        seuil_score = body.score_min.unwrap_or(60.0),
        smax_tend = SCORE_MAX_TENDANCE as u32, smax_ob = SCORE_MAX_ORDER_BLOCK as u32,
        smax_imb = SCORE_MAX_IMBALANCE as u32, smax_ifvg = SCORE_MAX_IFVG as u32,
        smax_fib = SCORE_MAX_FIBONACCI as u32,
    );

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:32b".to_string());
    match ollama::interroger(&prompt).await {
        Ok(texte) => HttpResponse::Ok().json(ReponseAnalyse {
            analyse: texte,
            modele,
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Lancez Ollama: ollama serve && ollama pull qwen3:32b"
        })),
    }
}

// ─── POST /api/ia/chat ────────────────────────────────────────────────────────
pub async fn chat(state: web::Data<AppState>, body: web::Json<RequeteChat>) -> impl Responder {
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

    let api_key = state
        .db
        .lire_config("anthropic_api_key")
        .await
        .ok()
        .flatten();
    let forcer_ollama = body.forcer_ollama.unwrap_or(false);
    if !forcer_ollama {
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            let coach_prompt = crate::prompts_handler::prompt_effectif("coach");
            match crate::anthropic::chat_claude(&historique, &coach_prompt, &key).await {
                Ok(reponse) => {
                    return HttpResponse::Ok().json(ReponseChat {
                        reponse,
                        modele: crate::anthropic::MODELE_CLAUDE.to_string(),
                    })
                }
                Err(e) => {
                    tracing::warn!("Anthropic indisponible, bascule sur Ollama: {}", e);
                    // fall-through → Ollama
                }
            }
        }
    }

    let _coach_prompt = crate::prompts_handler::prompt_effectif("coach");

    match ollama::interroger_chat_modele_avec_systeme(
        &historique,
        ollama::MODELE_COACH,
        ollama::SYSTEM_PROMPT_COACH_OLLAMA,
    )
    .await
    {
        Ok(reponse) => HttpResponse::Ok().json(ReponseChat {
            reponse,
            modele: ollama::MODELE_COACH.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Clé Anthropic non configurée et Ollama injoignable. Configurez la clé dans Paramètres ou lancez: ollama serve"
        })),
    }
}

// ─── POST /api/ia/diagram ─────────────────────────────────────────────────────
pub async fn generer_diagram(body: web::Json<RequeteDiagram>) -> impl Responder {
    if body.sujet.trim().is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "sujet ne peut pas être vide" }));
    }
    // Templates pré-construits : résultat garanti, zéro latence GPU
    if let Some(svg) = ollama::diagram_templates::trouver_template(&body.sujet) {
        return HttpResponse::Ok().json(ReponseChat {
            reponse: format!("<htmldiagram>{}</htmldiagram>", svg),
            modele: "smc-templates".to_string(),
        });
    }
    let prompt_utilisateur = format!(
        "Génère un diagramme SVG de trading SMC illustrant : {}",
        body.sujet.trim()
    );
    let historique = vec![("user".to_string(), prompt_utilisateur)];
    match ollama::interroger_chat_modele_avec_systeme(
        &historique,
        ollama::MODELE_COACH_DIAGRAM,
        ollama::SYSTEM_PROMPT_COACH_DIAGRAM,
    )
    .await
    {
        Ok(reponse) => HttpResponse::Ok().json(ReponseChat {
            reponse,
            modele: ollama::MODELE_COACH_DIAGRAM.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "qwen2.5-coder:14b requis : ollama pull qwen2.5-coder:14b"
        })),
    }
}

// ─── GET /api/ia/status ───────────────────────────────────────────────────────
pub async fn statut() -> impl Responder {
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:32b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let client = &*crate::ollama::OLLAMA_HTTP_CLIENT;
    let disponible = client
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
