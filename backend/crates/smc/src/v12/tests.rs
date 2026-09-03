//! Tests d'intégration du moteur SMC v12 sur les 700 bars XAUUSD M15 du spike.
//!
//! Le test principal affiche (println) le compte de pivots et BOS détectés pour
//! comparaison visuelle avec TradingView (mêmes labels HH/HL/BOS sur XAUUSD M15).

use super::*;
use std::io::BufRead;

const XAUUSD_M15_CSV: &str = "/mnt/IA/nautilus-smc-spike/xauusd_m15.csv";

/// Charge le CSV (timestamp,open,high,low,close,volume) en `BarInput`.
/// Robuste : ignore lignes mal formées (aucun panic/unwrap).
fn load_xauusd_m15() -> Vec<BarInput> {
    let file = match std::fs::File::open(XAUUSD_M15_CSV) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut bars = Vec::new();
    for line in std::io::BufReader::new(file).lines().skip(1) {
        let Ok(l) = line else { continue };
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 6 {
            continue;
        }
        let parsed = (
            f[0].parse::<i64>().ok(),
            f[1].parse::<f64>().ok(),
            f[2].parse::<f64>().ok(),
            f[3].parse::<f64>().ok(),
            f[4].parse::<f64>().ok(),
            f[5].parse::<f64>().ok(),
        );
        let (Some(timestamp), Some(open), Some(high), Some(low), Some(close), Some(volume)) =
            parsed
        else {
            continue;
        };
        bars.push(BarInput {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        });
    }
    bars
}

#[test]
fn engine_traite_700_bars_xauusd_sans_panic() {
    let bars = load_xauusd_m15();
    assert_eq!(bars.len(), 700, "Le CSV doit contenir 700 bars XAUUSD M15");

    let mut engine = SmcV12Engine::new("XAUUSD", "M15");
    assert_eq!(engine.calibration.swing_length, 3);
    assert_eq!(engine.tf_sec(), 900, "M15 ⇒ 900 s");

    let (mut ph, mut pl, mut bos_h, mut bos_b) = (0u32, 0u32, 0u32, 0u32);
    let (mut mss_h, mut mss_b, mut choch_h, mut choch_b) = (0u32, 0u32, 0u32, 0u32);
    let (mut swp_h, mut swp_b, mut swp_pdh, mut swp_pdl) = (0u32, 0u32, 0u32, 0u32);
    let (mut eqh, mut eql) = (0u32, 0u32);
    // Compteurs cumulés de création (MODULES 6/7/8b/8c/13b).
    let (mut fvg_bull, mut fvg_bear) = (0u32, 0u32);
    let (mut ob_bull, mut ob_bear) = (0u32, 0u32);
    let (mut brk_bull, mut brk_bear) = (0u32, 0u32);
    let (mut prop_bull, mut prop_bear) = (0u32, 0u32);
    let (mut ib_bull, mut ib_bear) = (0u32, 0u32);
    // Compteurs Phase 2.3 (contexte).
    let (mut pd_prem, mut pd_disc, mut ote_bull, mut ote_bear) = (0u32, 0u32, 0u32, 0u32);
    let (mut kz, mut ndog_new, mut nwog_new) = (0u32, 0u32, 0u32);
    let (mut conf_h1, mut conf_h4, mut conf_w1, mut conf_mn) = (0u32, 0u32, 0u32, 0u32);
    let (mut coeur_bull, mut coeur_bear) = (0u32, 0u32);
    let mut last_sh1: Option<f64> = None;
    let mut last_sl1: Option<f64> = None;
    let mut atr_final = 0.0_f64;
    let mut last_pdha: Option<f64> = None;
    let mut last_pwha: Option<f64> = None;
    for bar in &bars {
        let out = engine.update(bar);
        if out.pivot.is_pivot_high {
            ph += 1;
        }
        if out.pivot.is_pivot_low {
            pl += 1;
        }
        if out.bos.bullish {
            bos_h += 1;
        }
        if out.bos.bearish {
            bos_b += 1;
        }
        if out.mss.mss_haussier {
            mss_h += 1;
        }
        if out.mss.mss_baissier {
            mss_b += 1;
        }
        if out.mss.choch_haussier {
            choch_h += 1;
        }
        if out.mss.choch_baissier {
            choch_b += 1;
        }
        if out.sweep.sweep_haussier {
            swp_h += 1;
        }
        if out.sweep.sweep_baissier {
            swp_b += 1;
        }
        if out.liquidite.sweep_pdh {
            swp_pdh += 1;
        }
        if out.liquidite.sweep_pdl {
            swp_pdl += 1;
        }
        if out.liquidite.is_eqh {
            eqh += 1;
        }
        if out.liquidite.is_eql {
            eql += 1;
        }
        // MODULES 6/7/8b/8c/13b — comptage des zones créées.
        if out.fvg.new_bull.is_some() {
            fvg_bull += 1;
        }
        if out.fvg.new_bear.is_some() {
            fvg_bear += 1;
        }
        if out.order_blocks.new_bull.is_some() {
            ob_bull += 1;
        }
        if out.order_blocks.new_bear.is_some() {
            ob_bear += 1;
        }
        for bz in &out.breaker.created {
            if bz.bull {
                brk_bull += 1;
            } else {
                brk_bear += 1;
            }
        }
        prop_bull += out.propulsion.new_bull.len() as u32;
        prop_bear += out.propulsion.new_bear.len() as u32;
        if out.imbalance.new_bull.is_some() {
            ib_bull += 1;
        }
        if out.imbalance.new_bear.is_some() {
            ib_bear += 1;
        }
        // Phase 2.3 — contexte.
        if out.premium_discount.in_premium {
            pd_prem += 1;
        }
        if out.premium_discount.in_discount {
            pd_disc += 1;
        }
        if out.ote.in_ote_bull {
            ote_bull += 1;
        }
        if out.ote.in_ote_bear {
            ote_bear += 1;
        }
        if out.kill_zone.in_kz {
            kz += 1;
        }
        if out.ndog.new_ndog.is_some() {
            ndog_new += 1;
        }
        if out.ndog.new_nwog.is_some() {
            nwog_new += 1;
        }
        if out.mtf.confluence_h1 {
            conf_h1 += 1;
        }
        if out.mtf.confluence_h4 {
            conf_h4 += 1;
        }
        if out.mtf.confluence_w1 {
            conf_w1 += 1;
        }
        if out.mtf.confluence_mn {
            conf_mn += 1;
        }
        coeur_bull += out.zone_coeur.bull.len() as u32;
        coeur_bear += out.zone_coeur.bear.len() as u32;
        if out.sh1.is_some() {
            last_sh1 = out.sh1;
        }
        if out.sl1.is_some() {
            last_sl1 = out.sl1;
        }
        if out.liquidite.pdh_active.is_some() {
            last_pdha = out.liquidite.pdh_active;
        }
        if out.liquidite.pwh_active.is_some() {
            last_pwha = out.liquidite.pwh_active;
        }
        atr_final = out.atr14;
    }

    let total_pivots = ph + pl;
    let total_bos = bos_h + bos_b;
    let total_mss = mss_h + mss_b;
    let total_choch = choch_h + choch_b;
    let total_swp = swp_h + swp_b;
    println!(
        "\n===== SMC v12 — 700 bars XAUUSD M15 =====\n\
         Pivots high      : {ph}\n\
         Pivots low       : {pl}\n\
         TOTAL pivots     : {total_pivots}\n\
         BOS haussiers    : {bos_h}\n\
         BOS baissiers    : {bos_b}\n\
         TOTAL BOS        : {total_bos}\n\
         MSS haussiers    : {mss_h}\n\
         MSS baissiers    : {mss_b}\n\
         TOTAL MSS        : {total_mss}\n\
         CHOCH haussiers  : {choch_h}\n\
         CHOCH baissiers  : {choch_b}\n\
         TOTAL CHOCH      : {total_choch}\n\
         EQH              : {eqh}\n\
         EQL              : {eql}\n\
         Sweeps PDH       : {swp_pdh}\n\
         Sweeps PDL       : {swp_pdl}\n\
         Sweep haussiers  : {swp_h}\n\
         Sweep baissiers  : {swp_b}\n\
         TOTAL sweeps     : {total_swp}\n\
         ATR14 final      : {atr_final:.4}\n\
         sh1 final        : {last_sh1:?}\n\
         sl1 final        : {last_sl1:?}\n\
         PDH active       : {last_pdha:?}\n\
         PWH active       : {last_pwha:?}\n\
         tendance hauss.  : {}\n\
         tendance baiss.  : {}\n\
         ==========================================",
        engine.structure.tendance_haussiere(),
        engine.structure.tendance_baissiere(),
    );

    // Comptes actifs (zones encore vivantes à la dernière bar) + cumuls créés.
    let fvg_act = engine.fvg.bull_zones().len() + engine.fvg.bear_zones().len();
    let ob_act = engine.order_blocks.bull_zones().len() + engine.order_blocks.bear_zones().len();
    let brk_act = engine.breaker.bull_zones().len() + engine.breaker.bear_zones().len();
    let prop_act = engine.propulsion.bull_zones().len() + engine.propulsion.bear_zones().len();
    let ib_act = engine.imbalance.bull_zones().len() + engine.imbalance.bear_zones().len();
    println!(
        "\n===== MODULES 6/7/8b/8c/13b — ZONES (700 bars XAUUSD M15) =====\n\
         FVG      : créés bull={fvg_bull} bear={fvg_bear} | actifs={fvg_act}\n\
         OB       : créés bull={ob_bull} bear={ob_bear} | actifs={ob_act}\n\
         Breaker  : créés bull={brk_bull} bear={brk_bear} | actifs={brk_act}\n\
         Propuls. : créés bull={prop_bull} bear={prop_bear} | actifs={prop_act}\n\
         Imbalan. : créés bull={ib_bull} bear={ib_bear} | actifs={ib_act}\n\
         ================================================================"
    );

    println!(
        "\n===== PHASE 2.3 — CONTEXTE (700 bars XAUUSD M15) =====\n\
         Premium/Discount : premium={pd_prem} discount={pd_disc}\n\
         OTE              : inOTE bull={ote_bull} bear={ote_bear} (expiry {} bars)\n\
         Kill Zones       : bars inKZ={kz}\n\
         NDOG/NWOG        : NDOG créés={ndog_new} NWOG créés={nwog_new}\n\
         MTF confluences  : H1={conf_h1} H4={conf_h4} W1={conf_w1} MN={conf_mn}\n\
         Zone-cœur        : bull={coeur_bull} bear={coeur_bear}\n\
         ========================================================",
        engine.ote.expiry_bars(),
    );

    assert!(total_pivots > 0, "Doit détecter des pivots sur 700 bars");
    assert!(total_bos > 0, "Doit détecter des BOS sur 700 bars");
    assert!(atr_final > 0.0, "ATR14 doit être calculé");
    // Au moins un EQH/EQL ou sweep sur 700 bars XAUUSD M15 (très volatil).
    assert!(
        eqh + eql + total_swp > 0,
        "Doit détecter au moins une liquidité/sweep sur 700 bars"
    );
    // Les nouveaux modules doivent produire des zones sur 700 bars XAUUSD M15.
    assert!(
        fvg_bull + fvg_bear > 0,
        "Doit détecter au moins un FVG sur 700 bars"
    );
    assert!(
        ob_bull + ob_bear > 0,
        "Doit détecter au moins un Order Block sur 700 bars"
    );
    assert!(
        ib_bull + ib_bear > 0,
        "Doit détecter au moins une Imbalance sur 700 bars"
    );
    // FIFO respectés.
    assert!(
        engine.fvg.bull_zones().len() <= 10 && engine.fvg.bear_zones().len() <= 10,
        "FVG FIFO ≤ 10 par sens"
    );
    assert!(
        engine.order_blocks.bull_zones().len() <= 40
            && engine.order_blocks.bear_zones().len() <= 40,
        "OB FIFO ≤ 40 par sens"
    );
    assert!(
        engine.breaker.bull_zones().len() <= 5 && engine.breaker.bear_zones().len() <= 5,
        "Breaker FIFO ≤ 5 par sens"
    );
    assert!(
        engine.propulsion.bull_zones().len() <= 3 && engine.propulsion.bear_zones().len() <= 3,
        "Propulsion FIFO ≤ 3 par sens"
    );
    assert!(
        engine.imbalance.bull_zones().len() <= 10 && engine.imbalance.bear_zones().len() <= 10,
        "Imbalance FIFO ≤ 10 par sens"
    );

    // --- Phase 2.3 : assertions contexte ---
    // Premium/Discount : au moins un BOS sur 700 bars ⇒ la plage doit être capturée à la fin.
    assert!(
        engine.premium_discount.last_event().pd_range_h.is_some(),
        "PD : plage capturée au dernier BOS (≥1 BOS sur 700 bars)"
    );
    assert!(
        engine.premium_discount.last_event().equilibrium.is_some(),
        "PD : equilibrium calculé"
    );
    // PD premium+discount couvre une partie des 700 bars (sans panic, sans tout couvrir).
    assert!(
        (pd_prem + pd_disc) > 0 && (pd_prem + pd_disc) < 700,
        "PD : au moins une bar classée, mais pas toutes (zone tampon equilibrium)"
    );
    // OTE : expiration M15 = 12 bars.
    assert_eq!(engine.ote.expiry_bars(), 12, "OTE_EXPIRY_BARS M15 = 12");
    // Kill Zones : les 700 bars M15 XAUUSD couvrent plusieurs jours UTC ⇒ KZ non vide.
    assert!(kz > 0, "Kill Zones : au moins une bar en KZ sur 700 bars");
    // NDOG : M15 ⇒ tf_ndog actif ; sur plusieurs jours, des gaps se créent (ou aucun si
    // marché continu, mais le mécanisme tourne sans panic). On bornit le compteur.
    assert!(ndog_new <= 700, "NDOG compteur cohérent");
    // MTF : H1 doit produire au moins une confluence sur 700 bars M15 (≈175 bars H1,
    // suffisamment de pivots/OB). W1/MN peuvent rester à 0 (fenêtre trop courte).
    assert!(
        conf_h1 > 0,
        "MTF : au moins une confluence H1 sur 700 bars M15"
    );
    // Zone-cœur : pas de panic ; borne supérieure lâche (détection stricte).
    assert!(
        coeur_bull + coeur_bear <= 700,
        "Zone-cœur : compteur cohérent"
    );
}

/// Phase 2.5 — Test d'intégration du CERVEAU : scoring + signaux + lifecycle
/// sur les 700 bars XAUUSD M15. Affiche le nombre de signaux v11 vs BSZones, les
/// verdicts (TP1/TP2/TP3/SL/BE/Expire) et les stats (win rate, R moyen).
#[test]
fn engine_genere_signaux_et_lifecycle_700_bars() {
    let bars = load_xauusd_m15();
    assert_eq!(bars.len(), 700, "Le CSV doit contenir 700 bars XAUUSD M15");

    let mut engine = SmcV12Engine::new("XAUUSD", "M15");
    for bar in &bars {
        let _ = engine.update(bar);
    }

    use crate::v12::trade::{CloseReason, TradeSource, TradeState, Verdict};
    let trades = &engine.signals.trades;

    let total = trades.len();
    let v11 = trades
        .iter()
        .filter(|t| t.source == TradeSource::Ob)
        .count();
    let bs = trades
        .iter()
        .filter(|t| t.source == TradeSource::BsZones)
        .count();
    let closed = trades
        .iter()
        .filter(|t| t.state == TradeState::Closed)
        .count();
    let open = total - closed;

    // Verdicts (sur clôturés).
    let mut n_tp1 = 0usize;
    let mut n_tp2 = 0usize;
    let mut n_tp3 = 0usize;
    let mut n_sl = 0usize;
    let mut n_be = 0usize;
    let mut n_expire = 0usize;
    let mut n_cancel = 0usize;
    for t in trades {
        if t.state != TradeState::Closed {
            continue;
        }
        match t.close_reason {
            Some(CloseReason::Cancel) => n_cancel += 1,
            _ => match t.verdict() {
                Verdict::Tp3 => n_tp3 += 1,
                Verdict::Tp2 => n_tp2 += 1,
                Verdict::Tp1 => n_tp1 += 1,
                Verdict::Sl => n_sl += 1,
                Verdict::Be => n_be += 1,
                Verdict::Expire => n_expire += 1,
                Verdict::Ts => n_expire += 1, // TS compté côté expire pour les stats
                _ => {}
            },
        }
    }

    // Win rate / R moyen sur trades clôturés réellement exécutés (exclut Cancel =
    // ordre jamais rempli). Win = TP1/TP2/TP3.
    let executed: Vec<&Trade> = trades
        .iter()
        .filter(|t| t.state == TradeState::Closed && t.close_reason != Some(CloseReason::Cancel))
        .collect();
    let wins = executed.iter().filter(|t| t.is_win()).count();
    let win_rate = if executed.is_empty() {
        0.0
    } else {
        wins as f64 / executed.len() as f64 * 100.0
    };
    let r_mean = if executed.is_empty() {
        0.0
    } else {
        executed.iter().map(|t| t.realized_r()).sum::<f64>() / executed.len() as f64
    };
    let r_total = executed.iter().map(|t| t.realized_r()).sum::<f64>();
    let n_exec = executed.len();
    let s_wr = format!("{:.1}", win_rate);
    let s_rm = format!("{:+.3}", r_mean);
    let s_rt = format!("{:+.2}", r_total);

    // Diagnostic BSZones : zones encore actives + derniers scores + tendance HTF finale.
    let bs_bull = engine.scoring_bs.bull_zones();
    let bs_bear = engine.scoring_bs.bear_zones();
    let bs_bull_max = bs_bull.iter().map(|z| z.score).max().unwrap_or(-1);
    let bs_bear_max = bs_bear.iter().map(|z| z.score).max().unwrap_or(-1);
    let (bs_births_bull, bs_births_bear) = engine.scoring_bs.total_births();
    let (bs_ds_bull, bs_ds_bear) = engine.scoring_bs.total_disp_sweep();
    let (mb_bull, mb_bear, bok_bull, bok_bear) = engine.scoring_bs.gate_diag();
    let h1t = engine.mtf.last_event().h1.trend;
    let h4t = engine.mtf.last_event().h4.trend;

    println!(
        "\n===== PHASE 2.5 — CERVEAU (700 bars XAUUSD M15) =====\n\
         Trades créés      : {total}  (v11={v11} · BSZones={bs})\n\
         Clôturés / ouverts: {closed} / {open}\n\
         --- VERDICTS (clôturés) ---\n\
         TP3   : {n_tp3}\n\
         TP2   : {n_tp2}\n\
         TP1   : {n_tp1}\n\
         SL    : {n_sl}\n\
         BE    : {n_be}\n\
         Expire: {n_expire}\n\
         Cancel: {n_cancel}  (ordre jamais rempli)\n\
         --- STATS (executes, hors Cancel) ---\n\
         Win rate : {s_wr}%  ({wins}/{n_exec})\n\
         R moyen  : {s_rm}R\n\
         R total  : {s_rt}R\n\
         --- DIAGNOSTIC BSZones ---\n\
         Zones actives     : bull={} bear={}  (score max bull={} bear={})\n\
         Zones nées (cumul): bull={bs_births_bull} bear={bs_births_bear}\n\
         disp+sweep (cumul): bull={bs_ds_bull} bear={bs_ds_bear}  (pré-gate baseScore/HTF)\n\
         max baseScore     : bull={mb_bull} bear={mb_bear}  (base≥6 : bull={bok_bull} bear={bok_bear})\n\
         HTF trend final   : h1={} h4={}\n\
         =====================================================",
        bs_bull.len(),
        bs_bear.len(),
        bs_bull_max,
        bs_bear_max,
        h1t,
        h4t,
    );

    // --- Assertions de bon sens (pas de panic, lifecycle cohérent) ---
    assert!(
        total < 700,
        "Nettement moins de trades que de bars (anti-doublon)"
    );
    assert_eq!(v11 + bs, total, "source v11 + BS = total");
    assert_eq!(
        n_tp1 + n_tp2 + n_tp3 + n_sl + n_be + n_expire + n_cancel,
        closed,
        "somme des verdicts = clôturés"
    );
    // 1 trade max par bar (anti-doublon) : le nombre de trades ≤ nombre de bars.
    assert!(total <= 700);
    // Tout trade fermé a un close_reason et un R.
    for t in trades {
        if t.state == TradeState::Closed {
            assert!(t.close_reason.is_some(), "trade fermé sans close_reason");
            assert!(t.close_r.is_some(), "trade fermé sans close_r");
        }
    }
}
