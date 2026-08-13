//! Handler HTTP pour l'analyse SMC v12 (replay bar-par-bar du moteur complet).
//!
//! `GET /api/smc/v12/analyse?asset=XAUUSD&timeframe=M15&limit=500`
//!
//! Reconstruit l'indicateur Pine `smc_indicateur_v12.pine` en rejouant toutes les
//! bougies clôturées depuis la DB via `SmcV12Engine::update`. Collecte les
//! événements (pivots/BOS/MSS/CHOCH/sweeps) + les états actifs (OB/FVG vivants) +
//! tous les trades générés (avec leur verdict), puis applique les limites d'affichage
//! FIFO (comme TradingView : on ne garde que les N derniers visibles).

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use smc::v12::trade::{Side, TradeSource, Verdict};
use smc::v12::{BarInput, FvgState, ObState, ScoringV11, SmcV12Engine};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

// ── Limites FIFO d'affichage (Pine « max visible ») ──────────────────────────
const MAX_BOS: usize = 6;
const MAX_STRUCTURE: usize = 12;
const MAX_MSS: usize = 6;
const MAX_CHOCH: usize = 6;
const MAX_SWEEPS: usize = 6;
const MAX_FVG_PAR_SENS: usize = 10;
// OB déjà plafonnés à MAX_OB=40 par sens côté moteur.

#[derive(Deserialize)]
pub struct V12Query {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
struct PivotOut {
    ts: i64,
    #[serde(rename = "type")]
    ptype: &'static str,
    price: f64,
    bar_idx: usize,
}

#[derive(Serialize)]
struct NiveauStructOut {
    ts: i64,
    dir: &'static str,
    level: f64,
    bar_idx: usize,
}

#[derive(Serialize)]
struct ObOut {
    ts: i64,
    dir: &'static str,
    top: f64,
    bot: f64,
    state: &'static str,
    force: i32,
    bar_idx: usize,
}

#[derive(Serialize)]
struct FvgOut {
    ts: i64,
    dir: &'static str,
    top: f64,
    bot: f64,
    state: &'static str,
    bar_idx: usize,
}

#[derive(Serialize)]
struct SignalOut {
    ts: i64,
    dir: &'static str,
    entry: f64,
    sl: f64,
    tp1: f64,
    tp2: f64,
    tp3: f64,
    force: i32,
    source: &'static str,
    verdict: &'static str,
}

#[derive(Serialize)]
struct V12AnalyseResponse {
    asset: String,
    timeframe: String,
    nb_bougies: usize,
    pivots: Vec<PivotOut>,
    bos: Vec<NiveauStructOut>,
    mss: Vec<NiveauStructOut>,
    chochs: Vec<NiveauStructOut>,
    sweeps: Vec<NiveauStructOut>,
    obs: Vec<ObOut>,
    fvgs: Vec<FvgOut>,
    signals: Vec<SignalOut>,
    tendance: &'static str,
    atr14: f64,
}

/// GET /api/smc/v12/analyse?asset=XAUUSD&timeframe=M15&limit=500
///
/// Replay complet du moteur v12 sur les `limit` dernières bougies clôturées.
pub async fn analyse_v12(
    query: web::Query<V12Query>,
    state: web::Data<AppState>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Asset inconnu: {}", query.asset)
            }));
        }
    };
    let timeframe = parse_timeframe(query.timeframe.as_deref().unwrap_or("M15"));
    let limit = query.limit.unwrap_or(500).max(50) as i64;
    let asset_str = asset.as_str();
    let tf_str = timeframe.as_str();

    let bougies = state
        .db
        .obtenir_bougies(&asset, &timeframe, limit)
        .await
        .unwrap_or_default();

    if bougies.len() < 30 {
        // Données insuffisantes pour un replay fiable (warmup ATR + pivots).
        return HttpResponse::Ok().json(V12AnalyseResponse {
            asset: asset_str.to_string(),
            timeframe: tf_str.to_string(),
            nb_bougies: bougies.len(),
            pivots: Vec::new(),
            bos: Vec::new(),
            mss: Vec::new(),
            chochs: Vec::new(),
            sweeps: Vec::new(),
            obs: Vec::new(),
            fvgs: Vec::new(),
            signals: Vec::new(),
            tendance: "neutre",
            atr14: 0.0,
        });
    }

    // Replay bar-par-bar.
    let mut engine = SmcV12Engine::new(asset_str, tf_str);
    let mut ts_by_idx: Vec<i64> = Vec::with_capacity(bougies.len());
    let mut pivots: Vec<PivotOut> = Vec::new();
    let mut bos_list: Vec<NiveauStructOut> = Vec::new();
    let mut mss_list: Vec<NiveauStructOut> = Vec::new();
    let mut choch_list: Vec<NiveauStructOut> = Vec::new();
    let mut sweeps: Vec<NiveauStructOut> = Vec::new();

    let mut last_atr = 0.0_f64;
    let mut last_tendance = "neutre";

    for b in &bougies {
        let bar = BarInput {
            timestamp: b.timestamp.timestamp(),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        };
        ts_by_idx.push(bar.timestamp);
        let out = engine.update(&bar);
        last_atr = out.atr14;
        if out.tendance_haussiere {
            last_tendance = "haussiere";
        } else if out.tendance_baissiere {
            last_tendance = "baissiere";
        }

        // ── Structure (HH/HL/LH/LL) ── positionnée au pivot réel.
        let pbi = out.pivot.pivot_bar_index;
        if out.structure.is_hh || out.structure.is_lh {
            if let (Some(price), Some(bi)) = (out.pivot.pivot_high_price, pbi) {
                pivots.push(PivotOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    ptype: if out.structure.is_hh { "HH" } else { "LH" },
                    price,
                    bar_idx: bi,
                });
            }
        }
        if out.structure.is_hl || out.structure.is_ll {
            if let (Some(price), Some(bi)) = (out.pivot.pivot_low_price, pbi) {
                pivots.push(PivotOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    ptype: if out.structure.is_hl { "HL" } else { "LL" },
                    price,
                    bar_idx: bi,
                });
            }
        }

        // ── BOS ── bar courante (cassure).
        if (out.bos.bullish || out.bos.bearish) && out.bos.level.is_some() {
            if let (Some(level), Some(bi)) = (out.bos.level, out.bos.bar_index) {
                bos_list.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    dir: if out.bos.bullish { "bull" } else { "bear" },
                    level,
                    bar_idx: bi,
                });
            }
        }

        // ── MSS ──
        if (out.mss.mss_haussier || out.mss.mss_baissier) && out.mss.mss_level.is_some() {
            if let (Some(level), Some(bi)) = (out.mss.mss_level, out.mss.mss_bar) {
                mss_list.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    dir: if out.mss.mss_haussier { "bull" } else { "bear" },
                    level,
                    bar_idx: bi,
                });
            }
        }

        // ── CHOCH ──
        if (out.mss.choch_haussier || out.mss.choch_baissier) && out.mss.choch_level.is_some() {
            if let (Some(level), Some(bi)) = (out.mss.choch_level, out.mss.choch_bar) {
                choch_list.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    dir: if out.mss.choch_haussier { "bull" } else { "bear" },
                    level,
                    bar_idx: bi,
                });
            }
        }

        // ── Sweeps confirmés ──
        if out.sweep.sweep_haussier && out.sweep.sweep_h_level.is_some() {
            if let (Some(level), Some(bi)) = (out.sweep.sweep_h_level, out.sweep.sweep_h_bar) {
                sweeps.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    dir: "bull",
                    level,
                    bar_idx: bi,
                });
            }
        }
        if out.sweep.sweep_baissier && out.sweep.sweep_b_level.is_some() {
            if let (Some(level), Some(bi)) = (out.sweep.sweep_b_level, out.sweep.sweep_b_bar) {
                sweeps.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    dir: "bear",
                    level,
                    bar_idx: bi,
                });
            }
        }
    }

    // ── États actifs (post-replay) ──
    let cal = &engine.calibration;
    let mut obs: Vec<ObOut> = Vec::new();
    for z in engine.order_blocks.bull_zones() {
        let force = ScoringV11::force(engine.scoring_v11.ob_score(true, z.impulse_bar), cal);
        obs.push(ObOut {
            ts: z.timestamp,
            dir: "bull",
            top: z.top,
            bot: z.bot,
            state: ob_state_str(z.state),
            force,
            bar_idx: z.ob_bar,
        });
    }
    for z in engine.order_blocks.bear_zones() {
        let force = ScoringV11::force(engine.scoring_v11.ob_score(false, z.impulse_bar), cal);
        obs.push(ObOut {
            ts: z.timestamp,
            dir: "bear",
            top: z.top,
            bot: z.bot,
            state: ob_state_str(z.state),
            force,
            bar_idx: z.ob_bar,
        });
    }

    let mut fvgs: Vec<FvgOut> = Vec::new();
    for z in engine.fvg.bull_zones() {
        fvgs.push(FvgOut {
            ts: ts_at(&ts_by_idx, z.bar, 0),
            dir: "bull",
            top: z.top,
            bot: z.bot,
            state: fvg_state_str(z.state),
            bar_idx: z.bar,
        });
    }
    for z in engine.fvg.bear_zones() {
        fvgs.push(FvgOut {
            ts: ts_at(&ts_by_idx, z.bar, 0),
            dir: "bear",
            top: z.top,
            bot: z.bot,
            state: fvg_state_str(z.state),
            bar_idx: z.bar,
        });
    }

    // ── Trades ── tous, avec verdict dérivé du lifecycle.
    let mut signals: Vec<SignalOut> = Vec::new();
    for t in &engine.signals.trades {
        signals.push(SignalOut {
            ts: t.open_ts,
            dir: if t.side == Side::Buy { "Long" } else { "Short" },
            entry: t.entry,
            sl: t.sl,
            tp1: t.tp1,
            tp2: t.tp2,
            tp3: t.tp3,
            force: t.score,
            source: match t.source {
                TradeSource::Ob => "v11",
                TradeSource::BsZones => "bszones",
            },
            verdict: verdict_str(t.verdict()),
        });
    }

    // ── FIFO : ne garder que les N derniers visibles ──
    garder_derniers(&mut bos_list, MAX_BOS);
    garder_derniers(&mut pivots, MAX_STRUCTURE);
    garder_derniers(&mut mss_list, MAX_MSS);
    garder_derniers(&mut choch_list, MAX_CHOCH);
    garder_derniers(&mut sweeps, MAX_SWEEPS);
    fvgs = garder_derniers_fvg_par_sens(fvgs, MAX_FVG_PAR_SENS);

    HttpResponse::Ok().json(V12AnalyseResponse {
        asset: asset_str.to_string(),
        timeframe: tf_str.to_string(),
        nb_bougies: bougies.len(),
        pivots,
        bos: bos_list,
        mss: mss_list,
        chochs: choch_list,
        sweeps,
        obs,
        fvgs,
        signals,
        tendance: last_tendance,
        atr14: last_atr,
    })
}

/// Récupère le timestamp d'une bar par son index global (fallback sécurisé).
fn ts_at(ts_by_idx: &[i64], idx: usize, fallback: i64) -> i64 {
    if idx < ts_by_idx.len() {
        ts_by_idx[idx]
    } else {
        fallback
    }
}

/// Garde uniquement les `n` derniers éléments (chronologiques, dernier = le plus récent).
fn garder_derniers<T>(v: &mut Vec<T>, n: usize) {
    if v.len() > n {
        let drain = v.len() - n;
        v.drain(0..drain);
    }
}

/// FVG : limite à `n` par sens (les plus récents = bar_idx le plus grand).
fn garder_derniers_fvg_par_sens(mut fvgs: Vec<FvgOut>, n: usize) -> Vec<FvgOut> {
    fvgs.sort_by_key(|f| (f.dir == "bull", f.bar_idx)); // bull d'abord, puis bar_idx asc
    let mut out: Vec<FvgOut> = Vec::with_capacity(fvgs.len());
    let mut bull_kept = 0usize;
    let mut bear_kept = 0usize;
    // Parcours inversé : on prend les plus récents en premier.
    for f in fvgs.into_iter().rev() {
        let kept = if f.dir == "bull" {
            &mut bull_kept
        } else {
            &mut bear_kept
        };
        if *kept < n {
            out.push(f);
            *kept += 1;
        }
    }
    out
}

fn ob_state_str(s: ObState) -> &'static str {
    match s {
        ObState::Vierge => "vierge",
        ObState::Partiel => "partiel",
        ObState::Profond => "profond",
    }
}

fn fvg_state_str(s: FvgState) -> &'static str {
    match s {
        FvgState::Fresh => "vierge",
        FvgState::Partial => "partiel",
    }
}

fn verdict_str(v: Verdict) -> &'static str {
    match v {
        Verdict::Tp3 => "TP3",
        Verdict::Tp2 => "TP2",
        Verdict::Tp1 => "TP1",
        Verdict::Sl => "SL",
        Verdict::Be => "BE",
        Verdict::Expire => "Expire",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garder_derniers_tronque_tete() {
        let mut v: Vec<i32> = (0..10).collect();
        garder_derniers(&mut v, 3);
        assert_eq!(v, vec![7, 8, 9]);
    }

    #[test]
    fn garder_derniers_sous_limet_inchange() {
        let mut v = vec![1, 2];
        garder_derniers(&mut v, 5);
        assert_eq!(v, vec![1, 2]);
    }

    #[test]
    fn fvg_par_sens_limite_a_n() {
        let fvgs = vec![
            FvgOut { ts: 0, dir: "bull", top: 1.0, bot: 0.0, state: "vierge", bar_idx: 1 },
            FvgOut { ts: 0, dir: "bull", top: 1.0, bot: 0.0, state: "vierge", bar_idx: 2 },
            FvgOut { ts: 0, dir: "bear", top: 1.0, bot: 0.0, state: "vierge", bar_idx: 3 },
        ];
        let out = garder_derniers_fvg_par_sens(fvgs, 1);
        let bulls = out.iter().filter(|f| f.dir == "bull").count();
        let bears = out.iter().filter(|f| f.dir == "bear").count();
        assert_eq!(bulls, 1);
        assert_eq!(bears, 1);
        // Le bull conservé doit être le plus récent (bar_idx=2).
        assert!(out.iter().any(|f| f.dir == "bull" && f.bar_idx == 2));
    }

    #[test]
    fn ts_at_fallback_si_hors_plage() {
        let t = ts_at(&[10, 20, 30], 5, 99);
        assert_eq!(t, 99);
        assert_eq!(ts_at(&[10, 20, 30], 1, 99), 20);
    }
}
