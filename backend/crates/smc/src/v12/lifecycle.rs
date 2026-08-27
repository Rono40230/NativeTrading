//! Cycle de vie des trades — évaluation INTRABAR (Pine lignes 3791-4118).
//!
//! Reproduit fidèlement la machine à états Pine, évaluée sur CHAQUE bar (sans
//! `barstate.isconfirmed`) :
//!
//! 1. **Fill** (retest) — `time > openTs && low <= entry` (buy) / `high >= entry` (sell).
//! 2. **Expiration** — `age > tradeMaxMins` OU (TP2 armé `&& age > tp3MaxMins`).
//! 3. **Sorties** (ordre strict de précédence) :
//!    `slHit` (SL avant TP1) → `beHit` (BE après TP1, avant TP2) → `tp2SLHit`
//!    (après TP2, prix repassé sous TP1) → `tp3Hit` (TP3 atteint) → `expire`
//!    → `cancel` (ordre en attente + BOS opposé).
//! 4. **BE forcé** — si `filled && (beForce || scoreDeg) && !tp1Hit` : SL→entry,
//!    `tp1Hit=true` (mais pas `tp1_price_touched`). Trade maintenu ouvert.
//! 5. **Progression normale** — TP1 touché → SL→entry (BE) ; TP2 touché → arme TP3.
//!
//! Les milestones de prix (TP1/TP2/TP3 réellement touchés) sont suivis séparément
//! pour produire le verdict (TP1/TP2/TP3/SL/BE/Expire) et le R-multiple.

use super::calibration::AssetCalibration;
use super::scoring_v11::ScoringV11;
use super::trade::{CloseReason, TradeState};
    use super::types::{ObState, ObZone};
use super::types::{BarInput, SmcOutput};

/// Mode de gestion du BE forcé sur BOS opposé (étude comparatif 26/08 —
/// « 95 % des trades fermés à BE »). Classique = production fidèle Pine v12 ;
/// les autres modes servent au binaire `comparatif_be` pour trancher par
/// les chiffres. La dégradation de score (scoreDeg) n'est PAS concernée :
/// seule la cause BOS opposé varie entre modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeBeForce {
    /// SL → entrée au BOS opposé BRUT (Pine v12 — production).
    #[default]
    Classique,
    /// Clôture immédiate au prix du tick du BOS opposé (R partiel réalisé,
    /// positif ou négatif — pas de plancher 0R).
    Marche,
    /// Aucune action sur trade rempli : il vit jusqu'à SL/TP (l'annulation
    /// des ordres EN ATTENTE reste active — règle distincte).
    Supprime,
    /// BE uniquement sur MSS opposé (cassure avec displacement) — un
    /// micro-BOS ne suffit plus.
    Qualifie,
}

/// Gestionnaire du cycle de vie — opère sur le carnet de trades du `SignalGenerator`.
#[derive(Clone)]

pub struct TradeLifecycle {
    /// `i_tradeMaxMins × 60` en secondes (Pine 2374-2375).
    trade_max_secs: i64,
    /// `i_tp3MaxMins × 60` en secondes (Pine 2372).
    tp3_max_secs: i64,
    /// Mode du BE forcé (étude comparatif — défaut Classique = production).
    mode_be_force: ModeBeForce,
}

impl TradeLifecycle {
    pub fn new(trade_max_secs: i64, tp3_max_secs: i64) -> Self {
        Self {
            trade_max_secs,
            tp3_max_secs,
            mode_be_force: ModeBeForce::Classique,
        }
    }

    /// Sélectionne le mode du BE forcé (étude comparatif).
    pub fn definir_mode_be_force(&mut self, mode: ModeBeForce) {
        self.mode_be_force = mode;
    }

    /// Évalue tous les trades non clôturés sur la bar courante (Pine 3797-3952 / 3963-4118).
    pub fn update(
        &self,
        trades: &mut [super::trade::Trade],
        out: &SmcOutput,
        bar: &BarInput,
        bar_index: usize,
        cal: &AssetCalibration,
        scoring: &mut ScoringV11,
        ob_bull: &[super::types::ObZone],
        ob_bear: &[super::types::ObZone],
    ) {
        for t in trades.iter_mut() {
            if t.state == TradeState::Closed {
                continue;
            }
            let is_buy = matches!(t.side, super::trade::Side::Buy);
            self.update_trade(t, is_buy, out, bar, bar_index, cal, scoring, ob_bull, ob_bear);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_trade(
        &self,
        t: &mut super::trade::Trade,
        is_buy: bool,
        out: &SmcOutput,
        bar: &BarInput,
        bar_index: usize,
        cal: &AssetCalibration,
        scoring: &mut ScoringV11,
        ob_bull: &[super::types::ObZone],
        ob_bear: &[super::types::ObZone],
    ) {
        // Snapshot de l'état de contrôle (état en début de bar).
        let sl = t.sl;
        let entry = t.entry;
        let tp1 = t.tp1;
        let tp2 = t.tp2;
        let tp3 = t.tp3;
        let tp1_hit = t.tp1_hit;
        let tp2_ts = t.tp2_ts;
        let open_ts = t.open_ts;
        let filled = t.filled;

        // --- 1. Fill (retest) — Pine 3811 / 3977 ---
        // Modèle "Retest (limite)" : fill sur bar strictement après création,
        // quand le prix touche l'entrée (low<=entry buy / high>=entry sell).
        if !filled && bar.timestamp > open_ts {
            let touch_entry = if is_buy {
                bar.low <= entry
            } else {
                bar.high >= entry
            };
            if touch_entry {
                t.filled = true;
                t.fill_ts = Some(bar.timestamp);
                t.state = TradeState::Open;
            }
        }
        let filled = t.filled;

        // --- 2. Expiration — Pine 3851-3852 / 4017-4018 ---
        let age_expire = (bar.timestamp - open_ts) > self.trade_max_secs;
        let expire = (tp2_ts > 0 && (bar.timestamp - tp2_ts) > self.tp3_max_secs) || age_expire;

        // --- 3. Sorties (avec l'état de début de bar) — Pine 3853-3856 / 4019-4022 ---
        let (sl_hit, be_hit, tp2_sl_hit, tp3_hit) = if is_buy {
            (
                filled && bar.low < sl && !tp1_hit,
                filled && bar.low < entry && tp1_hit && tp2_ts == 0,
                filled && bar.low < tp1 && tp1_hit && tp2_ts > 0,
                filled && bar.high >= tp3,
            )
        } else {
            (
                filled && bar.high > sl && !tp1_hit,
                filled && bar.high > entry && tp1_hit && tp2_ts == 0,
                filled && bar.high > tp1 && tp1_hit && tp2_ts > 0,
                filled && bar.low <= tp3,
            )
        };

        // --- BOS opposé (beForce) + score degradation (scoreDeg) ---
        // beForce = !tp1_hit && BOS opposé BRUT (Pine `bosBaissier`/`bosHaussier`
        // lignes 457-458, jamais masqués par le filtre MSS — un BOS-MSS force
        // aussi le BE). Variantes de l'étude comparatif :
        // - Supprime : jamais (le trade vit jusqu'à SL/TP).
        // - Qualifie : MSS opposé uniquement (displacement), pas un micro-BOS.
        let bos_oppose = if is_buy { out.bos_raw.bearish } else { out.bos_raw.bullish };
        let mss_oppose = if is_buy { out.mss.mss_baissier } else { out.mss.mss_haussier };
        let be_force = match self.mode_be_force {
            ModeBeForce::Supprime => false,
            ModeBeForce::Qualifie => !tp1_hit && mss_oppose,
            ModeBeForce::Classique | ModeBeForce::Marche => !tp1_hit && bos_oppose,
        };
        // --- 4. Sortie si condition — précédence stricte Pine ---
        let close_reason = if sl_hit {
            Some(CloseReason::Sl)
        } else if be_hit {
            Some(CloseReason::Be)
        } else if tp2_sl_hit {
            Some(CloseReason::Tp2Sl)
        } else if tp3_hit {
            // Verdict TP3 : on enregistre le milestone tp3 touché.
            t.tp3_touched = true;
            Some(CloseReason::Tp3)
        } else if expire {
            Some(CloseReason::Expire)
        } else if !filled && be_force {
            Some(CloseReason::Cancel)
        } else {
            None
        };

        if let Some(reason) = close_reason {
            t.state = TradeState::Closed;
            t.close_reason = Some(reason);
            t.close_ts = Some(bar.timestamp);
            t.close_bar = Some(bar_index);
            t.close_r = Some(t.realized_r());
            return;
        }

        // --- 5. BE forcé — Pine 3908-3923 / 4074-4089 ---
        if filled && be_force && !tp1_hit {
            // Variante Marché : clôture immédiate au prix courant — le R
            // partiel est réalisé tel quel (souvent négatif : le BOS opposé
            // survient contre le trade).
            if self.mode_be_force == ModeBeForce::Marche && be_force {
                t.state = TradeState::Closed;
                t.close_reason = Some(CloseReason::Be);
                t.be_forced = true;
                t.close_ts = Some(bar.timestamp);
                t.close_bar = Some(bar_index);
                let r_marche = if t.risk0 > 0.0 {
                    if is_buy {
                        (bar.close - t.entry) / t.risk0
                    } else {
                        (t.entry - bar.close) / t.risk0
                    }
                } else {
                    0.0
                };
                t.close_r = Some(r_marche);
                return;
            }
            t.sl = entry; // SL → entry (BE).
            t.tp1_hit = true; // Neutralise (n'a plus besoin de TP1 gate).
            t.be_forced = true;
            t.state = TradeState::Open;
            // Pine 3936-3941 + 3987-3988 : un-signal le PREMIER OB signalé du
            // carneau (source OB uniquement — _srcBull == 0), PAS forcément
            // l'OB du trade. Sémantique exacte du Pine : limite les re-trades.
            if t.ob_key.is_some() {
                let zones = if is_buy { ob_bull } else { ob_bear };
                scoring.unmark_premier_signale(is_buy, zones);
            }
            return;
        }

        // --- 6. Progression normale — Pine 3934-3947 / 4100-4113 ---
        if filled {
            // TP1 touché → BE (SL→entry).
            if !tp1_hit {
                let touch_tp1 = if is_buy {
                    bar.high >= tp1
                } else {
                    bar.low <= tp1
                };
                if touch_tp1 {
                    t.tp1_hit = true;
                    t.tp1_price_touched = true;
                    t.sl = entry;
                }
            }
            // TP2 touché → arme TP3 (timestamp).
            if t.tp1_hit && t.tp2_ts == 0 {
                let touch_tp2 = if is_buy {
                    bar.high >= tp2
                } else {
                    bar.low <= tp2
                };
                if touch_tp2 {
                    t.tp2_ts = bar.timestamp;
                }
            }
            // tp3 milestone (cas où tp3 touché sans déclencher tp3_hit car !filled
            // au début — rempli cette bar ; on l'enregistre pour stats).
            if !t.tp3_touched {
                let touch_tp3 = if is_buy {
                    bar.high >= tp3
                } else {
                    bar.low <= tp3
                };
                if touch_tp3 {
                    t.tp3_touched = true;
                }
            }
        }
        if filled {
            t.state = TradeState::Open;
        }
    }
}

#[cfg(test)]
mod tests {
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
}
