use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use data::{providers::{BinanceProvider, IbGatewayProvider}, DataProvider};
use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};
use indicators::{calculer_ema, calculer_sma};

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TendanceQuery {
    pub asset: String,
    /// Période de la MM rapide (défaut : 9)
    pub mm_rapide: Option<usize>,
    /// Période de la MM lente (défaut : 21)
    pub mm_lente: Option<usize>,
    /// Type de MA : "ema" (défaut) ou "sma"
    pub ma_type: Option<String>,
}

// ─── Réponse ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Haussier,
    Baissier,
}

#[derive(Serialize)]
pub struct LigneTendance {
    pub tf: String,
    pub tendance: Option<Direction>,
    /// Valeur MM rapide (dernière bougie)
    pub mm_rapide: Option<f64>,
    /// Valeur MM lente (dernière bougie)
    pub mm_lente: Option<f64>,
}

#[derive(Serialize)]
pub struct ReponseTendanceMultiTf {
    pub asset: String,
    pub mm_rapide_periode: usize,
    pub mm_lente_periode: usize,
    pub ma_type: String,
    pub lignes: Vec<LigneTendance>,
}

// ─── Timeframes analysés ──────────────────────────────────────────────────────

const TIMEFRAMES_ANALYSE: &[(&str, u32)] = &[
    ("1m",  1),
    ("5m",  10),
    ("15m", 21),
    ("30m", 35),
    ("1H",  42),
    ("4H",  60),
    ("1D",  100),
];

/// Mappe label affiché → code Timeframe DB
fn label_vers_tf(label: &str) -> &'static str {
    match label {
        "1m"  => "M1",
        "5m"  => "M5",
        "15m" => "M15",
        "30m" => "M30",
        "1H"  => "H1",
        "4H"  => "H4",
        "1D"  => "D1",
        _     => "M15",
    }
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/tendance/multi-tf?asset=BTC&mm_rapide=9&mm_lente=21&ma_type=ema
///
/// Retourne la direction (Haussier / Baissier) pour chaque timeframe en
/// comparant la dernière valeur de la MM rapide avec celle de la MM lente.
pub async fn tendance_multi_tf(
    state: web::Data<AppState>,
    query: web::Query<TendanceQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "erreur": "Asset non supporté" }));
        }
    };

    let periode_rapide = query.mm_rapide.unwrap_or(9).max(1);
    let periode_lente  = query.mm_lente.unwrap_or(21).max(1);
    let ma_type        = query.ma_type.clone().unwrap_or_else(|| "ema".to_string());
    let use_ema        = ma_type.to_lowercase() != "sma";

    // Périodes × 20 pour que l'EMA ait le temps de converger (TradingView utilise l'historique complet)
    // Ex : EMA(21) → min 420 bougies ; on prend au moins 500.
    let limit_bougies = (periode_lente * 20).max(500) as i64;

    let mut lignes: Vec<LigneTendance> = Vec::with_capacity(TIMEFRAMES_ANALYSE.len());

    for &(label, _) in TIMEFRAMES_ANALYSE {
        let tf_code = label_vers_tf(label);
        let tf      = parse_timeframe(tf_code);

        // 1. Essayer la DB
        let bougies_db = state.db.obtenir_bougies(&asset, &tf, limit_bougies).await.unwrap_or_default();

        // 2. Si DB insuffisante → fetch Binance (ou IB pour les non-crypto)
        let bougies: Vec<common::Candle> = if bougies_db.len() >= limit_bougies as usize {
            bougies_db
        } else {
            let resultat = match &asset {
                common::Asset::BTC | common::Asset::ETH => {
                    BinanceProvider.fetch_candles(asset.clone(), tf, limit_bougies as usize).await
                }
                _ => {
                    IbGatewayProvider::new(state.ib_port, state.ib_client_id)
                        .fetch_candles(asset.clone(), tf, limit_bougies as usize)
                        .await
                }
            };
            match resultat {
                Ok(b) => {
                    if let Err(e) = state.db.inserer_bougies(&asset, &tf, &b).await {
                        tracing::warn!("Tendance multi-TF — cache {} {}: {}", label, query.asset, e);
                    }
                    b
                }
                Err(e) => {
                    tracing::warn!("Tendance multi-TF — fetch {} {}: {}", label, query.asset, e);
                    // Utiliser ce qu'on a en DB même si insuffisant
                    if !bougies_db.is_empty() { bougies_db } else {
                        lignes.push(LigneTendance {
                            tf: label.to_string(),
                            tendance: None,
                            mm_rapide: None,
                            mm_lente: None,
                        });
                        continue;
                    }
                }
            }
        };

        if bougies.len() < periode_lente {
            lignes.push(LigneTendance {
                tf: label.to_string(),
                tendance: None,
                mm_rapide: None,
                mm_lente: None,
            });
            continue;
        }

        let (serie_rapide, serie_lente) = if use_ema {
            (calculer_ema(&bougies, periode_rapide), calculer_ema(&bougies, periode_lente))
        } else {
            (calculer_sma(&bougies, periode_rapide), calculer_sma(&bougies, periode_lente))
        };

        let val_rapide = serie_rapide.last().copied().filter(|v| v.is_finite());
        let val_lente  = serie_lente.last().copied().filter(|v| v.is_finite());

        let direction = match (val_rapide, val_lente) {
            (Some(r), Some(l)) if r > l => Some(Direction::Haussier),
            (Some(r), Some(l)) if r < l => Some(Direction::Baissier),
            _ => None,
        };

        lignes.push(LigneTendance {
            tf: label.to_string(),
            tendance: direction,
            mm_rapide: val_rapide,
            mm_lente: val_lente,
        });
    }

    HttpResponse::Ok().json(ReponseTendanceMultiTf {
        asset: query.asset.to_uppercase(),
        mm_rapide_periode: periode_rapide,
        mm_lente_periode: periode_lente,
        ma_type: if use_ema { "ema".to_string() } else { "sma".to_string() },
        lignes,
    })
}
