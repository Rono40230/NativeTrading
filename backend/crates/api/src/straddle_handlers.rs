use actix_web::{web, HttpResponse, Responder};
use data::{providers::BinanceProvider, DataProvider};

use llm::straddle_analyse;
use crate::state::AppState;
use crate::straddle_utils::{
    limite_bougies, periode_en_mois, ReponseAnalyse, RequeteAnalyse, MAX_BOUGIES_RESEAU,
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
            let res = if asset.est_cotable_bybit() {
                BinanceProvider
                    .fetch_candles(asset.clone(), Timeframe::H1, limite_reseau)
                    .await
            } else {
                tracing::warn!(
                    "Straddle: cache H1 vide pour {} — Lightstreamer alimentera",
                    asset_str
                );
                Ok(vec![])
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

    // Annonces économiques HIGH impact imminentes (<2h) — enrichit le prompt LLM
    let annonces_imminentes = {
        let maintenant = chrono::Utc::now().timestamp();
        let dans_2h = maintenant + 7200;
        state
            .db
            .lire_calendrier_cache(3600)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|a| {
                a["impact"].as_str() == Some("High")
                    && a["date_heure"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| {
                            let ts = dt.timestamp();
                            ts >= maintenant && ts <= dans_2h
                        })
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>()
    };

    // Analyse LLM
    match straddle_analyse::analyser_creneaux(
        &asset_str,
        periode_mois,
        &bougies,
        &annonces_imminentes,
    )
    .await
    {
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


