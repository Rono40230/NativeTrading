//! Tests du détecteur MTF (extraits de mtf.rs — limite 600 lignes).

#[cfg(test)]
use super::*;

fn ltf_bar(ts: i64, open: f64, high: f64, low: f64, close: f64) -> BarInput {
    BarInput {
        timestamp: ts,
        open,
        high,
        low,
        close,
        volume: 1.0,
    }
}

/// Construit une série H1 haussière : pivot high net confirmé, puis plus tard un BOS
/// au-dessus → doit produire un OB bull H1 (la dernière bougie baissière avant le BOS).
#[test]
fn h1_aggregation_et_ob_bull_apres_bos() {
    let mut det = MtfDetector::new();
    // sw_len=3 ⇒ pivot confirmé à i = pivot_idx + 3. Le BOS doit survenir à une bar
    // POSTÉRIEURE au pivot ET dont le high ne casse pas la confirmation du pivot.
    //   i0..i2 : doji 100 (high 102)
    //   i3     : pic high=110 (pivot candidat)
    //   i4..i6 : doji 100 (high 102 < 110 ⇒ pivot i3 confirmé à i6 ⇒ sh1=110)
    //   i7     : bear candle (open 105, close 100, low 99) ⇒ OB candidat (l_b_t=105)
    //   i8     : doji 100 (close==open ⇒ conserve le candidat 105)
    //   i9     : BOS (close 111 > 110, prev_close 100 <= 110) ⇒ OB bull = [105,99]
    let bars = [
        (0 * 3600, 100.0, 102.0, 98.0, 100.0),
        (1 * 3600, 100.0, 102.0, 98.0, 100.0),
        (2 * 3600, 100.0, 102.0, 98.0, 100.0),
        (3 * 3600, 100.0, 110.0, 99.0, 100.0), // pic
        (4 * 3600, 100.0, 102.0, 98.0, 100.0),
        (5 * 3600, 100.0, 102.0, 98.0, 100.0),
        (6 * 3600, 100.0, 102.0, 98.0, 100.0), // pivot i3 confirmé ⇒ sh1=110
        (7 * 3600, 105.0, 106.0, 99.0, 100.0), // bear candle ⇒ OB candidat [105,99]
        (8 * 3600, 100.0, 102.0, 98.0, 100.0), // doji ⇒ conserve candidat
        (9 * 3600, 100.0, 112.0, 99.0, 111.0), // BOS : close 111 > 110
    ];
    for (ts, o, h, l, c) in bars {
        det.update(&ltf_bar(ts, o, h, l, c));
    }
    let st = &det.last_event.h1;
    assert!(!st.bull_obs.is_empty(), "BOS up ⇒ au moins un OB bull H1");
    let ob = &st.bull_obs[0];
    assert!(
        (ob.top - 105.0).abs() < 1e-9,
        "OB top = open du bear candle (105)"
    );
    assert!(
        (ob.bot - 99.0).abs() < 1e-9,
        "OB bot = low du bear candle (99)"
    );
    assert_eq!(st.trend, 1, "BOS up ⇒ trend=1");
}

#[test]
fn aggregation_h1_regroupe_4_bars_m15() {
    let mut det = MtfDetector::new();
    // 4 bars M15 dans la même heure (ts 0,900,1800,2700).
    det.update(&ltf_bar(0, 100.0, 102.0, 98.0, 101.0));
    det.update(&ltf_bar(900, 101.0, 105.0, 100.0, 103.0));
    det.update(&ltf_bar(1800, 103.0, 108.0, 102.0, 107.0));
    det.update(&ltf_bar(2700, 107.0, 110.0, 106.0, 109.0));
    // La bar H1 courante (pas encore clôturée) agrège les 4.
    let mut series = Vec::new();
    det.h1.series(&mut series);
    assert_eq!(series.len(), 1, "4 bars M15 ⇒ 1 bar H1 en cours");
    let h1bar = series[0];
    assert!((h1bar.open - 100.0).abs() < 1e-9, "open = 1ʳᵉ bar");
    assert!((h1bar.high - 110.0).abs() < 1e-9, "high = max");
    assert!((h1bar.low - 98.0).abs() < 1e-9, "low = min");
    assert!((h1bar.close - 109.0).abs() < 1e-9, "close = dernière");
}

#[test]
fn confluence_fausse_sans_ob() {
    let mut det = MtfDetector::new();
    // Aucun pivot/BOS ⇒ aucun OB ⇒ pas de confluence.
    for i in 0..10 {
        det.update(&ltf_bar(i * 3600, 100.0, 101.0, 99.0, 100.0));
    }
    let ev = det.last_event();
    assert!(!ev.confluence_h1);
}

#[test]
fn fifo_htf_cappe_a_max() {
    // S'assure qu'au-delà de MAX_HTF_BARS le tampon ne croît pas indéfiniment.
    let mut agg = HtfAggregator::new(Period::Seconds(3600));
    for i in 0..(MAX_HTF_BARS + 50) as i64 {
        agg.add(&ltf_bar(i * 3600, 100.0, 101.0, 99.0, 100.0));
    }
    // Toutes les périodes sont distinctes ⇒ chaque bar clôt la précédente (sauf la dernière en cours).
    assert!(agg.closed.len() <= MAX_HTF_BARS);
}

#[test]
fn pas_de_panic_sur_serie_courte() {
    let st = replay_htf(&[], HTF_SWING);
    assert_eq!(st.trend, 0);
    assert!(st.bull_obs.is_empty() && st.bear_obs.is_empty());
}

#[test]
fn primer_w1_produit_des_ob_invisibles_sans_amorcage() {
    // Même motif que h1_aggregation_et_ob_bull_apres_bos (pic → bear → BOS),
    // en semaines : pivot confirmé à i6, OB candidat à i7, BOS à i9.
    let motif: [(f64, f64, f64, f64); 10] = [
        (100.0, 102.0, 98.0, 100.0),
        (100.0, 102.0, 98.0, 100.0),
        (100.0, 102.0, 98.0, 100.0),
        (100.0, 110.0, 99.0, 100.0), // pic
        (100.0, 102.0, 98.0, 100.0),
        (100.0, 102.0, 98.0, 100.0),
        (100.0, 102.0, 98.0, 100.0), // pivot confirmé
        (105.0, 106.0, 99.0, 100.0), // bear candle ⇒ OB [105,99]
        (100.0, 102.0, 98.0, 100.0),
        (100.0, 112.0, 99.0, 111.0), // BOS
    ];
    let mut w1_hist: Vec<BarInput> = Vec::new();
    for (i, (o, h, l, c)) in motif.iter().enumerate() {
        w1_hist.push(ltf_bar(
            1_700_000_000 + (i as i64) * 604_800,
            *o,
            *h,
            *l,
            *c,
        ));
    }
    let t0 = w1_hist.last().unwrap().timestamp + 604_800; // 1re bar LTF après l'historique
                                                          // Bar LTF DANS la zone OB [99,105] : la confluence W1 ne peut exister qu'amorcée.
    let bar_ltf = ltf_bar(t0 + 60, 101.0, 102.0, 100.0, 101.0);

    // Sans amorçage : 1 bar LTF → 1 bar W1 en cours → aucun OB W1.
    let mut det_nu = MtfDetector::new();
    let ev_nu = det_nu.update(&bar_ltf);
    assert!(ev_nu.w1.bull_obs.is_empty(), "sans amorçage : aucun OB W1");

    // Avec amorçage : l'OB W1 [99,105] issu de l'historique existe et le
    // close LTF (101) est dedans ⇒ confluence W1 active (composante +5 du score).
    let mut det = MtfDetector::new();
    det.primer(&[], &[], &w1_hist, &[], t0);
    let ev = det.update(&bar_ltf);
    assert!(
        !ev.w1.bull_obs.is_empty(),
        "avec amorçage : OB W1 détecté depuis l'historique ({:?})",
        ev.w1.bull_obs
    );
    assert!(
        ev.confluence_w1,
        "close dans l'OB W1 ⇒ confluence W1 active"
    );
}

#[test]
fn primer_bar_chevauchante_complete_aux_valeurs_exactes() {
    // La bar HTF contenant t0 devient la bar "en cours" ; les bars LTF qui
    // suivent la complètent — l'état clôturé doit rester exact (high/low monotones).
    let mut det = MtfDetector::new();
    // H1 contenant t0 : 10:00-11:00, t0 = 10:15.
    let h1_chevauchante = ltf_bar(1_700_000_000 + 10 * 3600, 100.0, 120.0, 95.0, 110.0);
    let t0 = h1_chevauchante.timestamp + 900;
    det.primer(&[h1_chevauchante], &[], &[], &[], t0);
    // Bars LTF 10:15 → 11:00 (incluses) puis 11:00 (nouvelle période).
    det.update(&ltf_bar(t0, 105.0, 112.0, 101.0, 108.0));
    det.update(&ltf_bar(t0 + 3600, 110.0, 115.0, 109.0, 114.0));
    // La bar H1 clôturée doit contenir le high 120 de l'amorce (monotone).
    let mut serie = Vec::new();
    det.h1.series(&mut serie);
    let cloturee = serie
        .iter()
        .find(|b| b.timestamp == h1_chevauchante.timestamp);
    assert!(cloturee.is_some(), "bar H1 chevauchante clôturée");
    assert!(
        (cloturee.unwrap().high - 120.0).abs() < 1e-9,
        "high conservé"
    );
}

#[test]
fn agreger_mensuel_regrouppe_par_mois_calendaire() {
    // 3 jours janvier + 2 jours février → 2 bars MN.
    use chrono::TimeZone;
    let j = |y: i32, m: u32, d: u32, close: f64| {
        let ts = chrono::Utc
            .with_ymd_and_hms(y, m, d, 0, 0, 0)
            .unwrap()
            .timestamp();
        ltf_bar(ts, close - 2.0, close + 2.0, close - 3.0, close)
    };
    let d1 = vec![
        j(2026, 1, 10, 100.0),
        j(2026, 1, 20, 104.0),
        j(2026, 1, 31, 98.0),
        j(2026, 2, 5, 110.0),
        j(2026, 2, 20, 112.0),
    ];
    let mn = agreger_mensuel(&d1);
    assert_eq!(mn.len(), 2);
    assert!((mn[0].open - 98.0).abs() < 1e-9, "open = 1er jour (100-2)");
    assert!(
        (mn[0].close - 98.0).abs() < 1e-9,
        "close = dernier jour de janvier"
    );
    assert!((mn[0].high - 106.0).abs() < 1e-9, "high max des 3 jours");
    assert!((mn[1].close - 112.0).abs() < 1e-9);
}
