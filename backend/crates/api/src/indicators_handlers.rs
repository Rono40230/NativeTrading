use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IndicatorsQuery {
    pub asset: String,
    pub tf: Option<String>,
    pub ema_periode: Option<usize>,
    pub rsi_periode: Option<usize>,
    pub ema: Option<bool>,
    pub rsi: Option<bool>,
    pub macd: Option<bool>,
    pub bollinger: Option<bool>,
    pub atr: Option<bool>,
    pub smc_ob: Option<bool>,
    pub smc_fvg: Option<bool>,
    pub smc_ifvg: Option<bool>,
    pub smc_fib: Option<bool>,
    pub smc_tendance: Option<bool>,
    pub smc_liquidites: Option<bool>,
    pub limit: Option<u32>,
}

// ─── Réponse ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PointSerie {
    pub time: i64,
    pub value: f64,
}

#[derive(Serialize)]
pub struct SeriesMacd {
    pub macd: Vec<PointSerie>,
    pub signal: Vec<PointSerie>,
    pub histogramme: Vec<PointSerie>,
}

#[derive(Serialize)]
pub struct SeriesBollinger {
    pub haute: Vec<PointSerie>,
    pub milieu: Vec<PointSerie>,
    pub basse: Vec<PointSerie>,
}

#[derive(Serialize)]
pub struct ReponseIndicators {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema: Option<Vec<PointSerie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsi: Option<Vec<PointSerie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr: Option<Vec<PointSerie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macd: Option<SeriesMacd>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bollinger: Option<SeriesBollinger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_blocks: Option<Vec<smc::OrderBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imbalances: Option<Vec<smc::Imbalance>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifvg: Option<Vec<smc::Ifvg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fibonacci: Option<smc::NiveauxFibonacci>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendance: Option<smc::ResultatTendance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidites: Option<Vec<smc::NiveauLiquidite>>,
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/indicators — Retourne indicateurs techniques + zones SMC + liquidités.
/// Paramètres actifs uniquement si flag=true pour éviter les calculs inutiles.
pub async fn get_indicators(
    state: web::Data<AppState>,
    query: web::Query<IndicatorsQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté" }))
        }
    };
    let tf = parse_timeframe(query.tf.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(200).min(500) as i64;

    let bougies = match state.db.obtenir_bougies(&asset, &tf, limit).await {
        Ok(b) if b.len() >= 20 => b,
        _ => {
            return HttpResponse::Ok().json(serde_json::json!({
                "error": "Données insuffisantes — lancez d'abord GET /api/candles"
            }))
        }
    };

    let timestamps: Vec<i64> = bougies.iter().map(|b| b.timestamp.timestamp()).collect();
    let ema_p = query.ema_periode.unwrap_or(20);
    let rsi_p = query.rsi_periode.unwrap_or(14);

    // ── Indicateurs techniques ───────────────────────────────────────────────
    let ts = &timestamps;
    // Filtre les NaN : lightweight-charts v4 rejette silencieusement les valeurs NaN
    let serie = |vals: Vec<f64>| -> Vec<PointSerie> {
        vals.into_iter()
            .zip(ts.iter())
            .filter(|(v, _)| v.is_finite())
            .map(|(v, &t)| PointSerie { time: t, value: v })
            .collect()
    };

    let ema = query
        .ema
        .unwrap_or(false)
        .then(|| serie(indicators::calculer_ema(&bougies, ema_p)));

    let rsi = query
        .rsi
        .unwrap_or(false)
        .then(|| serie(indicators::calculer_rsi(&bougies, rsi_p)));

    let atr = query
        .atr
        .unwrap_or(false)
        .then(|| serie(indicators::calculer_atr(&bougies, 14)));

    let macd = query.macd.unwrap_or(false).then(|| {
        let m = indicators::calculer_macd(&bougies, 12, 26, 9);
        SeriesMacd {
            macd: serie(m.ligne),
            signal: serie(m.signal),
            histogramme: serie(m.histogramme),
        }
    });

    let bollinger = query.bollinger.unwrap_or(false).then(|| {
        let b = indicators::calculer_bollinger(&bougies, 20, 2.0);
        SeriesBollinger {
            haute: serie(b.superieure),
            milieu: serie(b.milieu),
            basse: serie(b.inferieure),
        }
    });

    // ── Zones SMC ────────────────────────────────────────────────────────────
    let order_blocks = query
        .smc_ob
        .unwrap_or(false)
        .then(|| smc::order_blocks::detecter(&bougies));

    let imbalances = query
        .smc_fvg
        .unwrap_or(false)
        .then(|| smc::imbalance::detecter(&bougies));

    let ifvg = query
        .smc_ifvg
        .unwrap_or(false)
        .then(|| smc::ifvg::detecter(&bougies));

    let fibonacci = query
        .smc_fib
        .unwrap_or(false)
        .then(|| smc::fibonacci::calculer(&bougies))
        .flatten();

    let tendance = query
        .smc_tendance
        .unwrap_or(false)
        .then(|| smc::tendances::analyser(&bougies))
        .flatten();

    let liquidites = query
        .smc_liquidites
        .unwrap_or(false)
        .then(|| smc::liquidites::detecter(&bougies));

    HttpResponse::Ok().json(ReponseIndicators {
        ema,
        rsi,
        atr,
        macd,
        bollinger,
        order_blocks,
        imbalances,
        ifvg,
        fibonacci,
        tendance,
        liquidites,
    })
}

