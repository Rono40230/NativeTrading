use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::{state::AppState, utils};
use actix_web::{web, HttpResponse};
use serde::Deserialize;

/// Fenêtre des patterns M1 — alignée sur la rétention M1 (12 mois,
/// décision owner 05/10 étape 14 : la base M1 ne garde plus que 12 mois).
pub(crate) const MOIS_PATTERNS: i64 = 12;
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
/// P85) de TOUS les assets actifs du pipeline sur 12 mois au M1 — la matière
/// première du bloc Créneaux de volatilité du dashboard (jour courant par
/// asset + analyses repliées). Le calcul scanne l'historique complet par
/// asset : la réponse est mise en cache une heure (les patterns M1
/// n'évoluent qu'à la bougie suivante).
/// Cache des patterns-jour (1 h) — au niveau module pour que le préchauffage
/// de démarrage (étape 1-bis, incident 05/10) remplisse le MÊME cache que
/// l'endpoint.
static CACHE_PATTERNS_JOUR: OnceLock<RwLock<Option<(Instant, serde_json::Value)>>> = OnceLock::new();

/// Préchauffe les patterns-jour en tâche de fond au boot (étape 1-bis) : le
/// calcul à froid scannait tout l'historique M1 par asset PENDANT que le dashboard
/// chargeait — tous les fetchs patientaient derrière (incident 05/10 « tout
/// à 0 »). Retourne la valeur calculée (et remplit le cache).
pub async fn prechauffer_patterns_jour(db: &std::sync::Arc<db::Database>) -> serde_json::Value {
    let cache = CACHE_PATTERNS_JOUR.get_or_init(|| RwLock::new(None));
    let valeur = calculer_patterns_jour(db).await;
    *cache.write().await = Some((Instant::now(), valeur.clone()));
    valeur
}

/// GET /api/volatility/patterns-jour
/// Patterns horaires (heure UTC × jour de semaine, clusters quartiles + seuil
/// P85) de TOUS les assets actifs du pipeline sur 12 mois au M1 — la matière
/// première du bloc Créneaux de volatilité du dashboard (jour courant par
/// asset + analyses repliées). Le calcul scanne l'historique complet par
/// asset : la réponse est mise en cache une heure (les patterns M1
/// n'évoluent qu'à la bougie suivante).
pub async fn get_patterns_jour(state: web::Data<AppState>) -> impl actix_web::Responder {
    let cache = CACHE_PATTERNS_JOUR.get_or_init(|| RwLock::new(None));

    if let Some((calcule_le, valeur)) = cache.read().await.clone() {
        if calcule_le.elapsed() < Duration::from_secs(3600) {
            return HttpResponse::Ok().json(valeur);
        }
    }

    let valeur = calculer_patterns_jour(&state.db).await;
    *cache.write().await = Some((Instant::now(), valeur.clone()));
    HttpResponse::Ok().json(valeur)
}

async fn calculer_patterns_jour(db: &std::sync::Arc<db::Database>) -> serde_json::Value {
    let workers = match db.lister_assets_worker().await {
        Ok(w) => w,
        Err(_) => return serde_json::json!({ "assets": [], "timeframe": "M1", "mois": 24 }),
    };
    let timeframe = utils::parse_timeframe("M1");

    let mut assets = Vec::new();
    for w in workers.into_iter().filter(|w| w.actif) {
        let Some(asset) = utils::parse_asset(&w.id) else { continue };
        // Un asset sans historique suffisant est simplement absent de la réponse.
        if let Ok(rep) = db.obtenir_patterns_horaires(&asset, &timeframe, MOIS_PATTERNS).await {
            if let Ok(mut v) = serde_json::to_value(&rep) {
                v["asset"] = serde_json::Value::String(w.id.clone());
                assets.push(v);
            }
        }
    }

    serde_json::json!({ "assets": assets, "timeframe": "M1", "mois": 24 })
}
