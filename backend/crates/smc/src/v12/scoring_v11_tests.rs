//! Tests du scoring v11 (extraits de scoring_v11.rs — limite 600 lignes).

#[cfg(test)]
use super::*;

use super::*;
use crate::v12::calibration::AssetCalibration;

fn cal_xau() -> AssetCalibration {
    AssetCalibration::detect("XAUUSD", "M15")
}

#[test]
fn force_xau_4_bandes() {
    let c = cal_xau();
    // XAU : SEUIL_MOYEN=7, FORT=10, INSTIT=12, scoreMax=13.
    // sc=0 → 1+0 = 1.
    assert_eq!(ScoringV11::force(0, &c), 1);
    // sc=7 (==moyen) → 2e bande : 5 + (7-7)/3 = 5.
    assert_eq!(ScoringV11::force(7, &c), 5);
    // sc=10 (==fort) → 3e bande : 7 + (10-10)/2 = 7.
    assert_eq!(ScoringV11::force(10, &c), 7);
    // sc=12 (==instit) → 4e bande : 9 + (12-12)/1 = 9.
    assert_eq!(ScoringV11::force(12, &c), 9);
    // sc=13 → 9 + 1/1 = 10 (plafond).
    assert_eq!(ScoringV11::force(13, &c), 10);
    // sc=100 → clamp 10.
    assert_eq!(ScoringV11::force(100, &c), 10);
}

#[test]
fn force_btc_plafond_moyen_only() {
    // BTC : MOYEN=8, FORT=INSTIT=99 (Moyen-only) → la note plafonne ~5-6.
    let c = AssetCalibration::detect("BTCUSD", "M15");
    assert_eq!(ScoringV11::force(8, &c), 5, "sc=moyen → 5");
    // sc=15 (scoreMax) → 2e bande (15<99) : 5 + (15-8)/91 ≈ 5.077 → 5.
    assert_eq!(ScoringV11::force(15, &c), 5);
}

#[test]
fn live_score_asset_non_reconnu_zero() {
    let c = AssetCalibration::detect("EURUSD", "M15");
    let out = SmcOutput::default();
    let bar = BarInput::new(100.0, 110.0, 99.0, 105.0);
    assert_eq!(ScoringV11::live_score(true, &out, &bar, &c), 0);
    assert!(!c.asset_reconnu);
}

#[test]
fn live_score_bos_seul_plafonne_a_8() {
    // Un BOS haussier sans aucune confluence → plafond 8 (garde anti-bruit).
    let c = cal_xau();
    let mut out = SmcOutput::default();
    out.atr14 = 2.0;
    out.bos.bullish = true;
    // Corps = 3 (>= 1.5×ATR=3) → poids 6.
    let bar = BarInput::new(100.0, 106.0, 99.0, 103.0);
    let sc = ScoringV11::live_score(true, &out, &bar, &c);
    assert_eq!(sc, 8, "BOS seul (poids 6) → plafond 8, pas 6");
}

#[test]
fn live_score_bos_plus_fvg_depasse_plafond() {
    // BOS (6) + FVG (wFVG=5 pour XAU) → 11 (>8, garde ne s'applique pas).
    let c = cal_xau();
    let mut out = SmcOutput::default();
    out.atr14 = 2.0;
    out.bos.bullish = true;
    out.fvg.is_fvg_bull = true;
    let bar = BarInput::new(100.0, 106.0, 99.0, 103.0);
    let sc = ScoringV11::live_score(true, &out, &bar, &c);
    assert!(sc >= 11, "BOS+FVG doit dépasser 8, got {sc}");
}

#[test]
fn accum_scores_sticky_max() {
    // Un OB vierge : score sticky = max(précédent, live+fresh+prox).
    let c = cal_xau();
    let mut s = ScoringV11::new(&c, 15);
    let mut out = SmcOutput::default();
    out.atr14 = 2.0;
    out.bos.bullish = true; // live élève
    let bar = BarInput::new(100.0, 106.0, 99.0, 103.0);
    let ob = ObZone {
        top: 102.0,
        bot: 98.0,
        state: ObState::Vierge,
        impulse_bar: 5,
        ob_bar: 4,
        timestamp: 0,
        is_ib: false,
    };
    s.update(&out, &bar, &c, &[ob], &[], &[], None);
    let sc1 = s.ob_score(true, 5);
    assert!(
        sc1 >= 8,
        "premier update: score >= 8 (plafond BOS), got {sc1}"
    );

    // Bar suivante sans BOS → live faible, mais sticky max conserve sc1.
    let mut out2 = SmcOutput::default();
    out2.atr14 = 2.0;
    let bar2 = BarInput::new(103.0, 104.0, 102.0, 103.0);
    s.update(&out2, &bar2, &c, &[ob], &[], &[], None);
    let sc2 = s.ob_score(true, 5);
    assert_eq!(
        sc2, sc1,
        "sticky : le score ne redescend pas pour un OB vierge"
    );
}

#[test]
fn zn_qual_dol_via_asian_high_seul() {
    // Pine 3052 : le DoL accepte aussi _ahHighDrawn — une OB dont la
    // seule liquidité au-delà est l'Asian High DOIT qualifier.
    let cal = AssetCalibration::detect("XAUUSD", "M15");
    let sc = ScoringV11::new(&cal, 15);
    let out = SmcOutput {
        asian_hl: crate::v12::asian_hl::AsianHlEvent {
            high: Some(110.0),
            low: None,
        },
        ..Default::default()
    };
    let ob = ObZone {
        top: 105.0,
        bot: 100.0,
        state: ObState::Vierge,
        impulse_bar: 5,
        ob_bar: 4,
        timestamp: 0,
        is_ib: false,
    };
    let fvgs = vec![super::super::types::FvgZone {
        top: 103.0,
        bot: 101.0,
        state: super::super::types::FvgState::Fresh,
        bar: 5,
    }];
    assert!(
        sc.zn_qual_bull(&ob, &out, &fvgs),
        "Asian High 110 > top 105 ⇒ DoL OK (EQH/PDH/PWH tous absents)"
    );
    let out_sans = SmcOutput::default();
    assert!(
        !sc.zn_qual_bull(&ob, &out_sans, &fvgs),
        "aucune liquidité ⇒ DoL KO"
    );
}

fn zn_qual_neutralise_pour_dax_m15() {
    // DAX M15 → _znDaxHTF true → filtres neutralisés (toujours qualifiés).
    let c = AssetCalibration::detect("DAX", "M15");
    let s = ScoringV11::new(&c, 15);
    let ob = ObZone {
        top: 102.0,
        bot: 98.0,
        state: ObState::Vierge,
        impulse_bar: 1,
        ob_bar: 0,
        timestamp: 0,
        is_ib: false,
    };
    let out = SmcOutput::default();
    assert!(
        s.zn_qual_bull(&ob, &out, &[]),
        "DAX M15 : zone toujours qualifiée"
    );
}

/// Module F — proximité H/L de session : bull près d'un LOW drawn (Asie ou
/// Londres) ⇒ vrai ; bear ignore les lows ; hors rayon 0.35×ATR ⇒ faux.
#[test]
fn module_f_proximite_sessions_hl() {
    use crate::v12::asian_hl::SessHlLevels;
    use crate::v12::scoring_v11::sess_hl_near;
    let lvl = SessHlLevels {
        ah_high: Some(110.0),
        ah_low: Some(100.0),
        ld_high: Some(108.0),
        ld_low: None,
    };
    let atr = 10.0; // rayon = 0.35 × 10 = 3.5
    assert!(sess_hl_near(true, &lvl, 102.0, atr), "bull à 2.0 du low 100 ⇒ près");
    assert!(sess_hl_near(true, &lvl, 103.4, atr), "bull à 3.4 ≤ 3.5 ⇒ limite");
    assert!(!sess_hl_near(true, &lvl, 103.6, atr), "bull à 3.6 > 3.5 ⇒ hors");
    assert!(sess_hl_near(false, &lvl, 108.2, atr), "bear à 0.2 du high Londres");
    assert!(!sess_hl_near(false, &lvl, 103.0, atr), "bear loin des highs ⇒ faux");
    // ld_low absent : ne compte pas pour bull.
    assert!(!sess_hl_near(true, &lvl, 108.2, atr), "bull ignore les highs");
}
