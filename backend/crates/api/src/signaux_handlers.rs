use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct QuerySignaux {
    pub limit: Option<i64>,
}

/// GET /api/signaux?limit=N — historique avec verdict inclus
pub async fn get_signaux(
    state: web::Data<AppState>,
    query: web::Query<QuerySignaux>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(500);
    match state.db.obtenir_signaux(limit).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique signaux: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteMfe {
    pub ids: Vec<String>,
}

/// MFE en × R : (meilleur prix favorable − entrée) / risque.
/// Direction Both (Straddle) : le meilleur des deux sens — la passe ayant fini
/// SL, l'excursion de la jambe qui a couru est l'info de jugement.
/// Retourne (mfe_r, meilleur_prix).
fn calculer_mfe_r(direction: &str, entree: f64, sl: f64, haut_max: f64, bas_min: f64) -> Option<(f64, f64)> {
    let risque = (entree - sl).abs();
    if risque <= f64::EPSILON {
        return None;
    }
    let dir = direction.to_uppercase();
    let (mfe, meilleur) = if dir.contains("BOTH") {
        let long = (haut_max - entree) / risque;
        let short = (entree - bas_min) / risque;
        if long >= short { (long, haut_max) } else { (short, bas_min) }
    } else if dir.contains("LONG") {
        ((haut_max - entree) / risque, haut_max)
    } else {
        ((entree - bas_min) / risque, bas_min)
    };
    Some((mfe, meilleur))
}

/// POST /api/signaux/mfe — corps { ids: [...] } → { id: { mfe_r, meilleur_prix } }
/// Les ids non trouvés / sans bougies couvrantes sont simplement absents de la
/// réponse (le front affiche « — »).
pub async fn post_mfe_signaux(
    state: web::Data<AppState>,
    body: web::Json<RequeteMfe>,
) -> impl Responder {
    const MAX_IDS: usize = 300;
    let ids: Vec<String> = body.ids.iter().take(MAX_IDS).cloned().collect();
    let lignes = match state.db.obtenir_signaux_sl_par_ids(&ids).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("MFE lecture signaux: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }));
        }
    };

    let mut reponse = serde_json::Map::new();
    for l in &lignes {
        // M1 d'abord (précision maximale), sinon repli sur le TF du signal.
        let extremes_m1 = state
            .db
            .extremes_bougies(&l.asset, "M1", l.cree_le, l.ferme_le)
            .await
            .ok()
            .flatten();
        let extremes = match extremes_m1 {
            Some(e) => Some(e),
            None => state
                .db
                .extremes_bougies(&l.asset, &l.timeframe, l.cree_le, l.ferme_le)
                .await
                .ok()
                .flatten(),
        };
        if let Some((haut_max, bas_min)) = extremes {
            if let Some((mfe_r, meilleur_prix)) =
                calculer_mfe_r(&l.direction, l.prix_entree, l.stop_loss, haut_max, bas_min)
            {
                reponse.insert(
                    l.id.clone(),
                    serde_json::json!({ "mfe_r": mfe_r, "meilleur_prix": meilleur_prix }),
                );
            }
        }
    }

    HttpResponse::Ok().json(serde_json::Value::Object(reponse))
}

#[cfg(test)]
mod tests_mfe {
    use super::calculer_mfe_r;

    #[test]
    fn mfe_long() {
        // entrée 2000, SL 1990 (risque 10) → pointe à 2009 = +0.9R
        let (r, p) = calculer_mfe_r("LONG", 2000.0, 1990.0, 2009.0, 1995.0).unwrap();
        assert!((r - 0.9).abs() < 1e-9);
        assert_eq!(p, 2009.0);
    }

    #[test]
    fn mfe_short() {
        // entrée 2000, SL 2012 (risque 12) → descend à 1990 = +10/12 R
        let (r, p) = calculer_mfe_r("SHORT", 2000.0, 2012.0, 2005.0, 1990.0).unwrap();
        assert!((r - 10.0 / 12.0).abs() < 1e-9);
        assert_eq!(p, 1990.0);
    }

    #[test]
    fn mfe_both_prend_le_meilleur_sens() {
        // passe straddle : le meilleur côté est le short (−20 du point bas)
        let (r, p) = calculer_mfe_r("Both", 2000.0, 2010.0, 2015.0, 1980.0).unwrap();
        assert!((r - 2.0).abs() < 1e-9);
        assert_eq!(p, 1980.0);
    }

    #[test]
    fn mfe_risque_nul_rend_none() {
        assert!(calculer_mfe_r("LONG", 2000.0, 2000.0, 2010.0, 1990.0).is_none());
    }
}

// ── Lot : taille de position au moment de l'émission (recalculée) ──────────
// Même formule que l'émission (signaux_officiels::formater_message) :
// lot = (capital composé de la stratégie à l'émission × risque %) /
//       (stop en pips × valeur du pip). Le capital d'époque est reconstitué
// depuis la simulation composée (dernière clôture avant l'émission).

#[derive(Deserialize)]
pub struct RequeteLots {
    pub ids: Vec<String>,
}

/// POST /api/signaux/lots — corps { ids } → { id: lot }.
/// Ids introuvables ou conventions manquantes : absents de la réponse
/// (le front affiche « — »). SMC (bases smc*) et straddle.
pub async fn post_lots_signaux(
    state: web::Data<AppState>,
    body: web::Json<RequeteLots>,
) -> impl Responder {
    use std::collections::HashMap;
    const MAX_IDS: usize = 500;
    let ids: Vec<String> = body.ids.iter().take(MAX_IDS).cloned().collect();
    let lignes = match state.db.obtenir_signaux_lot_par_ids(&ids).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Lots lecture signaux: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }));
        }
    };

    // Capital composé + risque par stratégie (une lecture par requête).
    let mut capitaux: HashMap<String, (f64, Vec<(i64, f64)>)> = HashMap::new();
    for l in &lignes {
        let strategie = if l.strategie.to_lowercase().starts_with("smc") {
            "SMC".to_string()
        } else {
            l.strategie.clone()
        };
        if capitaux.contains_key(&strategie) {
            continue;
        }
        let reg = state.db.lire_strategie(&strategie).await.ok().flatten();
        let points = crate::capital_simule::simuler(&state.db, &strategie)
            .await
            .map(|s| s.points.iter().map(|p| (p.ferme_le, p.capital_apres)).collect())
            .unwrap_or_default();
        let risque = reg.as_ref().map(|r| r.risque_pct).unwrap_or(1.0);
        capitaux.insert(strategie, (risque, points));
    }

    // Conventions par asset (cache requête).
    let mut conventions: HashMap<String, (f64, f64)> = HashMap::new();

    let mut reponse = serde_json::Map::new();
    for l in &lignes {
        let Some((risque_pct, points)) = capitaux.get(&l.strategie) else {
            continue;
        };
        let (taille_pip, valeur_pip) = match conventions.get(&l.asset) {
            Some(c) => *c,
            None => {
                let c = db::asset_params::lire_un(state.db.pool(), l.asset.as_str())
                    .await
                    .ok()
                    .flatten()
                    .map(|p| (p.taille_pip, p.valeur_pips))
                    .unwrap_or((0.0, 0.0));
                conventions.insert(l.asset.clone(), c);
                c
            }
        };
        if taille_pip <= 0.0 || valeur_pip <= 0.0 {
            continue;
        }
        let stop_pips = (l.prix_entree - l.stop_loss).abs() / taille_pip;
        if stop_pips <= 0.0 {
            continue;
        }
        // Capital composé au moment de l'émission : dernière clôture avant.
        let capital = points
            .iter()
            .filter(|(ferme, _)| *ferme <= l.cree_le)
            .next_back()
            .map(|(_, cap)| *cap)
            .unwrap_or(0.0);
        if capital <= 0.0 {
            continue;
        }
        let lot = capital * risque_pct / 100.0 / (stop_pips * valeur_pip);
        reponse.insert(l.id.clone(), serde_json::json!(lot));
    }

    HttpResponse::Ok().json(serde_json::Value::Object(reponse))
}
