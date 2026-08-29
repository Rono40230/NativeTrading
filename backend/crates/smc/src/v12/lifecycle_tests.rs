//! Tests du lifecycle des trades (scindés de `lifecycle.rs` — règle < 600 lignes).

use super::super::types::BarInput;
use super::*;

    use super::*;
    use crate::v12::calibration::AssetCalibration;
    use crate::v12::scoring_v11::ScoringV11;
    use crate::v12::trade::{Trade, TradeSource, Verdict};

    fn lc() -> TradeLifecycle {
        // tradeMax 240min=14400s, tp3Max 60min=3600s (XAU M15).
        TradeLifecycle::new(14400, 3600)
    }
    fn cal() -> AssetCalibration {
        AssetCalibration::detect("XAUUSD", "M15")
    }
    fn scoring() -> ScoringV11 {
        ScoringV11::new(&cal(), 15)
    }

    fn bar(ts: i64, o: f64, h: f64, l: f64, c: f64) -> BarInput {
        BarInput {
            timestamp: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0.0,
        }
    }

    fn buy_trade(entry: f64, sl: f64, tp1: f64, tp2: f64, tp3: f64) -> Trade {
        // Créé à bar 0 (ts=0), risk0 = entry-sl. ob_key=None pour isoler la logique
        // SL/BE/TP du scoreDeg (qui s'appuie sur le score OB lié).
        Trade::new_buy(
            1,
            TradeSource::Ob,
            entry,
            sl,
            tp1,
            tp2,
            tp3,
            10,
            entry - sl,
            &bar(0, 0.0, 0.0, 0.0, 0.0),
            0,
            None,
        )
    }

    #[test]
    fn be_force_un_signale_le_premier_ob_signale() {
        // Pine 3936-3941 : _obIdx = PREMIER OB signalé en scannant le carnet
        // (break au premier trouvé), PAS l'OB du trade. Avec deux OB signalés
        // [A(bar 10), B(bar 20)] et un trade lié à B : le BE-force un-signale A ;
        // B (l'OB du trade) reste verrouillé — pas de re-trade immédiat.
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 109.0);
        t.ob_key = Some(20);
        t.filled = true;
        let lc = lc();
        let mut out = SmcOutput::default();
        out.bos_raw.bearish = true; // BOS opposé ⇒ beForce
        let mut c = cal();
        let _ = &mut c;
        let mut sc = scoring();
        sc.mark_signaled(true, 10); // OB A (premier du carnet)
        sc.mark_signaled(true, 20); // OB B (OB du trade)
        let zones = vec![
            ObZone { top: 105.0, bot: 100.0, state: ObState::Vierge, impulse_bar: 10, ob_bar: 9, timestamp: 0, is_ib: false },
            ObZone { top: 110.0, bot: 106.0, state: ObState::Vierge, impulse_bar: 20, ob_bar: 19, timestamp: 0, is_ib: false },
        ];
        lc.update_trade(&mut t, true, &out, &bar(900, 100.0, 101.0, 99.5, 100.0), 1, &c, &mut sc, &zones, &[]);
        assert!(t.be_forced, "BE forcé appliqué");
        assert!(!sc.is_signaled(true, 10), "premier OB (A) un-signalé");
        assert!(sc.is_signaled(true, 20), "OB du trade (B) RESTE signalé — pas de re-trade");
    }

    #[test]
    fn fill_au_retest_bar_suivante() {
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 109.0);
        let lc = lc();
        let out = SmcOutput::default();
        let c = cal();
        let mut sc = scoring();
        // Bar 1 (ts=900>0), low=99.5 <= entry 100 → fill.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(900, 101.0, 102.0, 99.5, 101.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert!(t.filled);
        assert_eq!(t.fill_ts, Some(900));
    }

    #[test]
    fn sl_hit_avant_tp1() {
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 109.0);
        t.filled = true;
        let lc = lc();
        let out = SmcOutput::default();
        let c = cal();
        let mut sc = scoring();
        // low=96 < sl=97, !tp1_hit → slHit.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(900, 99.0, 100.0, 96.0, 97.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert_eq!(t.state, TradeState::Closed);
        assert_eq!(t.close_reason, Some(CloseReason::Sl));
        assert_eq!(t.verdict(), Verdict::Sl);
    }

    #[test]
    fn tp1_puis_be_donne_verdict_tp1() {
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 109.0);
        t.filled = true;
        let lc = lc();
        let out = SmcOutput::default();
        let c = cal();
        let mut sc = scoring();
        // Bar A : high=104 >= tp1=103 → tp1_hit, sl→entry(100), tp1_price_touched.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(900, 100.0, 104.0, 100.0, 103.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert!(t.tp1_hit);
        assert!(t.tp1_price_touched);
        assert!((t.sl - 100.0).abs() < 1e-9, "SL → entry après TP1");
        // Bar B : low=99 < entry=100, tp1_hit, tp2_ts==0 → beHit → verdict TP1.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(1800, 100.0, 101.0, 99.0, 100.0),
            2,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert_eq!(t.state, TradeState::Closed);
        assert_eq!(t.close_reason, Some(CloseReason::Be));
        assert_eq!(t.verdict(), Verdict::Tp1);
        assert!((t.realized_r() - 1.0).abs() < 1e-9, "TP1+BE = 1R acquis");
    }

    #[test]
    fn tp3_donne_verdict_tp3_distance_reelle() {
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 112.0);
        t.filled = true;
        let lc = lc();
        let out = SmcOutput::default();
        let c = cal();
        let mut sc = scoring();
        // high=112 >= tp3=112 → tp3Hit.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(900, 105.0, 112.0, 104.0, 111.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert_eq!(t.close_reason, Some(CloseReason::Tp3));
        assert_eq!(t.verdict(), Verdict::Tp3);
        // risk0=3, tp3-entry=12 → 4R.
        assert!((t.realized_r() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn be_force_par_bos_oppose_sans_tp1() {
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 109.0);
        t.filled = true;
        let lc = lc();
        let mut out = SmcOutput::default();
        out.bos_raw.bearish = true; // BOS baissier BRUT (opposé d'un BUY).
        let c = cal();
        let mut sc = scoring();
        // !tp1_hit && beForce → BE forcé : sl→entry, tp1_hit=true, be_forced.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(900, 100.0, 101.0, 99.5, 100.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert!(
            !matches!(t.state, TradeState::Closed),
            "BE forcé : trade maintenu ouvert"
        );
        assert!(t.be_forced);
        assert!((t.sl - 100.0).abs() < 1e-9);
        assert!(t.tp1_hit);
        assert!(!t.tp1_price_touched, "BE forcé ≠ TP1 prix touché");
        // Bar suivante : low=99 < entry=100, tp1_hit, tp2_ts==0 → beHit → verdict BE (0R).
        let out2 = SmcOutput::default();
        lc.update_trade(
            &mut t,
            true,
            &out2,
            &bar(1800, 100.0, 100.5, 99.0, 100.0),
            2,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert_eq!(t.verdict(), Verdict::Be);
        assert!((t.realized_r() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn expiration_age_max() {
        let mut t = buy_trade(100.0, 97.0, 103.0, 106.0, 109.0);
        t.filled = true;
        let lc = lc();
        let out = SmcOutput::default();
        let c = cal();
        let mut sc = scoring();
        // age = 15000s > 14400 → expire.
        lc.update_trade(
            &mut t,
            true,
            &out,
            &bar(15000, 100.0, 101.0, 99.5, 100.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert_eq!(t.close_reason, Some(CloseReason::Expire));
        assert_eq!(t.verdict(), Verdict::Expire);
    }

    #[test]
    fn sell_sl_hit_miroir() {
        let mut t = Trade::new_sell(
            1,
            TradeSource::Ob,
            100.0,
            103.0,
            97.0,
            94.0,
            91.0,
            10,
            3.0,
            &bar(0, 0.0, 0.0, 0.0, 0.0),
            0,
            None,
        );
        t.filled = true;
        let lc = lc();
        let out = SmcOutput::default();
        let c = cal();
        let mut sc = scoring();
        // SELL : sl_hit = high > sl=103, !tp1_hit.
        lc.update_trade(
            &mut t,
            false,
            &out,
            &bar(900, 102.0, 104.0, 101.0, 103.0),
            1,
            &c,
            &mut sc,
            &[],
            &[],
        );
        assert_eq!(t.close_reason, Some(CloseReason::Sl));
    }
