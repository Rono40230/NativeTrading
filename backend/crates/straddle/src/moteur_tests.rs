//! Tests du moteur straddle — mécanique corrigée 26/08 : le TIMER ouvre les
//! 2 jambes à E à T-10 s (pas d'attente de franchissement), gestion parallèle,
//! R net = somme des jambes.

use super::*;
use chrono::TimeZone;

fn ctx_tick(ts: i64, prix: f64) -> (engine::agregateur::BougieEnFormation, Asset, Timeframe) {
    let b = engine::agregateur::BougieEnFormation {
        debut: ts,
        open: prix,
        high: prix,
        low: prix,
        close: prix,
        volume: 0.0,
        nb_events: 1,
        dernier_event: None,
    };
    (b, Asset::from("XAUUSD"), Timeframe::try_from("M1").unwrap_or(Timeframe::M1))
}

fn tick(m: &mut StraddleEngine, ts: i64, prix: f64) -> SortieMoteur {
    let (b, a, t) = ctx_tick(ts, prix);
    let ctx = ContexteTick { asset: &a, tf: t, bougie: &b };
    m.on_tick(&ctx)
}

fn close(m: &mut StraddleEngine, ts: i64, o: f64, h: f64, l: f64, c: f64) {
    let bougie = common::Candle {
        timestamp: chrono::Utc.timestamp_opt(ts, 0).single().unwrap_or_default(),
        open: o, high: h, low: l, close: c, volume: 0.0,
    };
    let a = Asset::from("XAUUSD");
    let t = Timeframe::try_from("M1").unwrap_or(Timeframe::M1);
    let ctx = ContexteCloture { asset: &a, tf: t, bougie: &bougie, index_barre: 0 };
    let _ = m.on_close(&ctx);
}

/// Moteur chauffé : 60 clôtures M1 de range 1.0 ⇒ ATR14 ≈ 1.0 ⇒ R = 0.5.
fn moteur_pret(annonce_ts: i64) -> StraddleEngine {
    let mut m = StraddleEngine::nouveau(Asset::from("XAUUSD"), Timeframe::try_from("M1").unwrap_or(Timeframe::M1))
        .avec_annonces(vec![Annonce { ts: annonce_ts, devise: "USD".into(), titre: "NFP".into() }]);
    for i in 0..60 {
        let ts = annonce_ts - 3600 + i * 60;
        close(&mut m, ts, 100.0, 100.5, 99.5, 100.0);
    }
    m
}

/// Le verdict net d'une Cloture finale (« verdict|R »).
fn verdict_final(s: &SortieMoteur) -> (String, f64) {
    let c = s.evenements.iter()
        .find(|e| matches!(e.evenement, TypeEvenementTrade::Cloture))
        .unwrap_or_else(|| panic!("clôture finale attendue"));
    let mut it = c.detail.split('|');
    (it.next().unwrap_or("").to_string(), it.next().and_then(|r| r.parse().ok()).unwrap_or(0.0))
}

#[test]
fn le_timer_ouvre_les_deux_jambes_a_t_moins_10s() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    // T-30 : fenêtre de préparation.
    tick(&mut m, a_ts - 1800, 100.0);
    assert!(matches!(m.phase_courante(), Phase::Range { .. }));
    // T-11 s : encore en range.
    tick(&mut m, a_ts - 11, 100.2);
    assert!(matches!(m.phase_courante(), Phase::Range { .. }));
    // T-10 s : OUVERTURE PAR LE TIMER au prix courant, quel qu'il soit.
    let s = tick(&mut m, a_ts - 10, 100.2);
    match m.phase_courante() {
        Phase::Position { entree, r, jambes, .. } => {
            assert!((entree - 100.2).abs() < 1e-9, "E = prix courant à T-10 s");
            assert!((r - 0.5).abs() < 0.05, "R = sl_atr × ATR ≈ 0.5");
            assert!(jambes.iter().all(|j| j.ouverte()), "les 2 jambes ouvertes");
            assert!((jambes[0].sl - (100.2 - r)).abs() < 1e-9);
            assert!((jambes[1].sl - (100.2 + r)).abs() < 1e-9);
        }
        other => panic!("phase inattendue : {:?}", other),
    }
    assert_eq!(s.signaux.len(), 1, "un signal Both à l'ouverture");
    assert!(matches!(s.signaux[0].direction, Direction::Both));
    assert_eq!(s.signaux[0].prix_entree, 100.2);
}

#[test]
fn passe_sans_mouvement_timestop_referme_a_e() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0);
    // Le prix reste exactement à E pendant toute la passe…
    let s = tick(&mut m, a_ts + 30 * 60, 100.0);
    assert!(s.evenements.is_empty(), "aucune clôture avant le time-stop");
    // 60 min après l'OUVERTURE → TimeStop : 2 jambes à E, net 0R, journalisée.
    let s = tick(&mut m, a_ts - 10 + 61 * 60, 100.0);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "expire", "passe sans mouvement");
    assert!(r.abs() < 1e-9, "net 0R");
    assert!(matches!(m.phase_courante(), Phase::Idle), "annonce consommée");
}

#[test]
fn montee_gagnante_tp2_trailing_perdante_sl_net_positif() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // ouverture à E=100, R=0.5
    // Montée à E+1R (100.5) : LONG TP1 → BE, SHORT SL (-1R, silencieuse).
    let s = tick(&mut m, a_ts + 5, 100.5);
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Be)));
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Tp1)));
    assert!(!s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Cloture)),
        "pas de Cloture avant la fin des 2 jambes");
    // TP2 (101.0) → SL à TP1 + trailing ; nouveau haut 101.6 → SL ≈ 101.1.
    tick(&mut m, a_ts + 30, 101.0);
    tick(&mut m, a_ts + 60, 101.6);
    // Retrait sous le trailing (≈101.1) → clôture finale : TS (+2.2R) net -1R ⇒ tp2|+1.2R.
    let s = tick(&mut m, a_ts + 90, 101.0);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "tp2");
    assert!(r > 1.0, "R net = trailing gagnante − SL perdante = {:.2}", r);
}

#[test]
fn montee_puis_retour_la_gagnante_be_net_zero() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // E=100, R=0.5
    // Montée à TP1 (100.5) : LONG TP1+BE, SHORT SL.
    tick(&mut m, a_ts + 5, 100.5);
    // Retour à E (100.0) : LONG sort en BE (TP acquis +1R) → net = +1R −1R = 0.
    let s = tick(&mut m, a_ts + 60, 100.0);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "be", "TP1+BE (1R) − SL perdante (1R) = 0R net");
    assert!(r.abs() < 1e-9);
}

#[test]
fn baisse_directe_la_jambe_short_gagne() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // E=100, R=0.5
    // Chute directe à E−2R (99.0) : SHORT TP2 + trailing, LONG SL.
    tick(&mut m, a_ts + 10, 99.0);
    tick(&mut m, a_ts + 20, 98.4); // nouveau bas → trailing SHORT suit
    // Remontée au-dessus du trailing → clôture net > 0.
    let s = tick(&mut m, a_ts + 60, 99.1);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "tp2", "jambe short gagnante au-delà de TP2");
    assert!(r > 1.0);
}

#[test]
fn sl_avant_tout_tp_net_moins_1r() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // E=100, R=0.5
    // Petite montée puis chute directe sous le SL long (99.5) : LONG SL (-1R)…
    tick(&mut m, a_ts + 5, 100.2);
    let s = tick(&mut m, a_ts + 20, 99.4);
    // …puis la SHORT court ; TimeStop 60 min la referme au prix courant.
    let s = tick(&mut m, a_ts - 10 + 61 * 60, 99.6);
    let (_verdict, r) = verdict_final(&s);
    assert!(r < 0.0, "net négatif (SL long −1R + short ≈ +0.8R) : {:.2}", r);
}

#[test]
fn r_base_sur_l_atr_h1_injectee() {
    let a_ts = 1_800_000_000;
    // Chauffe M1 (ATR M1 ≈ 1) mais ATR H1 injectée = 6 → R = 0.5 × 6 = 3.
    let mut m = moteur_pret(a_ts).avec_atr_h1(Some(6.0));
    tick(&mut m, a_ts - 1800, 100.0);
    let s = tick(&mut m, a_ts - 10, 100.0);
    match m.phase_courante() {
        Phase::Position { r, jambes, .. } => {
            assert!((r - 3.0).abs() < 1e-9, "R = sl_atr × ATR H1 = 3.0 (got {r})");
            assert!((jambes[0].sl - 97.0).abs() < 1e-9, "SL long = E - 3");
            assert!((jambes[0].tp1 - 103.0).abs() < 1e-9, "TP1 = E + 3");
            assert!((jambes[1].sl - 103.0).abs() < 1e-9, "SL short = E + 3");
        }
        other => panic!("phase inattendue : {:?}", other),
    }
    assert_eq!(s.signaux.len(), 1);
}

#[test]
fn repli_atr_m1_sans_injection_h1() {
    let a_ts = 1_800_000_000;
    // Aucune injection H1 → repli sur l'ATR M1 (≈ 1) → R ≈ 0.5.
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0);
    match m.phase_courante() {
        Phase::Position { r, .. } => assert!((r - 0.5).abs() < 0.05, "repli M1"),
        other => panic!("phase inattendue : {:?}", other),
    }
}
