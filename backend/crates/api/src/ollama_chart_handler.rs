use actix_web::{web, HttpResponse, Responder};

use crate::ollama;
use crate::ollama_types::{ReponseChartAnalyse, RequeteChartAnalyse};
use crate::state::AppState;

// ─── POST /api/ia/chart ──────────────────────────────────────────────────────
/// Analyse visuelle via Claude Sonnet (Anthropic API).
/// La clé API est lue depuis la DB — jamais exposée au frontend.
pub async fn analyser_chart(
    state: web::Data<AppState>,
    body: web::Json<RequeteChartAnalyse>,
) -> impl Responder {
    if body.images.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Au moins une image requise" }));
    }

    let api_key = match state.db.lire_config("anthropic_api_key").await {
        Ok(Some(k)) if !k.is_empty() => k,
        _ => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "Clé API Anthropic non configurée",
                "aide": "Allez dans Paramètres → IA Vision pour saisir votre clé API Anthropic"
            }));
        }
    };

    let slices: Vec<(&str, &str)> = body
        .images
        .iter()
        .map(|img| (img.base64.as_str(), img.timeframe.as_str()))
        .collect();

    match crate::anthropic::analyser_images_claude(
        &slices,
        &body.asset,
        body.notes.as_deref(),
        &api_key,
    )
    .await
    {
        Ok(analyse) => HttpResponse::Ok().json(ReponseChartAnalyse {
            analyse,
            modele: crate::anthropic::MODELE_CLAUDE.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e)
        })),
    }
}

// ─── POST /api/ia/chart/local ────────────────────────────────────────────────
/// Analyse visuelle via le modèle vision local (llama3.2-vision ou qwen2.5-vl).
pub async fn analyser_chart_local(body: web::Json<RequeteChartAnalyse>) -> impl Responder {
    if body.images.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Au moins une image requise" }));
    }

    // Les images arrivent en data URL (data:image/...;base64,XXX) — extraire le base64 pur
    let images_pure: Vec<(String, &str)> = body
        .images
        .iter()
        .map(|img| {
            let b64 = if let Some(pos) = img.base64.find(',') {
                img.base64[pos + 1..].to_string()
            } else {
                img.base64.clone()
            };
            (b64, img.timeframe.as_str())
        })
        .collect();

    let slices: Vec<(&str, &str)> = images_pure
        .iter()
        .map(|(b, tf)| (b.as_str(), *tf))
        .collect();

    match ollama::analyser_images(&slices, &body.asset, body.notes.as_deref()).await {
        Ok(analyse) => HttpResponse::Ok().json(ReponseChartAnalyse {
            analyse,
            modele: ollama::MODELE_VISION.to_string(),
        }),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Vérifiez qu'Ollama est démarré et que llama3.2-vision:11b est installé"
        })),
    }
}
