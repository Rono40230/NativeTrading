//! API des réglages PAR ASSET (spec docs/spec_reglages_par_asset.md, 0121).
//!
//! `GET    /api/strategies/{id}/reglages-asset/{asset}` → surcharge + défauts
//! globaux (l'UI affiche le défaut en placeholder grisé).
//! `PUT`  → écrit la surcharge (fractions validées : 3 valeurs, somme 1,00) ;
//! le hot-reload ré-arme les couples de CET asset sous 60 s.
//! `DELETE` → supprime la surcharge (retour au défaut global).

use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

fn strategie_valide(id: &str) -> bool {
    matches!(id, "SMC" | "straddle" | "kdj_halftrend")
}

async fn asset_valide(state: &AppState, asset: &str) -> bool {
    crate::runtime_tick::assets_runtime(&state.db)
        .await
        .iter()
        .any(|a| a.as_str() == asset)
}

pub async fn get_reglages(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (id, asset) = path.into_inner();
    if !strategie_valide(&id) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Stratégie inconnue (SMC | straddle | kdj_halftrend)" }));
    }
    if !asset_valide(&state, &asset).await {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": format!("Asset inconnu : {asset}") }));
    }
    let pool = state.db.pool();
    let (surcharge, defaut) = match id.as_str() {
        "SMC" => {
            let s = db::reglages_asset::lire_smc_surcharge(pool, &asset).await.unwrap_or_default();
            let r = crate::reglages_asset_fusion::reglages_smc(&state.db, &asset).await;
            (
                serde_json::to_value(&s).unwrap_or_default(),
                serde_json::json!({
                    "tp1": r.tp1, "tp2": r.tp2,
                    "tp3_mode": if r.tp3.lointaine { "lointaine" } else { "fixe" },
                    "tp3_rfixe": r.tp3.rfixe,
                    "trailing_r": r.trailing,
                    "sl_fixe": r.sl_fixe,
                    "palier_faible": r.palier_faible,
                    "palier_fort": r.palier_fort,
                    "sl_pips": db::asset_params::lire_pips(pool, &asset)
                        .await
                        .ok()
                        .flatten()
                        .map(|(p, _)| p),
                    "taille_pip": db::asset_params::lire_pips(pool, &asset)
                        .await
                        .ok()
                        .flatten()
                        .map(|(_, t)| t),
                    "frac": crate::reglages_smc::lire_fractions(&state.db).await,
                }),
            )
        }
        "straddle" => {
            let s = db::reglages_asset::lire_straddle_surcharge(pool, &asset).await.unwrap_or_default();
            let p = db::strategies_params::lire_straddle_params(pool).await;
            (
                serde_json::to_value(&s).unwrap_or_default(),
                serde_json::json!({ "sl_mult": p.sl_mult, "trailing_r": p.trailing_r, "placement_sec": p.placement_sec }),
            )
        }
        _ => {
            let s = db::reglages_asset::lire_kdj_surcharge(pool, &asset).await.unwrap_or_default();
            let g = db::kdj_params::lire_kdj_params(pool).await;
            (
                serde_json::to_value(&s).unwrap_or_default(),
                serde_json::json!({ "period": g.period, "signal": g.signal, "amplitude": g.amplitude, "ratio_risk": g.ratio_risk, "adx_min": g.adx_min }),
            )
        }
    };
    HttpResponse::Ok().json(serde_json::json!({ "asset": asset, "surcharge": surcharge, "defaut": defaut }))
}

pub async fn put_reglages(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let (id, asset) = path.into_inner();
    if !strategie_valide(&id) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    if !asset_valide(&state, &asset).await {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": format!("Asset inconnu : {asset}") }));
    }
    let pool = state.db.pool();
    let corps = body.into_inner();
    let ecriture = match id.as_str() {
        "SMC" => {
            let s: db::reglages_asset::SmcSurcharge =
                match serde_json::from_value(corps) {
                    Ok(v) => v,
                    Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({ "error": format!("Corps invalide : {e}") })),
                };
            db::reglages_asset::ecrire_smc_surcharge(pool, &asset, &s).await
        }
        "straddle" => {
            let s: db::reglages_asset::StraddleSurcharge = match serde_json::from_value(corps) {
                Ok(v) => v,
                Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({ "error": format!("Corps invalide : {e}") })),
            };
            db::reglages_asset::ecrire_straddle_surcharge(pool, &asset, &s).await
        }
        _ => {
            let s: db::reglages_asset::KdjSurcharge = match serde_json::from_value(corps) {
                Ok(v) => v,
                Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({ "error": format!("Corps invalide : {e}") })),
            };
            db::reglages_asset::ecrire_kdj_surcharge(pool, &asset, &s).await
        }
    };
    match ecriture {
        Ok(()) => {
            tracing::info!(
                "Réglages par asset : {id}/{asset} surchargés — hot-reload ≤ 60 s (couples de cet asset seuls)"
            );
            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn delete_reglages(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (id, asset) = path.into_inner();
    if !strategie_valide(&id) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    let pool = state.db.pool();
    let suppression = match id.as_str() {
        "SMC" => db::reglages_asset::supprimer_smc_surcharge(pool, &asset).await,
        "straddle" => db::reglages_asset::supprimer_straddle_surcharge(pool, &asset).await,
        _ => db::reglages_asset::supprimer_kdj_surcharge(pool, &asset).await,
    };
    match suppression {
        Ok(()) => {
            tracing::info!("Réglages par asset : {id}/{asset} réinitialisés sur le défaut global");
            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}


/// PUT /api/assets/{asset}/sl_pips — SL fixe de la modale ⚙️ (source unique
/// asset_params, partagée avec Gestion du risque). Hot-reload ≤ 60 s.
pub async fn put_sl_pips(
    state: web::Data<AppState>,
    chemin: web::Path<String>,
    corps: web::Json<serde_json::Value>,
) -> HttpResponse {
    let asset = chemin.into_inner();
    let Some(sl) = corps.get("sl_pips").and_then(|v| v.as_f64()) else {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "erreur": "sl_pips (nombre) requis" }));
    };
    match db::asset_params::maj_sl_pips(state.db.pool(), &asset, sl).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "erreur": e.to_string() })),
    }
}
