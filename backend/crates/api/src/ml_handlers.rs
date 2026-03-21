use actix_web::{web, HttpResponse, Responder};
use common::Asset;
use db::entrainements::EntrainementRecord;
use ml::entrainer_walk_forward;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

#[derive(Deserialize)]
pub struct EntrainementQuery {
    pub asset: Option<String>,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ReponseEntrainement {
    pub success: bool,
    pub accuracy_rf: f64,
    pub accuracy_lstm: f64,
    pub accuracy_finale: f64,
    pub nb_echantillons: usize,
    pub duree_ms: u128,
    pub derive_detectee: bool,
    pub message: String,
}

/// POST /api/ml/train?asset=BTC&timeframe=M15&limit=1000
/// Lance l'entraînement RF + LSTM sur les données Binance. Retour synchrone (~30–90s sur CPU).
pub async fn entrainer_ml(
    query: web::Query<EntrainementQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    let asset = parse_asset(query.asset.as_deref().unwrap_or("BTC")).unwrap_or(Asset::BTC);
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(1000).min(2000) as usize;

    tracing::info!(
        "Entraînement ML demandé: {:?} {:?} limit={}",
        asset,
        timeframe,
        limit
    );

    // Récupération des bougies depuis la DB
    let bougies = match state
        .db
        .obtenir_bougies(
            &parse_asset(query.asset.as_deref().unwrap_or("XAUUSD")).unwrap_or(Asset::XAUUSD),
            &parse_timeframe(query.timeframe.as_deref().unwrap_or("M15")),
            limit as i64,
        )
        .await
    {
        Ok(b) if b.len() >= 100 => b,
        Ok(b) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Données insuffisantes: {} bougies (min 100) — IB Gateway doit être connecté", b.len())
            }));
        }
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": format!("DB: {}", e)
            }));
        }
    };

    let nb = bougies.len();
    let debut = Instant::now();

    // ── Walk-forward (métriques out-of-sample honnêtes) ───────────────────────
    let wf = match entrainer_walk_forward(&bougies) {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Walk-forward échoué: {}", e)
            }));
        }
    };

    // ── Entraînement pipeline principal sur 100 % des données ─────────────────
    let mut pipeline = state.pipeline_ml.lock().await;
    match pipeline.entrainer_sur_historique(&bougies, 5, 0.002) {
        Ok(_) => {
            drop(pipeline);
            let duree_ms = debut.elapsed().as_millis();

            // Dérive : accuracy < 60 % sur les 7 derniers jours
            let derive = state.db.detecter_derive_ml(0.60).await.unwrap_or(false);

            // Persistance en DB
            let rec = EntrainementRecord {
                asset: format!("{:?}", asset),
                timeframe: query.timeframe.clone().unwrap_or_else(|| "M15".to_string()),
                nb_bougies: nb as i64,
                accuracy_rf: wf.accuracy_rf,
                accuracy_lstm: wf.accuracy_lstm,
                accuracy_finale: wf.accuracy_finale,
                duree_ms: duree_ms as i64,
                derive_detectee: derive,
            };
            if let Err(e) = state.db.inserer_historique_entrainement(&rec).await {
                tracing::warn!("Échec enregistrement historique entrainement: {}", e);
            }

            if derive {
                tracing::warn!("⚠️ Dérive ML détectée après entraînement manuel");
            }

            tracing::info!(
                "Entraînement terminé en {}ms: RF={:.1}% LSTM={:.1}% Finale={:.1}%",
                duree_ms,
                wf.accuracy_rf * 100.0,
                wf.accuracy_lstm * 100.0,
                wf.accuracy_finale * 100.0,
            );
            HttpResponse::Ok().json(ReponseEntrainement {
                success: true,
                accuracy_rf: wf.accuracy_rf,
                accuracy_lstm: wf.accuracy_lstm,
                accuracy_finale: wf.accuracy_finale,
                nb_echantillons: nb,
                duree_ms,
                derive_detectee: derive,
                message: format!(
                    "RF: {:.1}% | LSTM: {:.1}% | Finale: {:.1}% ({} bougies en {}ms){}",
                    wf.accuracy_rf * 100.0,
                    wf.accuracy_lstm * 100.0,
                    wf.accuracy_finale * 100.0,
                    nb,
                    duree_ms,
                    if derive { " ⚠️ DÉRIVE" } else { "" }
                ),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Échec entraînement pipeline principal: {}", e)
        })),
    }
}

/// GET /api/ml/status — état du pipeline ML
pub async fn statut_ml(state: web::Data<AppState>) -> impl Responder {
    let pipeline = state.pipeline_ml.lock().await;
    HttpResponse::Ok().json(serde_json::json!({
        "modele_pret": pipeline.est_pret(),
        "lstm_pret": pipeline.lstm.est_pret(),
    }))
}

/// GET /api/ml/history?limit=30 — historique des entraînements + dérive
pub async fn historique_ml(
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> impl Responder {
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30)
        .clamp(1, 200);

    let historique = match state.db.obtenir_historique_entrainements(limit).await {
        Ok(h) => h,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("DB: {}", e)
            }));
        }
    };

    let derive = state.db.detecter_derive_ml(0.60).await.unwrap_or(false);
    let nb = historique.len();

    HttpResponse::Ok().json(serde_json::json!({
        "historique": historique,
        "derive_detectee": derive,
        "seuil_derive": 0.60,
        "nb_entrainements": nb,
    }))
}
