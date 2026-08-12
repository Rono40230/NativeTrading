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
        let (Some(timestamp), Some(open), Some(high), Some(low), Some(close), Some(volume)) = parsed
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

    assert!(total_pivots > 0, "Doit détecter des pivots sur 700 bars");
    assert!(total_bos > 0, "Doit détecter des BOS sur 700 bars");
    assert!(atr_final > 0.0, "ATR14 doit être calculé");
    // Au moins un EQH/EQL ou sweep sur 700 bars XAUUSD M15 (très volatil).
    assert!(
        eqh + eql + total_swp > 0,
        "Doit détecter au moins une liquidité/sweep sur 700 bars"
    );
}
