//! Tests du lifecycle COMMUN (crate gestion_trades) — comportement générique
//! avec `HookVide` (pas de structure) : remplissage, SL, BE après TP1 avec
//! et sans tampon, TP2→TP3, trailing stop, expiration. La fidélité Pine de la
//! SMC (BE forcé, scoreDeg, un-signal) reste couverte par les tests du shim
//! `smc::v12::lifecycle` — ces tests-ci verrouillent le socle partagé avec le
//! straddle.

use super::*;
use crate::trade::{Side, TradeSource, TradeState, Verdict};

fn bar(ts: i64, o: f64, h: f64, l: f64, c: f64) -> BarInput {
    BarInput { timestamp: ts, open: o, high: h, low: l, close: c, volume: 0.0 }
}

/// BUY E=100, SL=97 (risk0=3), TP1=101.8 (0.6R), TP2=106 (2R), TP3=109 (3R).
fn buy() -> Trade {
    Trade::new_buy(
        1,
        TradeSource::Ob,
        100.0,
        97.0,
        101.8,
        106.0,
        109.0,
        10,
        3.0,
        &bar(0, 0.0, 0.0, 0.0, 0.0),
        0,
        None,
    )
}

fn pas(lc: &TradeLifecycle, t: &mut Trade, b: &BarInput, i: usize) {
    lc.update(std::slice::from_mut(t), b, i, &mut HookVide);
}

#[test]
fn remplissage_puis_sl_ferme_a_moins_1r() {
    let lc = TradeLifecycle::new(14400, 3600);
    let mut t = buy();
    // Bar 1 (t=60) : retest de l'entrée → remplissage.
    pas(&lc, &mut t, &bar(60, 100.5, 100.8, 100.0, 100.5), 1);
    assert!(t.filled, "retest de l'entrée remplit");
    assert_eq!(t.state, TradeState::Open);
    // Bar 2 : chute sous le SL (97) sans avoir touché TP1 → SL, −1R.
    pas(&lc, &mut t, &bar(120, 100.0, 100.2, 96.5, 96.8), 2);
    assert_eq!(t.state, TradeState::Closed);
    assert_eq!(t.verdict(), Verdict::Sl);
    assert!((t.realized_r() - (-1.0)).abs() < 1e-9);
}

#[test]
fn tp1_puis_retour_a_l_entree_been_be() {
    let lc = TradeLifecycle::new(14400, 3600);
    let mut t = buy();
    pas(&lc, &mut t, &bar(60, 100.5, 100.8, 100.0, 100.5), 1); // fill
    pas(&lc, &mut t, &bar(120, 101.0, 102.0, 100.8, 101.5), 2); // TP1 touché → SL→entry
    assert!(t.tp1_hit);
    assert!((t.sl - 100.0).abs() < 1e-9, "SL remonté à l'entrée (BE sans tampon)");
    pas(&lc, &mut t, &bar(180, 100.5, 100.6, 99.9, 100.1), 3); // retour à l'entrée
    assert_eq!(t.state, TradeState::Closed);
    assert_eq!(t.verdict(), Verdict::Tp1);
    // Comptabilité 24/08 « TP acquis » : TP1+BE = +0.6R (distance du TP1),
    // pas 0R — la promesse touchée est acquise même si le solde sort à E.
    assert!((t.realized_r() - 0.6).abs() < 1e-9, "TP1 acquis = +0.6R");
}

#[test]
fn tampon_be_offset_0_5r_protege_du_whipsaw() {
    // Décision 27/08 (straddle) : après TP1 le stop passe à E−0,5R — un
    // retour à l'entrée ne clôt PAS le trade.
    let lc = {
        let mut l = TradeLifecycle::new(14400, 3600);
        l.definir_be_offset_r(0.5);
        l
    };
    let mut t = buy();
    pas(&lc, &mut t, &bar(60, 100.5, 100.8, 100.0, 100.5), 1); // fill
    pas(&lc, &mut t, &bar(120, 101.0, 102.0, 100.8, 101.5), 2); // TP1 → SL à 98.5
    assert!((t.sl - 98.5).abs() < 1e-9, "tampon = E − 0.5×risk0 = 98.5");
    pas(&lc, &mut t, &bar(180, 100.5, 100.7, 100.0, 100.2), 3); // retour à E : survit
    assert_eq!(t.state, TradeState::Open, "le tampon survit au retour à l'entrée");
    // Bar 4 : percute le tampon → clôture. Verdict Tp1 (TP prix touché) ;
    // comptabilité « TP acquis » : realized_r = +0.6R, le solde sorti au
    // tampon vaut −0.5R (close_r) — le R pondéré répartira les deux.
    pas(&lc, &mut t, &bar(240, 99.5, 99.8, 98.4, 98.6), 4);
    assert_eq!(t.state, TradeState::Closed);
    assert_eq!(t.verdict(), Verdict::Tp1);
    assert!((t.realized_r() - 0.6).abs() < 1e-9, "TP1 acquis = +0.6R");
    assert!((t.close_r.unwrap_or(0.0) - (-0.5)).abs() < 1e-6, "solde au tampon = −0.5R");
}

#[test]
fn tp2_puis_tp3_verdict_tp3() {
    let lc = TradeLifecycle::new(14400, 3600);
    let mut t = buy();
    pas(&lc, &mut t, &bar(60, 100.5, 100.8, 100.0, 100.5), 1); // fill
    pas(&lc, &mut t, &bar(120, 104.0, 106.5, 103.5, 105.5), 2); // TP1+TP2 touchés
    assert!(t.tp2_ts > 0, "TP2 armé");
    pas(&lc, &mut t, &bar(180, 107.0, 109.5, 106.8, 108.5), 3); // TP3
    assert_eq!(t.state, TradeState::Closed);
    assert_eq!(t.verdict(), Verdict::Tp3);
    assert!((t.realized_r() - 3.0).abs() < 1e-6, "TP3 = +3R");
}

#[test]
fn trailing_apres_tp2_verrouille_l_extreme() {
    // Trailing k=1R : après TP2, le stop suit l'extrême à 1×risk0 — un
    // retrait le touche → verdict TS au R réel du stop.
    let lc = {
        let mut l = TradeLifecycle::new(14400, 3600);
        l.definir_trailing_tp2(Some(1.0));
        l
    };
    let mut t = buy();
    pas(&lc, &mut t, &bar(60, 100.5, 100.8, 100.0, 100.5), 1); // fill
    pas(&lc, &mut t, &bar(120, 104.0, 106.5, 103.5, 105.5), 2); // TP1+TP2
    pas(&lc, &mut t, &bar(180, 107.0, 108.0, 106.5, 107.5), 3); // extrême 108 → stop 105
    pas(&lc, &mut t, &bar(240, 106.0, 106.4, 104.8, 105.2), 4); // retrait sous le stop
    assert_eq!(t.state, TradeState::Closed, "trailing touché");
    assert_eq!(t.verdict(), Verdict::Ts);
    assert!((t.realized_r() - ((105.0 - 100.0) / 3.0)).abs() < 1e-6, "R = distance réelle du stop");
}

#[test]
fn expiration_cloture_au_prix_courant() {
    let lc = TradeLifecycle::new(600, 3600); // trade max 10 min
    let mut t = buy();
    pas(&lc, &mut t, &bar(60, 100.5, 100.8, 100.0, 100.5), 1); // fill à t=60
    // Bar à t=700 : âge 640 s > 600 s sans TP → expire au prix courant.
    pas(&lc, &mut t, &bar(700, 100.4, 100.6, 100.2, 100.3), 2);
    assert_eq!(t.state, TradeState::Closed);
    assert_eq!(t.verdict(), Verdict::Expire);
    // Comptabilité 24/08 : expiration = rien d'acquis → 0R, même si le prix
    // courant est au-dessus de l'entrée (aucun TP promis n'a été touché).
    assert!(t.realized_r().abs() < 1e-9, "expire = 0R acquis");
}

#[test]
fn jambe_short_symetrique() {
    let lc = TradeLifecycle::new(14400, 3600);
    let mut t = Trade::new_sell(
        2,
        TradeSource::Ob,
        100.0,
        103.0,
        98.2,
        94.0,
        91.0,
        10,
        3.0,
        &bar(0, 0.0, 0.0, 0.0, 0.0),
        0,
        None,
    );
    pas(&lc, &mut t, &bar(60, 99.5, 100.0, 99.2, 99.6), 1); // fill au retest
    assert!(t.filled);
    assert!(matches!(t.side, Side::Sell));
    pas(&lc, &mut t, &bar(120, 98.0, 98.4, 90.5, 91.2), 2); // TP1+TP2+TP3 traversés
    assert_eq!(t.state, TradeState::Closed);
    assert_eq!(t.verdict(), Verdict::Tp3);
    assert!((t.realized_r() - 3.0).abs() < 1e-6);
}
