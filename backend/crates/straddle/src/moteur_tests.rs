//! Tests du moteur straddle (extraits — limite 600 lignes).

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
    (b, Asset::from("XAUUSD"), Timeframe::try_from("M5").unwrap_or(Timeframe::M5))
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
    let t = Timeframe::try_from("M5").unwrap_or(Timeframe::M5);
    let ctx = ContexteCloture { asset: &a, tf: t, bougie: &bougie, index_barre: 0 };
    let _ = m.on_close(&ctx);
}

/// T-30 → range [100, 101] ; ATR calé à 1.0 ⇒ stops 101.25 / 99.75.
fn moteur_pret(annonce_ts: i64) -> StraddleEngine {
    let mut m = StraddleEngine::nouveau(Asset::from("XAUUSD"), Timeframe::try_from("M5").unwrap_or(Timeframe::M5))
        .avec_annonces(vec![Annonce { ts: annonce_ts, devise: "USD".into(), titre: "NFP".into() }]);
    // 60 clôtures M5 pour un ATR14 stable ≈ 1.0.
    for i in 0..60 {
        let ts = annonce_ts - 3600 + i * 300;
        close(&mut m, ts, 100.0, 100.5, 99.5, 100.0); // range H-L = 1.0
    }
    m
}

#[test]
fn range_puis_ordres_poses_a_t_moins_5() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    // T-30 : entrée en range.
    let s = tick(&mut m, a_ts - 1800, 100.0);
    assert!(s.signaux.is_empty());
    // Pendant le range : pas d'ordre.
    tick(&mut m, a_ts - 1500, 101.0);
    tick(&mut m, a_ts - 1200, 100.2);
    // T-5 : ordres posés (pas encore de signal).
    let s = tick(&mut m, a_ts - 300, 100.4);
    assert!(s.signaux.is_empty(), "pose d'ordre ≠ signal");
    // Un tick au-dessus du buy-stop (101 + 0.25×ATR≈0.25) → fill.
    let s = tick(&mut m, a_ts - 240, 101.3);
    assert_eq!(s.signaux.len(), 1, "fill LONG au buy-stop");
    assert!(matches!(s.signaux[0].direction, Direction::Long));
    assert_eq!(s.evenements.len(), 1, "événement OCO");
}

#[test]
fn fill_short_oco_et_sl() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 1500, 99.8);
    tick(&mut m, a_ts - 300, 99.9);
    // Cassure sous le sell-stop (99.8 - 0.25×ATR ≈ 99.55) → fill SHORT.
    let s = tick(&mut m, a_ts - 200, 99.5);
    assert_eq!(s.signaux.len(), 1);
    assert!(matches!(s.signaux[0].direction, Direction::Short));
    // SL = entrée + 0.5×ATR = 100.2 → un tick au-dessus clôture SL.
    let s = tick(&mut m, a_ts - 100, 100.3);
    assert!(
        s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Cloture) && e.detail == "SL"),
        "clôture SL"
    );
}

#[test]
fn tp1_arme_le_be() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 300, 100.1);
    let s = tick(&mut m, a_ts - 200, 101.3); // fill LONG @ ~101.25
    let entree = s.signaux[0].prix_entree;
    // TP1 = entree + 1.5×ATR ≈ 102.75.
    let s = tick(&mut m, a_ts - 100, 102.8);
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Tp1)));
    assert!(s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Be)));
    // Retour à l'entrée ⇒ clôture BE (SL = entrée).
    let s = tick(&mut m, a_ts, entree - 0.01);
    assert!(
        s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Cloture) && e.detail == "BE"),
        "clôture BE après TP1"
    );
}

#[test]
fn time_stop_60_min() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 300, 100.1);
    tick(&mut m, a_ts - 200, 101.3); // fill
    // +61 min sans rien toucher → TimeStop.
    let s = tick(&mut m, a_ts + 3600 + 60, 101.0);
    assert!(
        s.evenements.iter().any(|e| matches!(e.evenement, TypeEvenementTrade::Cloture) && e.detail == "TimeStop"),
        "time-stop 60 min"
    );
}

#[test]
fn expiration_sans_fill() {
    let a_ts = 1_800_000_000;
    let mut m = moteur_pret(a_ts);
    tick(&mut m, a_ts - 1800, 100.0);
    tick(&mut m, a_ts - 300, 100.1);
    // T+30 min sans fill → ordres expirés, retour Idle.
    let s = tick(&mut m, a_ts + 1800 + 60, 100.2);
    assert!(s.signaux.is_empty());
    // La même annonce ne redéclenche pas.
    let s = tick(&mut m, a_ts + 2000, 103.0);
    assert!(s.signaux.is_empty(), "annonce consommée");
}
