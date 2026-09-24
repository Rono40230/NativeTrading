//! Verticale KDJ/Halftrend (7.E) : réglages moteur + scanner de tendance.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

/// GET /api/kdj/params — réglages moteur (table kdj_params).
pub async fn get_params(state: web::Data<AppState>) -> impl Responder {
    let p = db::kdj_params::lire_kdj_params(state.db.pool()).await;
    HttpResponse::Ok().json(p)
}

#[derive(Debug, Deserialize)]
pub struct ParamsCorps {
    pub period: i64,
    pub signal: i64,
    pub amplitude: i64,
    pub ratio_risk: f64,
    pub adx_min: f64,
}

/// PUT /api/kdj/params — effet au prochain armement des moteurs
/// (redémarrage de l'app), comme les réglages Straddle.
pub async fn put_params(state: web::Data<AppState>, corps: web::Json<ParamsCorps>) -> impl Responder {
    let c = corps.into_inner();
    let p = db::kdj_params::KdjParams {
        period: c.period,
        signal: c.signal,
        amplitude: c.amplitude,
        ratio_risk: c.ratio_risk,
        adx_min: c.adx_min,
    };
    match db::kdj_params::sauvegarder_kdj_params(state.db.pool(), &p).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "effet": "au prochain démarrage de l'app"
        })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "erreur": e.to_string() })),
    }
}

/// Actifs armés pour le moteur KDJ H1 (config `kdj_assets_armes`, JSON
/// tableau d'ids). Absent/illisible = TOUS armés (compatibilité) ;
/// tableau vide = aucun (KDJ silencieux). Choix propriétaire 24/09 — le
/// balayage 7.G montre que l'écart entre actifs dépasse celui des
/// paramètres. Lu à chaque tick runtime (60 s) : la modale s'applique
/// sans redémarrage.
pub async fn assets_armes_kdj(db: &db::Database) -> Option<std::collections::HashSet<String>> {
    db.lire_config("kdj_assets_armes")
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .map(|v| v.into_iter().collect())
}

/// Moteur KDJ/Halftrend (7.E) construit depuis la table `kdj_params` —
/// H1 uniquement (screening + rejeu) ; appelé par l'armement runtime.
pub(crate) fn moteur_kdj(
    p: &db::kdj_params::KdjParams,
    asset: &common::Asset,
    tf: common::Timeframe,
) -> kdj_halftrend::KdjEngine {
    kdj_halftrend::KdjEngine::nouveau(asset.clone(), tf).avec_params(kdj_halftrend::ParamsKdj {
        period: p.period.max(2) as usize,
        signal: p.signal.max(2) as usize,
        amplitude: p.amplitude.max(1) as usize,
        ratio_risk: p.ratio_risk.clamp(0.1, 10.0),
        adx_min: if p.adx_min >= 0.0 { Some(p.adx_min) } else { None },
    })
}

#[derive(Debug, Serialize)]
pub struct LigneScanner {
    pub asset: String,
    /// Type de l'actif (crypto | metal | forex | indice) — filtres du scanner.
    pub r#type: String,
    /// ADX(14) H1 — la force de tendance du TF de la stratégie.
    pub adx_h1: f64,
    /// ADX(14) D1 — contexte de fond.
    pub adx_d1: f64,
    /// Hausse | Baisse | Neutre (SMA100 vs EMA200, H1).
    pub direction: String,
    /// close vs EMA200 (H1).
    pub au_dessus_ema: bool,
    /// Tendance franche (ADX ≥ 25) | Modérée (≥ 20) | Range.
    pub label: String,
    /// Dernier KDJ (k, d, j) à la clôture H1.
    pub kdj: Option<(f64, f64, f64)>,
    /// SMA100 > EMA200 à la clôture (condition tendance long).
    pub sma_au_dessus: bool,
}

fn derniere_adx(bougies: &[common::Candle]) -> f64 {
    let adx = indicators::calculer_adx(bougies, 14);
    adx.iter().rev().find(|v| v.is_finite()).copied().unwrap_or(f64::NAN)
}

/// GET /api/kdj/scanner — classe les actifs par force de tendance (ADX,
/// étude 7.E : le filtre améliore les métaux mais dégrade SP500/BTC →
/// SÉLECTEUR d'actifs, pas filtre global). Tri : le plus franc d'abord.
pub async fn get_scanner(state: web::Data<AppState>) -> impl Responder {
    let db = Arc::clone(&state.db);
    let assets = crate::runtime_tick::assets_runtime(&db).await;
    // Type par actif (filtres du scanner) — lecture directe, légère.
    let types_assets: std::collections::HashMap<String, String> =
        sqlx::query_as::<_, (String, String)>("SELECT id, type FROM assets")
            .fetch_all(db.pool())
            .await
            .map(|lignes| lignes.into_iter().collect())
            .unwrap_or_default();
    let mut lignes: Vec<LigneScanner> = Vec::new();

    for a in &assets {
        let (b_h1, b_d1) = tokio::join!(
            db.obtenir_bougies_depuis_jours(a, &common::Timeframe::H1, 120),
            db.obtenir_bougies_depuis_jours(a, &common::Timeframe::D1, 200),
        );
        let Ok(b_h1) = b_h1 else { continue };
        if b_h1.len() < 120 {
            continue;
        }
        let adx_h1 = derniere_adx(&b_h1);
        let adx_d1 = b_d1.as_ref().map(|b| derniere_adx(b)).unwrap_or(f64::NAN);

        let sma = indicators::calculer_sma(&b_h1, 100);
        let ema = indicators::calculer_ema(&b_h1, 200);
        let last = b_h1.len() - 1;
        let (direction, au_dessus) = if sma[last].is_finite() && ema[last].is_finite() {
            let au_dessus = b_h1[last].close > ema[last];
            let dir = if sma[last] > ema[last] {
                "Hausse"
            } else if sma[last] < ema[last] {
                "Baisse"
            } else {
                "Neutre"
            };
            (dir.to_string(), au_dessus)
        } else {
            ("Neutre".into(), false)
        };

        let force = adx_h1.max(adx_d1);
        let label = if force.is_nan() {
            "—".to_string()
        } else if force >= 25.0 {
            "Tendance franche".to_string()
        } else if force >= 20.0 {
            "Modérée".to_string()
        } else {
            "Range".to_string()
        };

        let kdj_serie = indicators::calculer_kdj(&b_h1, 20, 7);
        lignes.push(LigneScanner {
            asset: a.as_str().to_string(),
            r#type: types_assets
                .get(a.as_str())
                .cloned()
                .unwrap_or_else(|| "indice".into()),
            adx_h1,
            adx_d1,
            direction,
            au_dessus_ema: au_dessus,
            label,
            kdj: if kdj_serie.j[last].is_finite() {
                Some((kdj_serie.k[last], kdj_serie.d[last], kdj_serie.j[last]))
            } else {
                None
            },
            sma_au_dessus: sma[last].is_finite() && sma[last] > ema[last],
        });
    }

    lignes.sort_by(|x, y| {
        y.adx_h1
            .max(y.adx_d1)
            .partial_cmp(&x.adx_h1.max(x.adx_d1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    HttpResponse::Ok().json(lignes)
}
