use actix_web::{web, HttpResponse, Responder};
use db::signaux;
use serde::Deserialize;
use std::time::Duration;

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

fn calculer_verdict(
    direction: &str,
    stop_loss: f64,
    take_profit: &[f64],
    prix: f64,
) -> Option<&'static str> {
    let long = direction.to_uppercase().contains("LONG");
    if long {
        if prix <= stop_loss {
            return Some("SL");
        }
        if take_profit.get(2).is_some_and(|&t| prix >= t) {
            return Some("TP3");
        }
        if take_profit.get(1).is_some_and(|&t| prix >= t) {
            return Some("TP2");
        }
        if take_profit.first().is_some_and(|&t| prix >= t) {
            return Some("TP1");
        }
    } else {
        if prix >= stop_loss {
            return Some("SL");
        }
        if take_profit.get(2).is_some_and(|&t| prix <= t) {
            return Some("TP3");
        }
        if take_profit.get(1).is_some_and(|&t| prix <= t) {
            return Some("TP2");
        }
        if take_profit.first().is_some_and(|&t| prix <= t) {
            return Some("TP1");
        }
    }
    None
}

// ── MFE : excursion favorable avant le SL (spéc propriétaire 31/08) ──────────
// Pour un trade clôturé sur SL, la vérité qui juge l'entrée n'est pas la sortie
// mais l'extrême favorable atteint avant la perte. Calcul depuis les bougies
// stockées (M1 en priorité, TF du signal en repli).

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

/// Worker lancé au démarrage : toutes les 5min, vérifie TP/SL des signaux SMC/Straddle.
pub async fn demarrer_worker_suivi_signaux(pool: sqlx::SqlitePool) {
    let client = &*crate::http_client::HTTP_CLIENT;

    loop {
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;

        if let Ok(n) = signaux::expirer_anciens(&pool).await {
            if n > 0 {
                tracing::info!("Signaux: {} expiré(s)", n);
            }
        }

        let actifs = match signaux::lister_actifs(&pool).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Worker signaux liste: {}", e);
                continue;
            }
        };

        for s in &actifs {
            let prix = match crate::prix_utils::fetch_prix_asset(&client, &s.asset).await {
                Some(p) => p,
                None => continue,
            };
            if let Some(v) = calculer_verdict(&s.direction, s.stop_loss, &s.take_profit, prix) {
                match signaux::maj_verdict(&pool, &s.id, v, prix).await {
                    Ok(_) => {
                        tracing::info!("Signal {} {} → {} @ {:.4}", s.asset, s.direction, v, prix)
                    }
                    Err(e) => tracing::warn!("Worker signaux verdict: {}", e),
                }
            }
        }
    }
}
