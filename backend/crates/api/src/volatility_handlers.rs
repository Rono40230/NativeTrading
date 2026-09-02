use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::{state::AppState, utils};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Deserialize)]
pub struct QueryPatterns {
    pub asset: Option<String>,
    pub timeframe: Option<String>,
    pub mois: Option<i64>,
}

/// GET /api/volatility/patterns?asset=BTC&timeframe=M15&mois=12
/// Retourne les patterns de volatilité ATR agrégés par heure/jour,
/// classifiés en 4 clusters + le seuil Straddle calibré (P85).
pub async fn get_patterns(
    state: web::Data<AppState>,
    query: web::Query<QueryPatterns>,
) -> impl actix_web::Responder {
    let asset = match utils::parse_asset(query.asset.as_deref().unwrap_or("BTC")) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Asset inconnu" }))
        }
    };
    let timeframe = utils::parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let mois = query.mois.unwrap_or(12).clamp(1, 60);

    match state
        .db
        .obtenir_patterns_horaires(&asset, &timeframe, mois)
        .await
    {
        Ok(rep) => HttpResponse::Ok().json(rep),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/volatility/patterns-jour
/// Patterns horaires (heure UTC × jour de semaine, clusters quartiles + seuil
/// P85) de TOUS les assets actifs du pipeline sur 24 mois au M1 — la matière
/// première du bloc Créneaux de volatilité du dashboard (jour courant par
/// asset + analyses repliées). Le calcul scanne l'historique complet par
/// asset : la réponse est mise en cache une heure (les patterns M1
/// n'évoluent qu'à la bougie suivante).
pub async fn get_patterns_jour(state: web::Data<AppState>) -> impl actix_web::Responder {
    static CACHE: OnceLock<RwLock<Option<(Instant, serde_json::Value)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(None));

    if let Some((calcule_le, valeur)) = cache.read().await.clone() {
        if calcule_le.elapsed() < Duration::from_secs(3600) {
            return HttpResponse::Ok().json(valeur);
        }
    }

    let workers = match state.db.lister_assets_worker().await {
        Ok(w) => w,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };
    let timeframe = utils::parse_timeframe("M1");

    let mut assets = Vec::new();
    for w in workers.into_iter().filter(|w| w.actif) {
        let Some(asset) = utils::parse_asset(&w.id) else { continue };
        // Un asset sans historique suffisant est simplement absent de la réponse.
        if let Ok(rep) = state.db.obtenir_patterns_horaires(&asset, &timeframe, 24).await {
            if let Ok(mut v) = serde_json::to_value(&rep) {
                v["asset"] = serde_json::Value::String(w.id.clone());
                assets.push(v);
            }
        }
    }

    let reponse = serde_json::json!({ "assets": assets, "timeframe": "M1", "mois": 24 });
    *cache.write().await = Some((Instant::now(), reponse.clone()));
    HttpResponse::Ok().json(reponse)
}
