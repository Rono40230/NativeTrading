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
            assert!(jambes.iter().all(|j| j.close_reason.is_none()), "les 2 jambes ouvertes");
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
    let mut m = moteur_pret(a_ts)
        .avec_params(crate::types::ParamsStraddle { trailing_r: 0.5, ..Default::default() });
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // ouverture à E=100, R=0.5
    // Montée à E+1R (100.51) : LONG TP1 → tampon (99.75), SHORT SL (strict >).
    let s = tick(&mut m, a_ts + 5, 100.51);
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Tp1)));
    assert!(!s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Be)), "plus de BE à E");
    assert!(!s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Cloture)),
        "pas de Cloture avant la fin des 2 jambes");
    // TP2 (101.0) → SL à TP1 + trailing 0.5R ; haut 101.4 (sous TP3 101.5)
    // → stop suivi = 101.4 − 0.25 = 101.15.
    tick(&mut m, a_ts + 30, 101.0);
    tick(&mut m, a_ts + 60, 101.4);
    // Retrait sous le trailing → clôture finale : TS (+2.3R) net −1R ⇒ tp2|+1.3R.
    let s = tick(&mut m, a_ts + 90, 101.0);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "tp2");
    assert!(r > 1.0, "R net = trailing gagnante − SL perdante = {:.2}", r);
}

#[test]
fn montee_puis_retour_a_e_la_gagnante_survit_tampon() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // E=100, R=0.5
    // Montée au-dessus de TP1 (100.6) : LONG TP1 → SL au tampon 99.75 ;
    // SHORT SL (100.6 > 100.5, strict) → −1R.
    tick(&mut m, a_ts + 5, 100.6);
    // Retour à E (100.0) : le tampon (99.75) n'est PAS touché — la gagnante
    // SURVIT au rebond (décision 27/08 : c'est le whipsaw qui tuait tout).
    let s = tick(&mut m, a_ts + 60, 100.0);
    assert!(s.evenements.is_empty(), "aucune clôture au rebond à E");
    assert!(matches!(m.phase_courante(), Phase::Position { .. }), "passe toujours ouverte");
    // Retour au tampon 99.75 : sortie SL tampon → −0,5R. Net = −0,5 −1 = −1,5R
    // tant que l'autre jambe est déjà morte → clôture finale.
    let s = tick(&mut m, a_ts + 120, 99.7);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "sl");
    assert!((r - (-1.5)).abs() < 1e-6, "tampon −0,5R + perdante −1R, got {r}");
}

#[test]
fn baisse_directe_la_jambe_short_gagne() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts)
        .avec_params(crate::types::ParamsStraddle { trailing_r: 0.5, ..Default::default() });
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // E=100, R=0.5
    // Chute directe à E−2R (99.0) : SHORT TP2 + trailing, LONG SL.
    tick(&mut m, a_ts + 10, 99.0);
    tick(&mut m, a_ts + 20, 98.6); // creux > TP3 (98.5) → trailing suit
    // Remontée au-dessus du trailing (98.85) → clôture net > 0.
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
    // Chute directe sous le SL long (99.5) : LONG SL (−1R) ; SHORT TP1
    // simultané (même niveau, comptabilité TP acquis du moteur commun).
    // Time-stop 60 min : la SHORT expire au verdict TP1 (+1R acquis).
    // Net = −1 + 1 = 0 → « be » — sous le moteur unifié, une passe où la
    // survivante touche TP1 ne peut plus être nette négative.
    tick(&mut m, a_ts + 5, 100.2);
    tick(&mut m, a_ts + 20, 99.4);
    let s = tick(&mut m, a_ts - 10 + 61 * 60, 99.6);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "be", "TP1 acquis compense le SL de la perdante");
    assert!(r.abs() < 1e-9, "net 0R (got {r})");
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

/// Tick avec instant d'ARRIVÉE distinct du début de bougie — la situation
/// de production : la bougie en formation est alignée à la minute, le prix
/// arrive à la seconde près.
fn tick_arrivee(m: &mut StraddleEngine, debut: i64, recu_le: i64, prix: f64) -> SortieMoteur {
    let mut b = engine::agregateur::BougieEnFormation {
        debut, open: prix, high: prix, low: prix, close: prix,
        volume: 0.0, nb_events: 1, dernier_event: None,
    };
    b.dernier_event = chrono::Utc.timestamp_opt(recu_le, 0).single();
    let a = Asset::from("XAUUSD");
    let t = Timeframe::try_from("M1").unwrap_or(Timeframe::M1);
    let ctx = ContexteTick { asset: &a, tf: t, bougie: &b };
    m.on_tick(&ctx)
}

/// Production : le début de bougie est aligné minute — sans l'instant
/// d'arrivée, le timer T-3 s ne peut JAMAIS ouvrir avant l'événement
/// (la bougie précédente débute à annonce−60 s, la suivante à l'annonce).
/// Le moteur doit s'armer sur l'ARRIVÉE (annonce−3 s), pas sur le début.
#[test]
fn le_timer_s_arme_sur_linstant_darrivee_pas_le_debut_de_bougie() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts)
        .avec_params(crate::types::ParamsStraddle { placement_avant_sec: 3, ..Default::default() });
    tick(&mut m, a_ts - 1800, 100.0);
    // Prix arrivé à annonce−4 s (bougie de la minute annonce−60) : trop tôt.
    let s1 = tick_arrivee(&mut m, a_ts - 60, a_ts - 4, 100.1);
    assert!(s1.signaux.is_empty());
    assert!(matches!(m.phase_courante(), Phase::Range { .. }), "T-4 s : encore en range");
    // Prix arrivé à annonce−3 s, même bougie en formation : OUVERTURE.
    let s2 = tick_arrivee(&mut m, a_ts - 60, a_ts - 3, 100.2);
    assert_eq!(s2.signaux.len(), 1, "T-3 s : jambes posées AVANT l'événement");
    match m.phase_courante() {
        Phase::Position { entree, ouverture_ts, .. } => {
            assert!((entree - 100.2).abs() < 1e-9, "E = prix courant à T-3 s");
            assert_eq!(*ouverture_ts, a_ts - 3, "ouverture horodatée à l'instant d'arrivée");
        }
        other => panic!("phase inattendue : {:?}", other),
    }
}

/// Priorité au créneau suivant (owner 29/09) : une passe dont le time-stop
/// canonique dépasserait l'événement SUIVANT est expirée à son T-N s — le
/// moteur est libre, jamais de file d'attente (14:30 ne mange plus 15:30).
#[test]
fn priorite_au_creneau_suivant_libere_le_moteur() {
    let a_ts = 1_800_000_000;
    let b_ts = a_ts + 1800; // créneau suivant 30 min plus tard
    let mut m = StraddleEngine::nouveau(Asset::from("XAUUSD"), Timeframe::try_from("M1").unwrap_or(Timeframe::M1))
        .avec_annonces(vec![
            Annonce { ts: a_ts, devise: "USD".into(), titre: "A".into() },
            Annonce { ts: b_ts, devise: "USD".into(), titre: "B".into() },
        ]);
    for i in 0..60 {
        close(&mut m, a_ts - 3600 + i * 60, 100.0, 100.5, 99.5, 100.0);
    }
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 3, 100.0); // passe A ouverte à E = 100
    // La passe vit, prix immobile — bien avant l'échéance raccourcie.
    let _ = tick(&mut m, a_ts + 60, 100.0);
    assert!(matches!(m.phase_courante(), Phase::Position { .. }), "passe A en cours");
    // Premier tick APRÈS le T-3 s du créneau B : A expire (time-stop
    // raccourci à B-3 s), verdict rendu au prix courant.
    let s = tick(&mut m, b_ts - 2, 100.0);
    let (verdict, r) = verdict_final(&s);
    assert_eq!(verdict, "expire", "passe immobile refermée pour libérer le moteur");
    assert!(r.abs() < 1e-9);
    // Deux ticks plus tard, la passe B est ouverte (un tick pour la
    // fenêtre Range, un pour l'ouverture — machine à états, une phase
    // par tick) : SANS la priorité, B aurait attendu la fin du time-stop
    // canonique de A (60 min).
    let _ = tick(&mut m, b_ts - 1, 100.0);
    let s2 = tick(&mut m, b_ts, 100.0);
    assert_eq!(s2.signaux.len(), 1, "passe B ouverte sans attendre");
    match m.phase_courante() {
        Phase::Position { annonce_ts, .. } => assert_eq!(*annonce_ts, b_ts),
        other => panic!("phase inattendue : {:?}", other),
    }
}
