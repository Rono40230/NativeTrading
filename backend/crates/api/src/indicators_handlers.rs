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
    pub ema_ma_type: Option<String>,
    pub rsi_periode: Option<usize>,
    pub macd_rapide: Option<usize>,
    pub macd_lente: Option<usize>,
    pub macd_signal: Option<usize>,
    pub bollinger_periode: Option<usize>,
    pub bollinger_stddev: Option<f64>,
    pub bollinger_ma_type: Option<String>,
    pub atr_periode: Option<usize>,
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
    /// Si `true`, calcule et retourne les signaux pour tous les indicateurs actifs
    pub signaux: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signaux: Option<Vec<indicators::signaux::SignalIndicateur>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr_valeurs: Option<Vec<PointSerie>>,
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/indicators — Retourne indicateurs techniques + zones SMC + liquidités.
/// Paramètres actifs uniquement si flag=true pour éviter les calculs inutiles.
/// Exception : si `signaux=true`, les indicateurs sont calculés en interne pour la
/// détection de signaux même si leur flag d'affichage est désactivé.
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
    let limit = query.limit.unwrap_or(500) as i64;

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
    let ema_sma = query.ema_ma_type.as_deref() == Some("sma");
    let rsi_p = query.rsi_periode.unwrap_or(14);
    let macd_rapide_p = query.macd_rapide.unwrap_or(12);
    let macd_lente_p = query.macd_lente.unwrap_or(26);
    let macd_signal_p = query.macd_signal.unwrap_or(9);
    let boll_p = query.bollinger_periode.unwrap_or(20);
    let boll_std = query.bollinger_stddev.unwrap_or(2.0);
    let boll_ma = query.bollinger_ma_type.as_deref().unwrap_or("sma");

    // ── Valeurs brutes (conservées pour la détection de signaux) ────────────────
    let ts = &timestamps;
    let need_signals = query.signaux.unwrap_or(false);
    let serie = |vals: &[f64]| -> Vec<PointSerie> {
        vals.iter()
            .zip(ts.iter())
            .filter(|(v, _)| v.is_finite())
            .map(|(v, &t)| PointSerie { time: t, value: *v })
            .collect()
    };

    // Calculer si le flag d'affichage OU si les signaux sont demandés
    let ema_raw: Option<Vec<f64>> = (query.ema.unwrap_or(false) || need_signals).then(|| {
        if ema_sma {
            indicators::calculer_sma(&bougies, ema_p)
        } else {
            indicators::calculer_ema(&bougies, ema_p)
        }
    });
    // Retourner dans la réponse seulement si le flag d'affichage est actif
    let ema = query.ema.unwrap_or(false).then(|| ema_raw.as_deref().map(serie)).flatten();

    let rsi_raw: Option<Vec<f64>> =
        (query.rsi.unwrap_or(false) || need_signals).then(|| indicators::calculer_rsi(&bougies, rsi_p));
    let rsi = query.rsi.unwrap_or(false).then(|| rsi_raw.as_deref().map(serie)).flatten();

    let atr_raw: Option<Vec<f64>> = (query.atr.unwrap_or(false) || need_signals)
        .then(|| indicators::calculer_atr(&bougies, query.atr_periode.unwrap_or(14)));
    let atr = query.atr.unwrap_or(false).then(|| atr_raw.as_deref().map(serie)).flatten();

    let macd_computed: Option<indicators::Macd> = (query.macd.unwrap_or(false) || need_signals)
        .then(|| indicators::calculer_macd(&bougies, macd_rapide_p, macd_lente_p, macd_signal_p));
    let macd = query.macd.unwrap_or(false).then(|| {
        macd_computed.as_ref().map(|m| SeriesMacd {
            macd: serie(&m.ligne),
            signal: serie(&m.signal),
            histogramme: serie(&m.histogramme),
        })
    }).flatten();

    let boll_computed: Option<indicators::Bollinger> = (query.bollinger.unwrap_or(false) || need_signals)
        .then(|| indicators::calculer_bollinger_avance(&bougies, boll_p, boll_std, boll_ma));
    let bollinger = query.bollinger.unwrap_or(false).then(|| {
        boll_computed.as_ref().map(|b| SeriesBollinger {
            haute: serie(&b.superieure),
            milieu: serie(&b.milieu),
            basse: serie(&b.inferieure),
        })
    }).flatten();

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

    // ── Signaux indicateurs (détection + confluence) ─────────────────────────
    let signaux = query.signaux.unwrap_or(false).then(|| {
        let closes: Vec<f64> = bougies.iter().map(|b| b.close).collect();
        let mut tous: Vec<indicators::signaux::SignalIndicateur> = Vec::new();
        if let Some(ref v) = ema_raw {
            tous.extend(indicators::signaux::detecter_signaux_ema(&timestamps, &closes, v));
        }
        if let Some(ref v) = rsi_raw {
            tous.extend(indicators::signaux::detecter_signaux_rsi(&timestamps, v, 70.0, 30.0));
        }
        if let Some(ref m) = macd_computed {
            tous.extend(indicators::signaux::detecter_signaux_macd(
                &timestamps, &m.ligne, &m.signal, &m.histogramme,
            ));
        }
        if let Some(ref b) = boll_computed {
            tous.extend(indicators::signaux::detecter_signaux_bollinger(
                &timestamps, &closes, &b.superieure, &b.milieu, &b.inferieure, boll_p,
            ));
        }
        if let Some(ref v) = atr_raw {
            tous.extend(indicators::signaux::detecter_signaux_atr(&timestamps, &closes, v, 14));
        }
        // Signaux combinés multi-indicateurs
        tous.extend(indicators::signaux::detecter_signaux_combines(
            &timestamps,
            &closes,
            ema_raw.as_deref(),
            rsi_raw.as_deref(),
            macd_computed.as_ref().map(|m| m.ligne.as_slice()),
            macd_computed.as_ref().map(|m| m.signal.as_slice()),
            boll_computed.as_ref().map(|b| b.superieure.as_slice()),
            boll_computed.as_ref().map(|b| b.milieu.as_slice()),
            boll_computed.as_ref().map(|b| b.inferieure.as_slice()),
            atr_raw.as_deref(),
        ));
        // Enrichir prix_entree avec le close réel au timestamp du signal
        let ts_to_close: std::collections::HashMap<i64, f64> =
            timestamps.iter().copied().zip(closes.iter().copied()).collect();
        for s in &mut tous {
            if let Some(&c) = ts_to_close.get(&s.timestamp) {
                s.prix_entree = c;
            }
        }
        indicators::signaux::calculer_confluence(tous)
    });

    // Valeurs ATR indexées par timestamp — retournées si signaux=true pour calcul SL/TP
    let atr_valeurs = query.signaux.unwrap_or(false)
        .then(|| atr_raw.as_deref().map(serie))
        .flatten();

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
        signaux,
        atr_valeurs,
    })
}

