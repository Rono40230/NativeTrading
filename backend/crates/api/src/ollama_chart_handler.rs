use actix_web::{web, HttpResponse, Responder};

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

#[derive(serde::Deserialize)]
pub struct SauvegardeAnalyseReq {
    pub image_base64: String,
    pub asset: String,
    pub timeframe: String,
}

// ─── POST /api/ia/save-analysis ──────────────────────────────────────────────
pub async fn analyser_chart_sauvegarde(body: web::Json<SauvegardeAnalyseReq>) -> impl Responder {
    use chrono::{Local, Datelike, Timelike};
    let maintenant = Local::now();
    
    let mois_noms = ["", "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    let nom_mois = mois_noms.get(maintenant.month() as usize).unwrap_or(&"");

    let nom_fichier = format!(
        "{} en {} le {} {} {} à {:02}h{:02}.png",
        body.asset.replace('/', "_"),
        body.timeframe.replace('/', "_"),
        maintenant.day(),
        nom_mois,
        maintenant.year(),
        maintenant.hour(),
        maintenant.minute()
    );
    let chemin_nom = format!("/home/rono/Téléchargements/{}", nom_fichier);

    let path = std::path::Path::new(&chemin_nom);
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": format!("Impossible de créer les dossiers: {}", e) }));
        }
    }

    use base64::{engine::general_purpose, Engine as _};
    match general_purpose::STANDARD.decode(&body.image_base64) {
        Ok(bytes) => match std::fs::write(&path, bytes) {
            Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "path": chemin_nom, "status": "success" })),
            Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": format!("Erreur d'écriture: {}", e) }))
        },
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": format!("Base64 invalide: {}", e) }))
    }
}
