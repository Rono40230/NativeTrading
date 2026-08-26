use actix_web::{web, HttpResponse, Responder};
use data::{providers::BinanceProvider, DataProvider};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── Health ───────────────────────────────────────────────────────────────────

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ─── Candles ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CandlesQuery {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
    pub force: Option<bool>,
}

pub async fn get_candles(
    state: web::Data<AppState>,
    query: web::Query<CandlesQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Asset non supporté. Voir GET /api/assets pour la liste complète." })),
    };

    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200).min(50_000) as usize;
    let force = query.force.unwrap_or(false);

    // M10 : VISUEL SEUL — pas de stockage propre, agrégé à la volée depuis
    // les M1 (frontières alignées 10 min = agrégation sans perte). La
    // dernière bucket incomplète est laissée au flux WS (bougie en formation).
    if timeframe == common::Timeframe::M10 {
        let besoin_m1 = (limit * 10).min(50_000) as i64;
        let mut m1 = state
            .db
            .obtenir_bougies(&asset, &common::Timeframe::M1, besoin_m1)
            .await
            .unwrap_or_default();
        if m1.is_empty() && asset.est_cotable_bybit() {
            if let Ok(b) = BinanceProvider
                .fetch_candles(asset.clone(), common::Timeframe::M1, besoin_m1 as usize)
                .await
            {
                let _ = state.db.inserer_bougies(&asset, &common::Timeframe::M1, &b).await;
                m1 = b;
            }
        }
        let maintenant = chrono::Utc::now().timestamp();
        return HttpResponse::Ok().json(agreger_bougies(m1, 600, maintenant));
    }

    // 1. Cache DB — toutes sources : c'est LA source du chart. Le backfill
    //    profond y a déposé l'historique (BTC M15 : ~2 ans) et le WS la
    //    maintient à jour — le provider REST (plafonné 1000/requête) ne
    //    servirait qu'à tronquer la fenêtre demandée. `force` ne contourne
    //    plus la DB : il ne sert qu'au rattrapage si la DB est vide (voir 2).
    let _ = force;
    if let Ok(bougies) = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit as i64)
        .await
    {
        if !bougies.is_empty() {
            return HttpResponse::Ok().json(bougies);
        }
    }

    // 2. Pour les crypto : fallback Binance REST si cache vide (ou si l'option force a échoué mais le cache est vide)
    if asset.est_cotable_bybit() {
        let resultat = BinanceProvider
            .fetch_candles(asset.clone(), timeframe, limit)
            .await;
        match resultat {
            Ok(bougies) => {
                if let Err(e) = state.db.inserer_bougies(&asset, &timeframe, &bougies).await {
                    tracing::warn!("Impossible de mettre en cache les bougies crypto: {}", e);
                }
                return HttpResponse::Ok().json(bougies);
            }
            Err(e) => {
                tracing::warn!("get_candles Binance échoué pour {}: {}", query.asset, e);
            }
        }
    }
    // Pour les assets non-crypto sans cache : pas encore de provider REST.

    HttpResponse::Ok().json(Vec::<serde_json::Value>::new())
}

/// Agrège des bougies M1 en buckets de `duree_sec` (frontières alignées).
/// Bougies triées par timestamp croissant (ordre DB). La dernière bucket
/// incomplète (encore ouverte à `maintenant`) est ignorée : elle revient au
/// flux WS en temps réel.
fn agreger_bougies(bougies: Vec<common::Candle>, duree_sec: i64, maintenant: i64) -> Vec<common::Candle> {
    use chrono::TimeZone;
    let mut out: Vec<common::Candle> = Vec::new();
    for b in bougies {
        let ts = b.timestamp.timestamp();
        let bucket = ts - ts % duree_sec;
        if bucket + duree_sec > maintenant {
            break; // bucket encore ouverte — WS
        }
        match out.last_mut() {
            Some(prev) if prev.timestamp.timestamp() == bucket => {
                prev.high = prev.high.max(b.high);
                prev.low = prev.low.min(b.low);
                prev.close = b.close;
                prev.volume += b.volume;
            }
            _ => out.push(common::Candle {
                timestamp: chrono::Utc.timestamp_opt(bucket, 0).single().unwrap_or(b.timestamp),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            }),
        }
    }
    out
}

// ─── Prédiction ML ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PredictQuery {
    pub asset: String,
    pub timeframe: Option<String>,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/PredictionML.ts")]
pub struct ReponsePrediction {
    pub asset: String,
    pub direction: String,
    pub confiance: f64,
    pub est_confiant: bool,
    pub modele_pret: bool,
}

pub async fn predict_ml(
    state: web::Data<AppState>,
    query: web::Query<PredictQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté." }));
        }
    };

    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));

    let bougies = match state.db.obtenir_bougies(&asset, &timeframe, 100).await {
        Ok(b) if !b.is_empty() => b,
        _ => {
            // Pas de données en cache → modèle non prêt, retourner 200
            return HttpResponse::Ok().json(ReponsePrediction {
                asset: query.asset.clone(),
                direction: "inconnu".to_string(),
                confiance: 0.0,
                est_confiant: false,
                modele_pret: false,
            });
        }
    };

    let pipeline = state.pipeline_ml.read().await;

    if !pipeline.est_pret() {
        return HttpResponse::Ok().json(ReponsePrediction {
            asset: query.asset.clone(),
            direction: "inconnu".to_string(),
            confiance: 0.0,
            est_confiant: false,
            modele_pret: false,
        });
    }

    match pipeline.predire(&bougies) {
        Ok(pred) => HttpResponse::Ok().json(ReponsePrediction {
            asset: query.asset.clone(),
            direction: format!("{:?}", pred.direction),
            confiance: pred.confiance,
            est_confiant: pred.est_confiant,
            modele_pret: true,
        }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ─── Prix actuel (Binance spot, tout ticker) ──────────────────────────────────




#[cfg(test)]
mod tests_m10 {
    use super::agreger_bougies;
    use chrono::TimeZone;

    fn m1(ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> common::Candle {
        common::Candle {
            timestamp: chrono::Utc.timestamp_opt(ts, 0).single().unwrap(),
            open: o, high: h, low: l, close: c, volume: v,
        }
    }

    #[test]
    fn dix_m1_fusionnees_en_une_bucket_m10() {
        // Bucket 12:00 → 12:10 (ts 43200..43209 en minutes), maintenant = fin.
        let bougies: Vec<_> = (0..10)
            .map(|i| m1(43200 * 60 + i * 60, 100.0 + i as f64, 101.0 + i as f64, 99.0, 100.5 + i as f64, 10.0))
            .collect();
        let out = agreger_bougies(bougies, 600, (43200 * 60) + 600 + 1);
        assert_eq!(out.len(), 1);
        let b = &out[0];
        assert_eq!(b.timestamp.timestamp(), 43200 * 60);
        assert!((b.open - 100.0).abs() < 1e-9, "open = première M1");
        assert!((b.high - 110.0).abs() < 1e-9, "high = max");
        assert!((b.low - 99.0).abs() < 1e-9, "low = min");
        assert!((b.close - 109.5).abs() < 1e-9, "close = dernière M1");
        assert!((b.volume - 100.0).abs() < 1e-9, "volume sommé");
    }

    #[test]
    fn bucket_ouverte_ignoree() {
        // 3 M1 dans la bucket courante (non fermée à `maintenant`).
        let bougies: Vec<_> = (0..3)
            .map(|i| m1(50000 * 60 + i * 60, 10.0, 11.0, 9.0, 10.0, 1.0))
            .collect();
        let out = agreger_bougies(bougies, 600, 50000 * 60 + 180);
        assert!(out.is_empty(), "bucket incomplète laissée au WS");
    }
}
