//! Handler HTTP pour l'analyse SMC v12 (replay bar-par-bar du moteur complet).
//!
//! `GET /api/smc/v12/analyse?asset=XAUUSD&timeframe=M15&limit=500`
//!
//! Reconstruit l'indicateur Pine `smc_indicateur_v12.pine` en rejouant toutes les
//! bougies clôturées depuis la DB via `SmcV12Engine::update`. Collecte les
//! événements (pivots/BOS/MSS/CHOCH/sweeps) + les états actifs (OB/FVG vivants) +
//! tous les trades générés (avec leur verdict), puis applique les limites d'affichage
//! FIFO (comme TradingView : on ne garde que les N derniers visibles).
//!
//! Les indicateurs étendus (liquidités PDH/PDL/PWH/PWL + EQH/EQL, breaker, imbalance,
//! OTE, Premium/Discount, MTF, NDOG/NWOG, sessions Kill Zones, Asian HL, bgcolor
//! volume/impulsion, zone-cœur) sont collectés et sérialisés via
//! [`crate::smc_v12_out`] (champ `extended` aplati dans la réponse JSON).

use actix_web::{web, HttpResponse, Responder};
use smc::v12::trade::{Side, TradeSource, TradeState};
use smc::v12::{BarInput, ScoringV11, SmcV12Engine};

use crate::smc_v12_collect::{collect_final_extended, BarCollectors};
use crate::smc_v12_out::*;
use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};

/// GET /api/smc/v12/analyse?asset=XAUUSD&timeframe=M15&limit=500
///
/// Replay complet du moteur v12 sur les `limit` dernières bougies clôturées.
/// Amorce MTF : charge H1/H4/W1 (+ D1 agrégée en MN) depuis la DB et convertit
/// en `BarInput` pour `engine.primer_mtf`. Échec DB → listes vides (replay
/// dégradé façon ancienne, jamais d'erreur 500).
pub(crate) async fn charger_amorce_mtf(
    db: &db::Database,
    asset: &common::Asset,
) -> (Vec<BarInput>, Vec<BarInput>, Vec<BarInput>, Vec<BarInput>) {
    use common::Timeframe;
    const MAX_BARS: i64 = 600; // = MAX_HTF_BARS du moteur (FIFO)

    let vers_bars = |bougies: Vec<common::Candle>| -> Vec<BarInput> {
        bougies
            .into_iter()
            .map(|b| BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            })
            .collect()
    };

    let h1 = db
        .obtenir_bougies(asset, &Timeframe::H1, MAX_BARS)
        .await
        .unwrap_or_default();
    let h4 = db
        .obtenir_bougies(asset, &Timeframe::H4, MAX_BARS)
        .await
        .unwrap_or_default();
    let w1 = db
        .obtenir_bougies(asset, &Timeframe::W1, MAX_BARS)
        .await
        .unwrap_or_default();
    // D1 plus profond (2000 ≈ 5,5 ans) : agrégée en MN, la profondeur
    // mensuelle alimente la confluence +6 (FIFO 600 du moteur la borne).
    let d1 = db
        .obtenir_bougies(asset, &Timeframe::D1, 2000)
        .await
        .unwrap_or_default();

    (
        vers_bars(h1),
        vers_bars(h4),
        vers_bars(w1),
        smc::v12::agreger_mensuel(&vers_bars(d1)),
    )
}

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
            bprs: Vec::new(),
            signals: Vec::new(),
            tendance: "neutre",
            atr14: 0.0,
            extended: ExtendedOutputs::default(),
        });
    }

    // Replay bar-par-bar.
    let mut engine = SmcV12Engine::new(asset_str, tf_str)
        // Aligné sur le moteur de production (décision 26/08 — étude comparatif_be).
        .avec_mode_be_force(smc::v12::lifecycle::ModeBeForce::Supprime);
    // Amorçage MTF (H1/H4/W1/MN de la DB) : sans lui, f_htf ne verrait que la
    // fenêtre LTF (~52 j en M15×5000) et les confluences W1 (+5) / MN (+6) du
    // scoring seraient structurellement froides (Pine/TV : années d'historique).
    if let Some(premiere) = bougies.first() {
        let t0 = premiere.timestamp.timestamp();
        let (h1, h4, w1, mn) = charger_amorce_mtf(&state.db, &asset).await;
        engine.primer_mtf(&h1, &h4, &w1, &mn, t0);
    }
    let mut ts_by_idx: Vec<i64> = Vec::with_capacity(bougies.len());
    let mut pivots: Vec<PivotOut> = Vec::new();
    let mut bos_list: Vec<NiveauStructOut> = Vec::new();
    let mut mss_list: Vec<NiveauStructOut> = Vec::new();
    let mut choch_list: Vec<NiveauStructOut> = Vec::new();
    let mut sweeps: Vec<NiveauStructOut> = Vec::new();

    let mut last_atr = 0.0_f64;
    let mut last_tendance = "neutre";
    // Collecteurs des indicateurs étendus par barre (sessions/volume/impulsion/
    // zone-cœur/Asian HL). Le seuil d'impulsion est lu sur la calibration.
    let mut col = BarCollectors::new(
        bougies.len(),
        engine.calibration.seuil_ib,
        engine.calibration.atr_seuil_affichage(),
    );

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
                let bullish = out.bos.bullish;
                bos_list.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    pivot_ts: dernier_pivot_ts(&pivots, bullish),
                    dir: if bullish { "bull" } else { "bear" },
                    level,
                    bar_idx: bi,
                    candle_high: None,
                    candle_low: None,
                });
            }
        }

        // ── MSS ──
        if (out.mss.mss_haussier || out.mss.mss_baissier) && out.mss.mss_level.is_some() {
            if let (Some(level), Some(bi)) = (out.mss.mss_level, out.mss.mss_bar) {
                let bullish = out.mss.mss_haussier;
                mss_list.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    pivot_ts: dernier_pivot_ts(&pivots, bullish),
                    dir: if bullish { "bull" } else { "bear" },
                    level,
                    bar_idx: bi,
                    candle_high: None,
                    candle_low: None,
                });
            }
        }

        // ── CHOCH ──
        if (out.mss.choch_haussier || out.mss.choch_baissier) && out.mss.choch_level.is_some() {
            if let (Some(level), Some(bi)) = (out.mss.choch_level, out.mss.choch_bar) {
                let bullish = out.mss.choch_haussier;
                choch_list.push(NiveauStructOut {
                    ts: ts_at(&ts_by_idx, bi, bar.timestamp),
                    pivot_ts: dernier_pivot_ts(&pivots, bullish),
                    dir: if bullish { "bull" } else { "bear" },
                    level,
                    bar_idx: bi,
                    candle_high: None,
                    candle_low: None,
                });
            }
        }

        // ── Sweeps confirmés ── (événement ponctuel : pivot_ts = ts, pas de ligne bornée)
        if out.sweep.sweep_haussier && out.sweep.sweep_h_level.is_some() {
            if let (Some(level), Some(bi)) = (out.sweep.sweep_h_level, out.sweep.sweep_h_bar) {
                let ts = ts_at(&ts_by_idx, bi, bar.timestamp);
                sweeps.push(NiveauStructOut {
                    ts,
                    pivot_ts: ts,
                    dir: "bull",
                    level,
                    bar_idx: bi,
                    candle_high: Some(bar.high),
                    candle_low: Some(bar.low),
                });
            }
        }
        if out.sweep.sweep_baissier && out.sweep.sweep_b_level.is_some() {
            if let (Some(level), Some(bi)) = (out.sweep.sweep_b_level, out.sweep.sweep_b_bar) {
                let ts = ts_at(&ts_by_idx, bi, bar.timestamp);
                sweeps.push(NiveauStructOut {
                    ts,
                    pivot_ts: ts,
                    dir: "bear",
                    level,
                    bar_idx: bi,
                    candle_high: Some(bar.high),
                    candle_low: Some(bar.low),
                });
            }
        }

        // ── Indicateurs étendus par barre ──
        col.on_bar(&bar, &out);
    }

    // ── États actifs (post-replay) : Order Blocks + FVG ──
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
            diag: engine
                .scoring_v11
                .ob_diag(true, z.impulse_bar)
                .map(str::to_string),
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
            diag: engine
                .scoring_v11
                .ob_diag(false, z.impulse_bar)
                .map(str::to_string),
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

    // ── BPR (MODULE 6b) — zones conservées (actives + figées, FIFO 20) ──
    // Ancrage gauche = bar_index[2] du gap récent (Pine box.new(bar_index[2], …))
    // = bar de naissance − 2, comme les boxes FVG.
    let mut bprs: Vec<BprOut> = Vec::new();
    for z in engine.bpr.zones() {
        bprs.push(BprOut {
            ts: ts_at(&ts_by_idx, z.bar.saturating_sub(2), 0),
            dir: if z.is_bull { "bull" } else { "bear" },
            top: z.top,
            bot: z.bot,
            ce: (z.top + z.bot) / 2.0,
            state: bpr_state_str(z.state),
            dead: z.dead,
            bar_idx: z.bar,
        });
    }

    // ── Trades ── tous, avec verdict dérivé du lifecycle + filtre sentiment.
    // Le sentiment courant (approximation : même score pour tous les trades du replay)
    // sert à qualifier l'alignement direction × sentiment de chaque signal.
    let sentiment_snap = state.sentiment.read().await.clone();
    // Label f_lblTrade (Pine 2503) : lot = (capital × risque) / (R / pip ×
    // valeur_pip) — capital/risque de la stratégie SMC (registre), conventions
    // pips de l'actif (onglet gestion du risque). Mêmes sources que Telegram.
    let reg_smc = state.db.lire_strategie("SMC").await.ok().flatten();
    let (mut capital, risque_pct) = reg_smc
        .as_ref()
        .map(|r| (r.capital, r.risque_pct))
        .unwrap_or((0.0, 1.0));
    // Capital SIMULÉ courant (composé à chaque clôture) — le label du trade
    // reflète le lot tel qu'il serait calculé maintenant, comme Telegram.
    if let Some(c) = crate::capital_simule::capital_actuel(&state.db, "SMC").await {
        capital = c;
    }
    let (taille_pip, valeur_pip) = db::asset_params::lire_un(state.db.pool(), asset_str)
        .await
        .ok()
        .flatten()
        .map(|p| (p.taille_pip, p.valeur_pips))
        .unwrap_or((1.0, 1.0));
    let pips = |a: f64, b: f64| ((a - b).abs() / taille_pip).round() as i64;
    let mut signals: Vec<SignalOut> = Vec::new();
    for t in &engine.signals.trades {
        let dir_str = if t.side == Side::Buy { "Long" } else { "Short" };
        let (sentiment_score, alignement) = match &sentiment_snap {
            Some(sc) => {
                let v = crate::sentiment_filter::filtrer_par_sentiment(dir_str, asset_str, sc);
                (Some(v.score_classe), Some(nom_alignement(v.alignement)))
            }
            None => (None, None),
        };
        // f_lblTrade — le lot suit le SL COURANT (Pine 4009 : recalcul au BE).
        let risque_euros = capital * risque_pct / 100.0;
        let r_courant = (t.entry - t.sl).abs();
        let lot = if r_courant > 0.0 && valeur_pip > 0.0 && taille_pip > 0.0 {
            risque_euros / (r_courant / taille_pip * valeur_pip)
        } else {
            0.0
        };
        let label = vec![
            format!(
                "{} - force {}/10",
                if t.side == Side::Buy { "BUY" } else { "SELL" },
                ScoringV11::force(t.score, &engine.calibration)
            ),
            format!("→ {:.2} lots ({:.0}$ risqués)", lot, risque_euros),
            format!("Entrée {:.2}$", t.entry),
            format!("SL {:.2}$ soit {} pips", t.sl, pips(t.sl, t.entry)),
            format!("TP1 {:.2}$ soit {} pips", t.tp1, pips(t.tp1, t.entry)),
            format!("TP2 {:.2}$ soit {} pips", t.tp2, pips(t.tp2, t.entry)),
            format!("TP3 {:.2}$ soit {} pips", t.tp3, pips(t.tp3, t.entry)),
        ];
        signals.push(SignalOut {
            ts: t.open_ts,
            dir: dir_str,
            entry: t.entry,
            sl: t.sl,
            tp1: t.tp1,
            tp2: t.tp2,
            tp3: t.tp3,
            // Pine affiche f_force (bandes /10, bornée 1-10), pas le score brut.
            force: ScoringV11::force(t.score, &engine.calibration),
            source: match t.source {
                TradeSource::Ob => "v11",
                TradeSource::BsZones => "bszones",
            },
            verdict: verdict_str(t.verdict()),
            filled: t.filled,
            fill_ts: t.fill_ts,
            be: t.tp1_hit,
            ferme: t.state == TradeState::Closed,
            label,
            sentiment: sentiment_score,
            alignement,
        });
    }

    // ── Indicateurs étendus : états finaux + compression des séries par barre ──
    let mut extended = collect_final_extended(&engine, &ts_by_idx, col);
    // EQH/EQL : bord gauche = timestamp du 1er pivot (Pine tFirst).
    let dernier_ts = ts_by_idx.last().copied().unwrap_or(0);
    for eq in extended.eqs.iter_mut() {
        eq.ts = ts_at(&ts_by_idx, eq.bar_idx, dernier_ts);
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
        bprs,
        signals,
        tendance: last_tendance,
        atr14: last_atr,
        extended,
    })
}

/// Nom sérialisable d'un `Alignement` de sentiment (pour le replay frontend).
fn nom_alignement(a: smc::v12::sentiment::Alignement) -> &'static str {
    use smc::v12::sentiment::Alignement;
    match a {
        Alignement::Aligne => "aligne",
        Alignement::Oppose => "oppose",
        Alignement::Neutre => "neutre",
        Alignement::Extreme => "extreme",
    }
}

/// Timestamp du dernier pivot swing cassé : borne de DÉBUT des lignes BOS/MSS/CHOCH.
///
/// `high = true` → cherche le dernier swing high (HH/LH), `high = false` → dernier
/// swing low (HL/LL). Renvoie `0` si aucun pivot du type voulu n'a encore été collecté
/// (le frontend traite alors la ligne comme un point unique à la cassure).
fn dernier_pivot_ts(pivots: &[PivotOut], high: bool) -> i64 {
    pivots
        .iter()
        .rev()
        .filter(|p| {
            if high {
                p.ptype == "HH" || p.ptype == "LH"
            } else {
                p.ptype == "HL" || p.ptype == "LL"
            }
        })
        .next()
        .map(|p| p.ts)
        .unwrap_or(0)
}
