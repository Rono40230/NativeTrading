use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};
use crate::indicators_types::{
    IndicatorsQuery, ReponseIndicators, PointSerie, SeriesMacd, SeriesBollinger,
};

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
        .then(|| {
            let sensitivity = query.smc_ob_sensitivity.unwrap_or(28.0);
            let mitigation_close = query.smc_ob_mitigation.as_deref() != Some("wick");
            smc::order_blocks::detecter(&bougies, sensitivity, mitigation_close)
        });

    let ifvg = query
        .smc_ifvg
        .unwrap_or(false)
        .then(|| {
            let show_last = query.smc_ifvg_show_last.unwrap_or(5) as usize;
            let signal_pref_close = query.smc_ifvg_signal_pref.as_deref() != Some("wick");
            let atr_mult = query.smc_ifvg_atr_mult.unwrap_or(0.25);
            smc::ifvg::detecter(&bougies, show_last, signal_pref_close, atr_mult)
        });

    let bpr = query
        .smc_bpr
        .unwrap_or(false)
        .then(|| {
            let show_last = query.smc_bpr_show_last.unwrap_or(5) as usize;
            let atr_mult = query.smc_bpr_atr_mult.unwrap_or(0.5);
            let fenetre = query.smc_bpr_fenetre.unwrap_or(30) as usize;
            let mitigation_close = query.smc_bpr_mitigation.as_deref() != Some("wick");
            smc::bpr::detecter(&bougies, show_last, atr_mult, fenetre, mitigation_close)
        });

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

    let imbalance = query
        .smc_imbalance
        .unwrap_or(false)
        .then(|| {
            let show_last = query.smc_imb_show_last.unwrap_or(5) as usize;
            let show_fvg  = query.smc_imb_show_fvg.unwrap_or(true);
            let show_og   = query.smc_imb_show_og.unwrap_or(true);
            let mitigation_close = query.smc_imb_mitigation.as_deref() != Some("wick");
            smc::imbalance::detecter(&bougies, show_last, show_fvg, show_og, mitigation_close)
        });

    let liquidites = query
        .smc_liquidites
        .unwrap_or(false)
        .then(|| {
            let params = smc::liquidites::ParamsLiquidites {
                swing_lookback:  query.smc_liq_swing_lookback.unwrap_or(50) as usize,
                swings_actif:    query.smc_liq_swings.unwrap_or(true),
                sessions_actif:  query.smc_liq_sessions.unwrap_or(true),
                session_asie:   query.smc_liq_session_asie.unwrap_or(true),
                dwm_actif:      query.smc_liq_dwm.unwrap_or(false),
                dwm_nb_jours:   query.smc_liq_dwm_nb.unwrap_or(2) as usize,
            };
            smc::liquidites::detecter(&bougies, params)
        });

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
        ifvg,
        bpr,
        imbalance,
        fibonacci,
        tendance,
        liquidites,
        signaux,
        atr_valeurs,
    })
}

