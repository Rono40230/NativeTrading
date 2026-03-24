use actix_web::{web, HttpResponse, Responder};
use data::{providers::BinanceProvider, providers::IbGatewayProvider, DataProvider};

use crate::ollama::straddle_analyse;
use crate::state::AppState;
use crate::straddle_precision;
use crate::straddle_utils::{
    limite_bougies, periode_en_mois, MaJCreneau, ReponseAnalyse, RequeteAnalyse, MAX_BOUGIES_RESEAU,
};
use crate::utils::parse_asset;

// ── POST /api/straddle/analyser ───────────────────────────────────────────────
/// Lance une analyse LLM des créneaux de volatilité pour un asset donné.
/// Supprime les anciens créneaux `a_tester` de l'asset avant d'insérer les nouveaux.
pub async fn analyser(
    state: web::Data<AppState>,
    body: web::Json<RequeteAnalyse>,
) -> impl Responder {
    use common::Timeframe;

    let asset_str = body.asset.trim().to_uppercase();
    let asset = match parse_asset(&asset_str) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté" }))
        }
    };

    let periode_mois = periode_en_mois(body.periode.as_deref());
    let limite = limite_bougies(periode_mois);

    tracing::info!(
        "Straddle analyse LLM: asset={} période={}m limite={} bougies H1",
        asset_str,
        periode_mois,
        limite
    );

    // Récupérer les bougies H1 (cache DB puis provider réseau en fallback)
    let bougies = match state
        .db
        .obtenir_bougies(&asset, &Timeframe::H1, limite as i64)
        .await
    {
        Ok(b) if !b.is_empty() => b,
        _ => {
            // Fallback: provider réseau (plafonné car API Binance : max 1000/appel)
            let limite_reseau = limite.min(MAX_BOUGIES_RESEAU);
            let res = if asset.is_crypto() {
                BinanceProvider
                    .fetch_candles(asset.clone(), Timeframe::H1, limite_reseau)
                    .await
            } else {
                IbGatewayProvider::new(state.ib_port, state.ib_client_id)
                    .fetch_candles(asset.clone(), Timeframe::H1, limite_reseau)
                    .await
            };
            match res {
                Ok(b) => {
                    let _ = state.db.inserer_bougies(&asset, &Timeframe::H1, &b).await;
                    b
                }
                Err(e) => {
                    tracing::warn!(
                        "Impossible de récupérer les bougies H1 pour {}: {}",
                        asset_str,
                        e
                    );
                    // Réponse métier claire — pas de 500 — provider non disponible
                    return HttpResponse::Ok().json(serde_json::json!({
                        "creneaux": [],
                        "nb_analyses": 0,
                        "nb_retenus": 0,
                        "message": format!(
                            "Données indisponibles pour {} : IB Gateway hors ligne et aucun historique en cache. \
                             Vérifiez que MetaTrader / IB Gateway est démarré.",
                            asset_str
                        )
                    }));
                }
            }
        }
    };

    let nb_analyses = bougies.len();

    // Guard : pas assez de bougies pour une analyse statistique fiable
    if nb_analyses < 100 {
        tracing::warn!(
            "Straddle {}: seulement {} bougies H1 en cache — analyse impossible",
            asset_str,
            nb_analyses
        );
        return HttpResponse::Ok().json(serde_json::json!({
            "creneaux": [],
            "nb_analyses": nb_analyses,
            "nb_retenus": 0,
            "message": format!(
                "{} : seulement {} bougies H1 disponibles (≈{} jours). \
                 L'analyse Straddle nécessite au moins 3 semaines de données. \
                 Démarrez IB Gateway / MetaTrader pour alimenter le cache.",
                asset_str,
                nb_analyses,
                nb_analyses / 24
            )
        }));
    }
    let _ = db::straddle::supprimer_creneaux_asset(state.db.pool(), &asset_str).await;

    // Analyse LLM
    match straddle_analyse::analyser_creneaux(&asset_str, periode_mois, &bougies).await {
        Ok(nouveaux) => {
            let nb_retenus = nouveaux.len();
            if let Err(e) = db::straddle::inserer_creneaux(state.db.pool(), &nouveaux).await {
                tracing::error!("Erreur insertion créneaux straddle: {}", e);
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() }));
            }

            match db::straddle::lister_creneaux_asset(state.db.pool(), &asset_str).await {
                Ok(creneaux) => HttpResponse::Ok().json(ReponseAnalyse {
                    creneaux,
                    nb_analyses,
                    nb_retenus,
                }),
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        Err(e) => {
            tracing::error!("Straddle LLM échoué pour {}: {}", asset_str, e);
            // Réponse métier claire — pas de 500 — LLM indisponible
            HttpResponse::Ok().json(serde_json::json!({
                "creneaux": [],
                "nb_analyses": nb_analyses,
                "nb_retenus": 0,
                "message": format!(
                    "Analyse impossible : le modèle LLM Ollama n'est pas disponible ({}). \
                     Vérifiez qu'Ollama est démarré et que le modèle est chargé.",
                    e
                )
            }))
        }
    }
}

// ── GET /api/straddle/creneaux ────────────────────────────────────────────────
/// Liste tous les créneaux identifiés, triés par conviction LLM.
pub async fn lister_creneaux(state: web::Data<AppState>) -> impl Responder {
    match db::straddle::lister_creneaux(state.db.pool()).await {
        Ok(creneaux) => HttpResponse::Ok().json(creneaux),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── PATCH /api/straddle/creneaux/{id} ────────────────────────────────────────
/// Met à jour le statut et/ou les résultats backtest d'un créneau.
pub async fn mettre_a_jour_creneau(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<MaJCreneau>,
) -> impl Responder {
    let id = path.into_inner();
    match db::straddle::mettre_a_jour_creneau(
        state.db.pool(),
        id,
        body.statut.clone(),
        body.backtest_winrate,
        body.backtest_profit_factor,
    )
    .await
    {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── POST /api/straddle/creneaux/{id}/precision ───────────────────────────────
/// Analyse les bougies M5 pour calculer la précision d'entrée sur un créneau.
pub async fn analyser_precision(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> impl Responder {
    use common::Timeframe;
    let id = path.into_inner();

    let creneau = match db::straddle::lister_creneaux(state.db.pool()).await {
        Ok(liste) => match liste.into_iter().find(|c| c.id == id) {
            Some(c) => c,
            None => {
                return HttpResponse::NotFound()
                    .json(serde_json::json!({ "error": "Créneau introuvable" }))
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let asset = match parse_asset(&creneau.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté" }))
        }
    };

    let limite_m5 = 6 * 30 * 24 * 12_i64;
    let candles_m5 = match state
        .db
        .obtenir_bougies(&asset, &Timeframe::M5, limite_m5)
        .await
    {
        Ok(b) if !b.is_empty() => b,
        _ => {
            let res = if asset.is_crypto() {
                BinanceProvider
                    .fetch_candles(asset.clone(), Timeframe::M5, 1000)
                    .await
            } else {
                IbGatewayProvider::new(state.ib_port, state.ib_client_id)
                    .fetch_candles(asset.clone(), Timeframe::M5, 1000)
                    .await
            };
            match res {
                Ok(b) => {
                    let _ = state.db.inserer_bougies(&asset, &Timeframe::M5, &b).await;
                    b
                }
                Err(e) => {
                    return HttpResponse::Ok().json(serde_json::json!({
                        "ok": false,
                        "message": format!(
                            "Données M5 indisponibles pour {} : {}. Vérifiez que le provider est démarré.",
                            creneau.asset, e
                        )
                    }))
                }
            }
        }
    };

    tracing::info!(
        "Précision M5 créneau#{} {} {}–{}: {} bougies M5",
        id,
        creneau.asset,
        creneau.heure_debut,
        creneau.heure_fin,
        candles_m5.len()
    );

    match straddle_precision::analyser_precision(
        &candles_m5,
        creneau.jour_semaine,
        &creneau.heure_debut,
        &creneau.heure_fin,
    ) {
        Some(precision) => {
            let result = serde_json::json!({
                "timing_optimal": precision.timing_optimal,
                "fenetre_entree": precision.fenetre_entree,
                "whipsaw_minutes": precision.whipsaw_minutes,
                "nb_occurrences": precision.nb_occurrences,
                "atr_pic": precision.atr_pic,
            });
            if let Err(e) =
                db::straddle::mettre_a_jour_precision(state.db.pool(), id, &precision).await
            {
                tracing::warn!("Impossible de sauvegarder la précision M5: {}", e);
            }
            HttpResponse::Ok().json(result)
        }
        None => HttpResponse::Ok().json(serde_json::json!({
            "ok": false,
            "message": "Pas assez de bougies M5 dans ce créneau pour calculer la précision."
        })),
    }
}
