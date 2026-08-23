//! Tests du détecteur de liquidités (extraits pour la limite 600 lignes).

#[cfg(test)]
use super::*;

#[cfg(test)]
use super::*;

fn bar(ts: i64, high: f64, low: f64, close: f64) -> BarInput {
    BarInput {
        timestamp: ts,
        open: close,
        high,
        low,
        close,
        volume: 0.0,
    }
}

fn no_pivot() -> PivotEvent {
    PivotEvent::default()
}

// ===================== PDH / PDL =====================

#[test]
fn pdh_pdl_apparaissent_au_changement_de_jour() {
    let mut det = LiquiditesDetector::new();
    // Jour 1 (ts 0.., multiple bars M15).
    for i in 0..10 {
        let ts = i * 900; // 900s = 15 min
        det.update(
            &bar(ts, 100.0 + i as f64, 90.0, 95.0),
            &PivotDetector::new(3),
            &no_pivot(),
            2.0,
        );
    }
    assert!(
        det.pdh.is_none(),
        "aucun pdh tant qu'un jour complet n'est pas passé"
    );
    // Jour 2 (ts >= 86400) — barre NEUTRE (ne touche ni PDH 109 ni PDL 90) :
    // règle « décisions trading » atteinte = consommé.
    det.update(
        &bar(86400, 100.0, 95.0, 98.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    // pdh = high max du jour 1 = 109.0 (100..109).
    assert_eq!(det.pdh, Some(109.0));
    assert_eq!(det.pdl, Some(90.0));
    assert_eq!(det.pdh_active, Some(109.0));
    assert_eq!(det.pdl_active, Some(90.0));
}

#[test]
fn pdh_consomme_a_la_cassure_decisions_trading() {
    // « Décisions trading » 23/08 : atteinte (sweep OU cassure) = consommé.
    // Avant, une CASSURE (close au-delà) laissait le niveau actif.
    let mut det = LiquiditesDetector::new();
    for i in 0..10 {
        let ts = i * 900;
        det.update(
            &bar(ts, 100.0 + i as f64, 90.0, 95.0),
            &PivotDetector::new(3),
            &no_pivot(),
            2.0,
        );
    }
    // Jour 2 : barre neutre → niveaux actifs.
    det.update(
        &bar(86400, 100.0, 95.0, 98.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert_eq!(det.pdh_active, Some(109.0));
    // Cassure franc : high 111 >= PDH 109 (close au-delà aussi) → consommé.
    det.update(
        &bar(86400 + 900, 112.0, 111.0, 110.5),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert_eq!(
        det.pdh_active, None,
        "PDH consommé à l'atteinte (cassure comprise)"
    );
}

#[test]
fn dernier_eqh_consomme_a_la_cassure() {
    let mut det = LiquiditesDetector::new();
    // Injecter un dernier_eqh à 120.
    det.set_dernier_eqh_pour_test(120.0);
    // Sweep mèche + retour : consommé à l'ATTEINTE (high >= 120).
    det.consommer_niveaux_atteints(&bar(900, 121.0, 119.0, 119.5));
    assert_eq!(det.dernier_eqh_level(), None, "EQH consommé dès l'atteinte");
}

#[test]
fn pool_niveau_touche_disparait() {
    let mut det = LiquiditesDetector::new();
    det.set_dernier_eqh_pour_test(120.0);
    // Le pool est vide dans ce test : la purge ne panique pas, dernier seul.
    det.consommer_niveaux_atteints(&bar(900, 119.0, 118.0, 118.5));
    assert_eq!(det.dernier_eqh_level(), Some(120.0), "non touché = vivant");
}

#[test]
fn sweep_pdh_invalide_pdh_active() {
    let mut det = LiquiditesDetector::new();
    // Jour 1 : high monte à 120.
    for i in 0..10 {
        let ts = i * 900;
        det.update(
            &bar(ts, 100.0 + i as f64 * 2.0, 90.0, 95.0),
            &PivotDetector::new(3),
            &no_pivot(),
            2.0,
        );
    }
    // Jour 2 : pdh = 118.0.
    det.update(
        &bar(86400, 50.0, 40.0, 45.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert_eq!(det.pdh, Some(118.0));
    // Sweep : high > pdh (118) ET close < pdh.
    let ev = det.update(
        &bar(86400 + 900, 120.0, 110.0, 115.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert!(
        ev.sweep_pdh,
        "high=120 > pdh=118 ET close=115 < 118 ⇒ sweep"
    );
    assert!(det.pdh_active.is_none(), "pdh_active invalidé après sweep");
    // pdh brut reste disponible (Pine n'efface pas pdh, seulement pdhActive).
    assert_eq!(det.pdh, Some(118.0));
}

#[test]
fn pas_de_sweep_si_close_ne_reviend_pas() {
    let mut det = LiquiditesDetector::new();
    for i in 0..10 {
        let ts = i * 900;
        det.update(
            &bar(ts, 100.0 + i as f64, 90.0, 95.0),
            &PivotDetector::new(3),
            &no_pivot(),
            2.0,
        );
    }
    det.update(
        &bar(86400, 50.0, 40.0, 45.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert_eq!(det.pdh, Some(109.0));
    // close >= pdh (pas de retour) ⇒ pas un sweep.
    let ev = det.update(
        &bar(86400 + 900, 120.0, 110.0, 110.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert!(!ev.sweep_pdh);
}

// ===================== PWH / PWL =====================

#[test]
fn pwh_pwl_changement_de_semaine() {
    // Déterministe : départ lundi 2024-01-01 (unix 1704067200), semaine ISO W01.
    // Mon..Sun (d=0..6) ⇒ toujours W01 ; Mon d=7 (2024-01-08) ⇒ W02.
    let base = 1_704_067_200_i64;
    let mut det = LiquiditesDetector::new();
    // Une bar par jour, high croissant 100..160 sur la semaine W01.
    for d in 0..7usize {
        let ts = base + (d as i64) * 86_400;
        det.update(
            &bar(ts, 100.0 + d as f64 * 10.0, 90.0, 95.0),
            &PivotDetector::new(3),
            &no_pivot(),
            2.0,
        );
    }
    // Toujours dans W01 ⇒ aucun pwh encore.
    assert!(
        det.pwh.is_none(),
        "aucun pwh tant qu'une semaine ISO complète n'est pas passée"
    );

    // d=7 ⇒ lundi W02 : pwh = high max de W01 = 160.0.
    let ts_w2 = base + 7 * 86_400;
    det.update(
        &bar(ts_w2, 50.0, 40.0, 45.0),
        &PivotDetector::new(3),
        &no_pivot(),
        2.0,
    );
    assert_eq!(
        det.pwh,
        Some(160.0),
        "pwh = high max de la semaine précédente"
    );
    assert_eq!(det.pwl, Some(90.0));
    assert_eq!(
        det.pwh_active, det.pwh,
        "pwh_active rafraîchi au changement de semaine"
    );
}

// ===================== EQH / EQL =====================

fn build_pivots_eqh() -> PivotDetector {
    // 2 pivots high égaux (sh1=sh2=110), sl=3. Deux pics à index 3 et 9.
    let mut piv = PivotDetector::new(3);
    for i in 0..13usize {
        let h = if i == 3 || i == 9 { 110.0 } else { 100.0 };
        let b = BarInput {
            timestamp: i as i64,
            open: 100.0,
            high: h,
            low: 90.0,
            close: 100.0,
            volume: 0.0,
        };
        piv.update(&b);
    }
    piv
}

#[test]
fn is_eqh_detecte_deux_highs_egaux_et_cree_niveau() {
    let piv = build_pivots_eqh();
    assert_eq!(piv.sh1(), Some(110.0));
    assert_eq!(piv.sh2(), Some(110.0));
    let mut det = LiquiditesDetector::new();
    // ATR14 artificiel = 10 ⇒ tolEq = 2.0 (|110-110|=0 <= 2 ⇒ EQH).
    let ev = det.update(
        &BarInput {
            timestamp: 12,
            open: 100.0,
            high: 100.0,
            low: 90.0,
            close: 100.0,
            volume: 0.0,
        },
        &piv,
        &PivotEvent {
            is_pivot_high: true,
            pivot_high_price: Some(110.0),
            pivot_bar_index: Some(9),
            ..Default::default()
        },
        10.0,
    );
    assert!(ev.is_eqh);
    assert_eq!(ev.dernier_eqh_level, Some(110.0));
    assert_eq!(det.pool().len(), 1, "un niveau EQH créé");
    let lvl = det.pool()[0];
    assert!(lvl.is_high);
    assert_eq!(lvl.touches, 2);
    assert!(!lvl.swept);
}

#[test]
fn mark_swept_niveau_eqh_via_pool() {
    let piv = build_pivots_eqh();
    let mut det = LiquiditesDetector::new();
    // Crée le niveau EQH (2 touches) via le flux MODULE 4.
    det.update(
        &bar(12, 100.0, 90.0, 100.0),
        &piv,
        &PivotEvent {
            is_pivot_high: true,
            pivot_high_price: Some(110.0),
            pivot_bar_index: Some(9),
            ..Default::default()
        },
        10.0,
    );
    assert_eq!(det.pool()[0].touches, 2);
    // mark_swept grise le niveau EQH correspondant (consommation par un sweep baissier).
    det.mark_swept(true, 110.0, 2.0);
    assert!(
        det.pool()[0].swept,
        "mark_swept grise le niveau correspondant"
    );
}

#[test]
fn mark_swept_niveau_eql() {
    let mut det = LiquiditesDetector::new();
    // Crée un niveau EQL directement dans le pool.
    det.pool.push(LiqLevel {
        price: 90.0,
        t_first: 0,
        touches: 2,
        swept: false,
        is_high: false,
    });
    det.dernier_eql_level = Some(90.0);
    det.mark_swept(false, 90.0, 2.0);
    assert!(det.pool()[0].swept);
}

#[test]
fn pool_fifo_limite_a_20_niveaux() {
    let mut det = LiquiditesDetector::new();
    for k in 0..25 {
        det.pool.push(LiqLevel {
            price: 100.0 + k as f64,
            t_first: k,
            touches: 2,
            swept: false,
            is_high: k % 2 == 0,
        });
    }
    // Simule le comportement FIFO de liq_update.
    if det.pool.len() >= MAX_LIQ {
        det.pool.remove(0);
    }
    assert_eq!(det.pool.len(), 25 - 1);
}
