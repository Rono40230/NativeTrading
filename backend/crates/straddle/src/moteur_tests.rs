//! Tests du moteur straddle — mécanique étape 4 (2 jambes à E commun).

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

#[test]
fn ordres_poses_a_t_moins_10s_au_prix_courant() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    // T-30 : fenêtre de préparation.
    tick(&mut m, a_ts - 1800, 100.0);
    assert!(matches!(m.phase_courante(), Phase::Range { .. }));
    // T-11 s : encore en range.
    tick(&mut m, a_ts - 11, 100.2);
    assert!(matches!(m.phase_courante(), Phase::Range { .. }));
    // T-10 s : les 2 jambes sont posées au prix courant E (ATR≈1, R=0.5).
    tick(&mut m, a_ts - 10, 100.0);
    match m.phase_courante() {
        Phase::Ordres { entree, r, .. } => {
            assert!((entree - 100.0).abs() < 1e-9, "E = prix courant à T-10 s");
            assert!((r - 0.5).abs() < 0.05, "R = sl_atr × ATR ≈ 0.5");
        }
        other => panic!("phase inattendue : {:?}", other),
    }
}

#[test]
fn premier_franchissement_remplit_la_jambe_oco() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0); // pose
    // Premier mouvement vers le haut → jambe LONG à E=100, OCO sell annulée.
    let s = tick(&mut m, a_ts - 2, 100.3);
    assert_eq!(s.signaux.len(), 1, "un seul signal : la jambe remplie");
    assert_eq!(s.signaux[0].prix_entree, 100.0, "entrée au MÊME prix E");
    assert!((s.signaux[0].stop_loss - 99.5).abs() < 0.05, "SL buy = E - 1R");
    assert!((s.signaux[0].take_profits[0] - 100.5).abs() < 0.05, "TP1 = 1R");
    assert!((s.signaux[0].take_profits[1] - 101.0).abs() < 0.05, "TP2 = 2R");
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Fill)));
    // La jambe courte se remplirait à l'inverse.
    let mut m2 = moteur_pret(a_ts);
    tick(&mut m2, a_ts - 1800, 100.0);
    tick(&mut m2, a_ts - 10, 100.0);
    let s2 = tick(&mut m2, a_ts - 2, 99.7);
    assert_eq!(s2.signaux.len(), 1);
    assert!((s2.signaux[0].stop_loss - 100.5).abs() < 0.05, "SL sell = E + 1R");
}

#[test]
fn tp1_be_puis_tp2_declenche_le_trailing() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0);
    tick(&mut m, a_ts - 2, 100.3); // fill LONG @100, SL 99.5
    // TP1 (100.5) touché → BE à l'entrée.
    let s = tick(&mut m, a_ts, 100.6);
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Be)));
    // TP2 (101.0) touché → SL à TP1 + trailing actif : prix 101.6 → SL ≈ 101.1.
    let s = tick(&mut m, a_ts + 30, 101.6);
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Tp2)));
    // Nouveau plus haut 102.2 → trailing suit AU TICK : SL ≈ 101.7.
    tick(&mut m, a_ts + 60, 102.2);
    // Retrait sous le trailing (101.7) → sortie TS avec R > 1.
    let s = tick(&mut m, a_ts + 90, 101.6);
    let cloture = s.evenements.iter().find(|e| matches!(e.evenement, TypeEvenementTrade::Cloture));
    let Some(c) = cloture else { panic!("clôture TS attendue") };
    let (verdict, r) = c.detail.split_once('|').unwrap_or(("", "0"));
    assert_eq!(verdict, "TS");
    let r: f64 = r.parse().unwrap_or(0.0);
    assert!(r > 1.0, "sortie trailing au-delà de TP1 : R = {:.2}", r);
}

#[test]
fn sl_avant_tp1_sort_moins_1r() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0);
    tick(&mut m, a_ts - 2, 100.3); // fill LONG
    // Chute directe sous le SL (99.5) → SL |-1R.
    let s = tick(&mut m, a_ts + 5, 99.4);
    let c = s.evenements.iter().find(|e| matches!(e.evenement, TypeEvenementTrade::Cloture));
    let Some(c) = c else { panic!("clôture SL attendue") };
    assert_eq!(c.detail, "SL|-1.0000");
}

#[test]
fn expiration_sans_fill_annule_les_deux_jambes() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0);
    // Le prix reste exactement à E (aucun franchissement)…
    tick(&mut m, a_ts + 60, 100.0);
    assert!(matches!(m.phase_courante(), Phase::Ordres { .. }));
    // …jusqu'à l'expiration (30 min après l'annonce) → retour Idle.
    tick(&mut m, a_ts + 30 * 60 + 1, 100.0);
    assert!(matches!(m.phase_courante(), Phase::Idle));
}

#[test]
fn time_stop_ferme_au_prix_courant() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 10, 100.0);
    tick(&mut m, a_ts - 2, 100.3); // fill LONG @ t0
    tick(&mut m, a_ts + 30, 100.6); // TP1 → BE
    // 60 min après le fill → TimeStop au prix courant.
    let s = tick(&mut m, a_ts + 61 * 60, 100.4);
    let c = s.evenements.iter().find(|e| matches!(e.evenement, TypeEvenementTrade::Cloture));
    let Some(c) = c else { panic!("clôture TimeStop attendue") };
    assert!(c.detail.starts_with("TimeStop|"));
}
