//! Gestion des données historiques — collecte bulk + couverture
use actix_web::{web, HttpResponse, Responder};
use common::Timeframe;
use data::{providers::BinanceProvider, DataProvider};
use serde::Deserialize;

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// Bougies estimées par mois selon le timeframe (conservateur — crypto 24/7)
fn limit_pour_mois(tf: &Timeframe, mois: u32) -> usize {
    let par_mois: usize = match tf {
        Timeframe::M1 => 43_200,
        Timeframe::M5 => 8_640,
        Timeframe::M15 => 2_880,
        Timeframe::M30 => 1_440,
        Timeframe::H1 => 720,
        Timeframe::H4 => 180,
        Timeframe::D1 => 30,
        Timeframe::W1 => 4,
    };
    par_mois * mois as usize
}

// ─── GET /api/data/coverage ───────────────────────────────────────────────────

/// Retourne la couverture de données stockées par asset × timeframe,
/// avec la taille actuelle de la base (PRAGMA page_count × page_size).
pub async fn get_coverage(state: web::Data<AppState>) -> impl Responder {
    let taille_db: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
    )
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    // Bougies reçues depuis minuit Paris — compteur de flux journalier
    // (la couverture % et la taille DB sont des jauges de fonds, muettes
    // à l'échelle du jour).
    let maintenant = chrono::Utc::now().timestamp();
    let minuit_paris =
        maintenant - ((maintenant + common::time::offset_paris_seconds(maintenant)) % 86_400);
    let bougies_auj: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM bougies WHERE timestamp >= ?1",
    )
    .bind(minuit_paris)
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    match state.db.obtenir_couverture_donnees().await {
        Ok(data) => HttpResponse::Ok().json(serde_json::json!({
            "couverture": data,
            "taille_db_octets": taille_db,
            "bougies_aujourd_hui": bougies_auj,
        })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "erreur": e.to_string() }))
        }
    }
}

// ─── POST /api/data/collect ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteCollecte {
    /// Assets à collecter (défaut : BTC + ETH uniquement — pas d'IB nécessaire)
    pub assets: Option<Vec<String>>,
    /// Timeframes à collecter (défaut : M5, M15, H1)
    pub timeframes: Option<Vec<String>>,
    /// Nombre de mois d'historique à collecter (défaut : 6)
    pub mois: Option<u32>,
}

pub async fn post_collect(
    state: web::Data<AppState>,
    body: web::Json<RequeteCollecte>,
) -> impl Responder {
    let mois = body.mois.unwrap_or(6).clamp(1, 24);

    // Assets à collecter — défaut Binance uniquement si non précisé
    let asset_ids: Vec<String> = body
        .assets
        .clone()
        .unwrap_or_else(|| vec!["BTC".to_string(), "ETH".to_string()]);

    // Timeframes — défaut M5, M15, H1
    let tf_ids: Vec<String> = body
        .timeframes
        .clone()
        .unwrap_or_else(|| vec!["M5".to_string(), "M15".to_string(), "H1".to_string()]);

    let mut resultats = Vec::new();
    let mut total_inseres: u64 = 0;

    for asset_str in &asset_ids {
        let asset = match parse_asset(asset_str) {
            Some(a) => a,
            None => {
                resultats.push(serde_json::json!({
                    "asset": asset_str, "erreur": "Asset inconnu"
                }));
                continue;
            }
        };

        // Sélection du provider selon l'asset
        let provider: Box<dyn DataProvider> = if asset.est_cotable_bybit() {
            Box::new(BinanceProvider)
        } else {
            // IG : pas de provider REST (Lightstreamer). data_collect ne fonctionne que pour crypto.
            return HttpResponse::Ok().json(serde_json::json!({
                "message": "Collecte historique IG non disponible — utiliser import CSV MT5"
            }));
        };

        for tf_str in &tf_ids {
            let tf = parse_timeframe(tf_str);

            let limit = limit_pour_mois(&tf, mois);

            tracing::info!(
                "Collecte bulk {} {} — {} bougies (~{} mois)",
                asset_str,
                tf_str,
                limit,
                mois
            );

            match tokio::time::timeout(
                std::time::Duration::from_secs(120),
                provider.fetch_candles(asset.clone(), tf, limit),
            )
            .await
            {
                Ok(Ok(bougies)) => {
                    let nb_fetched = bougies.len();
                    match state.db.inserer_bougies(&asset, &tf, &bougies).await {
                        Ok(inseres) => {
                            total_inseres += inseres;
                            resultats.push(serde_json::json!({
                                "asset": asset_str, "timeframe": tf_str,
                                "fetched": nb_fetched, "inseres": inseres,
                            }));
                        }
                        Err(e) => {
                            resultats.push(serde_json::json!({
                                "asset": asset_str, "timeframe": tf_str, "erreur": e.to_string()
                            }));
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Collecte {}/{} échouée: {}", asset_str, tf_str, e);
                    resultats.push(serde_json::json!({
                        "asset": asset_str, "timeframe": tf_str, "erreur": e.to_string()
                    }));
                }
                Err(_) => {
                    tracing::warn!("Collecte {}/{} timeout (120s)", asset_str, tf_str);
                    resultats.push(serde_json::json!({
                        "asset": asset_str, "timeframe": tf_str, "erreur": "Timeout 120s"
                    }));
                }
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "total_inseres": total_inseres,
        "mois": mois,
        "resultats": resultats,
    }))
}
