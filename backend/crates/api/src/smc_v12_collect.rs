//! Collecte des indicateurs étendus du moteur SMC v12 pour `/api/smc/v12/analyse`.
//!
//! Deux responsabilités :
//! - [`BarCollectors`] accumule pendant le replay les indicateurs « par barre »
//!   (sessions Kill Zones, volume fort, impulsion, zone-cœur **live** — miroir
//!   des boxes Pine supprimées à l'invalidation, Asian High/Low non tracké
//!   par `KillZoneDetector`).
//! - [`collect_final_extended`] lit les états finaux sur le moteur post-replay
//!   (liquidités PDH/PDL/PWH/PWL + EQH/EQL, breaker, imbalance, OTE,
//!   Premium/Discount, MTF, NDOG/NWOG) et compresse en run-length les séries
//!   par barre accumulées dans les collecteurs.

use smc::v12::{BarInput, HtfState, ImbalanceState, KillZone, SmcOutput, SmcV12Engine};
use std::collections::HashMap;

use crate::smc_v12_out::*;

/// Mappe une KillZone vers un label de session rendu (`None` si hors session).
/// NyAm et NyPm sont regroupées en "ny" (rendu bgcolor unifié New York).
pub(crate) fn kz_label(z: KillZone) -> Option<&'static str> {
    match z {
        KillZone::Asian => Some("asie"),
        KillZone::London => Some("londres"),
        KillZone::NyAm | KillZone::NyPm => Some("ny"),
        KillZone::None => None,
    }
}

fn ib_state_str(s: ImbalanceState) -> &'static str {
    match s {
        ImbalanceState::Fresh => "vierge",
        ImbalanceState::Partial => "partiel",
    }
}

/// Collecte les OB HTF (bull puis bear) d'un timeframe dans le vecteur de sortie.
fn collect_htf(tf: &'static str, st: &HtfState, out: &mut Vec<HtfObOut>) {
    for z in &st.bull_obs {
        out.push(HtfObOut {
            timeframe: tf,
            dir: "bull",
            top: z.top,
            bot: z.bot,
            ts: z.timestamp,
        });
    }
    for z in &st.bear_obs {
        out.push(HtfObOut {
            timeframe: tf,
            dir: "bear",
            top: z.top,
            bot: z.bot,
            ts: z.timestamp,
        });
    }
}

// ── Collecteurs par barre ────────────────────────────────────────────────────

/// Collecteurs des indicateurs « par barre » accumulés pendant le replay :
/// sessions, volume fort, impulsion, zone-cœur (dédoublonnée), Asian High/Low.
pub(crate) struct BarCollectors {
    /// `i_atrSeuil` par asset (Pine MODULE 10 : RANGE > seuil × ATR).
    atr_seuil: f64,
    /// Tendance par barre ("bull"|"bear"|None) — bgcolor Pine MODULE 1.
    trend_raw: Vec<(i64, Option<&'static str>)>,
    /// Premium/discount par barre (Pine MODULE 4b bgcolor) : "prem"|"disc"|None.
    prem_raw: Vec<(i64, Option<&'static str>)>,
    /// Box en cours : (label, (début FIXE, fin FIXE), high, low).
    sess_cur: Option<(&'static str, (i64, i64), f64, f64)>,
    sess_boxes: Vec<crate::smc_v12_out::SessionBox>,
    dernier_ts: i64,
    vol_raw: Vec<(i64, bool)>,
    imp_raw: Vec<(i64, Option<&'static str>)>,
    vol_buf: Vec<f64>,
    asian_day_key: Option<i64>,
    asian_h: f64,
    asian_l: f64,
    asian_inv_up: bool,
    asian_inv_down: bool,
    /// 1re bougie de la session Asie (bord gauche des lignes, Pine _ahStartBar).
    asian_start_ts: i64,
}

impl BarCollectors {
    pub(crate) fn new(cap: usize, atr_seuil: f64) -> Self {
        Self {
            atr_seuil,
            trend_raw: Vec::with_capacity(cap),
            prem_raw: Vec::with_capacity(cap),
            sess_cur: None,
            sess_boxes: Vec::new(),
            dernier_ts: 0,
            vol_raw: Vec::with_capacity(cap),
            imp_raw: Vec::with_capacity(cap),
            vol_buf: Vec::with_capacity(21),
            asian_day_key: None,
            asian_h: 0.0,
            asian_l: 0.0,
            asian_start_ts: 0,
            asian_inv_up: false,
            asian_inv_down: false,
        }
    }

    /// Finalise la box d'encours : bornes FIXES, rétention illimitée (27/08).
    fn finaliser_session(&mut self) {
        if let Some((l, (debut, fin), hi, lo)) = self.sess_cur.take() {
            self.sess_boxes.push(crate::smc_v12_out::SessionBox {
                start_ts: debut,
                end_ts: fin,
                session: l,
                high: hi,
                low: lo,
            });
        }
    }

    /// Fin d'analyse : pousse la box d'encours.
    pub(crate) fn finaliser_session_pub(&mut self) {
        self.finaliser_session();
    }

    pub(crate) fn on_bar(&mut self, bar: &BarInput, out: &SmcOutput) {
        // (Bande kill-zone supprimée — sessions via rectangles MODULE 14.)
        let _ = kz_label(out.kill_zone.zone);

        // ── Tendance par barre (Pine MODULE 1 : bullCount>=2 / bearCount>=2) ──
        let trend = if out.structure.tendance_haussiere {
            Some("bull")
        } else if out.structure.tendance_baissiere {
            Some("bear")
        } else {
            None
        };
        self.trend_raw.push((bar.timestamp, trend));

        // ── Premium/Discount par barre (Pine MODULE 4b bgcolor).
        let prem = if out.premium_discount.in_premium {
            Some("prem")
        } else if out.premium_discount.in_discount {
            Some("disc")
        } else {
            None
        };
        self.prem_raw.push((bar.timestamp, prem));
        self.dernier_ts = bar.timestamp;

        // ── Boxes sessions complètes (Pine MODULE 14, heures Europe/Paris) ──
        // Asie 00:00-06:30, Londres 08:00-16:30, NY 14:30-21:00 (Paris).
        let utc = chrono::DateTime::from_timestamp(bar.timestamp, 0).unwrap_or_default();
        let paris = utc.with_timezone(&chrono_tz::Europe::Paris);
        use chrono::Timelike as _;
        let mins = paris.hour() as i64 * 60 + paris.minute() as i64;
        let sess_label = if mins < 390 {
            Some("asie")
        } else if (480..990).contains(&mins) {
            Some("londres")
        } else if (870..1260).contains(&mins) {
            Some("ny")
        } else {
            None
        };
        match (sess_label, &self.sess_cur) {
            (Some(l), Some((cur, _start, hi, lo))) if *cur == l => {
                // Étendre (bornes recalculées — bascule minuit pour l'Asie).
                let hi = hi.max(bar.high);
                let lo = lo.min(bar.low);
                let bornes = crate::smc_v12_out::bornes_session_paris(l, &paris);
                self.sess_cur = Some((l, bornes, hi, lo));
            }
            (Some(l), _) => {
                self.finaliser_session();
                // ANCRAGE TEMPOREL : heure Paris FIXE, pas la 1re barre.
                let bornes = crate::smc_v12_out::bornes_session_paris(l, &paris);
                self.sess_cur = Some((l, bornes, bar.high, bar.low));
            }
            (None, _) => self.finaliser_session(),
        }

        // ── Volume fort : volume > SMA(volume, 20) (inclut la bar courante) ──
        self.vol_buf.push(bar.volume);
        if self.vol_buf.len() > 20 {
            self.vol_buf.remove(0);
        }
        let vol_ma = if self.vol_buf.is_empty() {
            0.0
        } else {
            self.vol_buf.iter().sum::<f64>() / self.vol_buf.len() as f64
        };
        self.vol_raw
            .push((bar.timestamp, vol_ma > 0.0 && bar.volume > vol_ma));

        // ── Impulsion (Pine MODULE 10) : RANGE > i_atrSeuil × ATR14.
        let atr_ok = out.atr14 > 0.0 && (bar.high - bar.low) > self.atr_seuil * out.atr14;
        let imp = if atr_ok {
            if bar.close > bar.open {
                Some("bull")
            } else {
                Some("bear")
            }
        } else {
            None
        };
        self.imp_raw.push((bar.timestamp, imp));

        // ── Asian High/Low (Pine MODULE 14) : session Asie EUROPE/PARIS
        //    00:00-06:30 (SES_PARIS_ASIE 0-390 min) — pas la KZ UTC 3h.
        //    Range high/low étendue pendant la session, figée après ;
        //    invalidation par CLOSE franchissant le niveau (Pine
        //    close > _ahHighDrawn / close < _ahLowDrawn → ligne supprimée).
        let dk = bar.timestamp.div_euclid(86_400);
        let en_asie_paris = mins < 390;
        if en_asie_paris {
            if self.asian_day_key != Some(dk) {
                self.asian_day_key = Some(dk);
                self.asian_h = bar.high;
                self.asian_l = bar.low;
                self.asian_inv_up = false;
                self.asian_inv_down = false;
                self.asian_start_ts = bar.timestamp;
            } else {
                if bar.high > self.asian_h {
                    self.asian_h = bar.high;
                }
                if bar.low < self.asian_l {
                    self.asian_l = bar.low;
                }
            }
        } else if self.asian_day_key.is_some() {
            if bar.close > self.asian_h {
                self.asian_inv_up = true;
            }
            if bar.close < self.asian_l {
                self.asian_inv_down = true;
            }
        }
    }
}

/// Assemble les sorties étendues après le replay : états finaux lus sur le moteur
/// + compression run-length des séries par barre accumulées dans `col`.
pub(crate) fn collect_final_extended(
    engine: &SmcV12Engine,
    ts_by_idx: &[i64],
    mut col: BarCollectors,
) -> ExtendedOutputs {
    col.finaliser_session_pub();
    // ── Liquidités PDH/PDL/PWH/PWL (état final) ──
    let liq = engine.liquidites.last_event();
    let liquidites = vec![
        LiquiditeLevelOut {
            level: "pdh",
            price: liq.pdh,
            active: liq.pdh_active.is_some(),
            ts_origine: liq.pdh_ts,
        },
        LiquiditeLevelOut {
            level: "pdl",
            price: liq.pdl,
            active: liq.pdl_active.is_some(),
            ts_origine: liq.pdl_ts,
        },
        LiquiditeLevelOut {
            level: "pwh",
            price: liq.pwh,
            active: liq.pwh_active.is_some(),
            ts_origine: liq.pwh_ts,
        },
        LiquiditeLevelOut {
            level: "pwl",
            price: liq.pwl,
            active: liq.pwl_active.is_some(),
            ts_origine: liq.pwl_ts,
        },
    ];

    // ── EQH/EQL — « décisions trading » : touchés (sweepés) = DISPARUS.
    let eqs: Vec<EqOut> = engine
        .liquidites
        .pool()
        .iter()
        .filter(|l| !l.swept)
        .map(|l| EqOut {
            dir: if l.is_high { "high" } else { "low" },
            price: l.price,
            touches: l.touches,
            swept: l.swept,
            bar_idx: l.t_first,
            ts: 0,
        })
        .collect();

    // ── Breaker blocks (FIFO 5/sens côté moteur) ──
    let mut propulsions: Vec<crate::smc_v12_out::PropulsionOut> = Vec::new();
    for z in engine.propulsion.bull_zones() {
        propulsions.push(crate::smc_v12_out::PropulsionOut {
            ts: ts_at(ts_by_idx, z.bar, 0),
            dir: "bull",
            top: z.top,
            bot: z.bot,
        });
    }
    for z in engine.propulsion.bear_zones() {
        propulsions.push(crate::smc_v12_out::PropulsionOut {
            ts: ts_at(ts_by_idx, z.bar, 0),
            dir: "bear",
            top: z.top,
            bot: z.bot,
        });
    }

    let mut breakers: Vec<BreakerOut> = Vec::new();
    for z in engine.breaker.bull_zones() {
        breakers.push(BreakerOut {
            ts: ts_at(ts_by_idx, z.bar, 0),
            dir: "bull",
            top: z.top,
            bot: z.bot,
            bar_idx: z.bar,
        });
    }
    for z in engine.breaker.bear_zones() {
        breakers.push(BreakerOut {
            ts: ts_at(ts_by_idx, z.bar, 0),
            dir: "bear",
            top: z.top,
            bot: z.bot,
            bar_idx: z.bar,
        });
    }

    // ── Imbalance (FIFO 10/sens côté moteur) ──
    let mut imbalances: Vec<ImbalanceOut> = Vec::new();
    for z in engine.imbalance.bull_zones() {
        imbalances.push(ImbalanceOut {
            ts: ts_at(ts_by_idx, z.bar, 0),
            dir: "bull",
            top: z.top,
            bot: z.bot,
            state: ib_state_str(z.state),
            bar_idx: z.bar,
        });
    }
    for z in engine.imbalance.bear_zones() {
        imbalances.push(ImbalanceOut {
            ts: ts_at(ts_by_idx, z.bar, 0),
            dir: "bear",
            top: z.top,
            bot: z.bot,
            state: ib_state_str(z.state),
            bar_idx: z.bar,
        });
    }

    // ── OTE swing institutionnelle (spec 08/10 § 2.2) : zone vivante unique,
    //    jamais d'historique figé (supprimée au premier toucher / jambe). ──
    let swing_ote = engine.swing_ote.zone().map(|z| SwingOteOut {
        dir: if z.bearish { "bear" } else { "bull" },
        top: z.top,
        bot: z.bot,
        mid: z.mid,
        ts_naissance: z.ts_naissance,
        pivot_haut: SwingAnchorOut {
            prix: z.haut.prix,
            ts: z.haut.ts,
        },
        pivot_bas: SwingAnchorOut {
            prix: z.bas.prix,
            ts: z.bas.ts,
        },
    });

    // ── Premium/Discount (état final) ──
    let pd_ev = engine.premium_discount.last_event();
    let premium_discount = PdOut {
        pd_range_h: pd_ev.pd_range_h,
        pd_range_l: pd_ev.pd_range_l,
        equilibrium: pd_ev.equilibrium,
        in_premium: pd_ev.in_premium,
        in_discount: pd_ev.in_discount,
    };

    // ── MTF : OB HTF actifs par timeframe (3 max/sens/TF côté moteur) ──
    let mtf_ev = engine.mtf.last_event();
    let mut mtf_obs: Vec<HtfObOut> = Vec::new();
    collect_htf("H1", &mtf_ev.h1, &mut mtf_obs);
    collect_htf("H4", &mtf_ev.h4, &mut mtf_obs);
    collect_htf("W1", &mtf_ev.w1, &mut mtf_obs);
    collect_htf("MN", &mtf_ev.mn, &mut mtf_obs);

    // ── NDOG/NWOG (FIFO 1/type côté moteur) ──
    let mut gaps: Vec<GapOut> = Vec::new();
    for g in engine.ndog.ndog_zones() {
        gaps.push(GapOut {
            ts: ts_at(ts_by_idx, g.bar, 0),
            gtype: "ndog",
            top: g.top,
            bot: g.bot,
            ce: (g.top + g.bot) / 2.0,
            mitigated: g.mitigated,
            bar_idx: g.bar,
        });
    }
    for g in engine.ndog.nwog_zones() {
        gaps.push(GapOut {
            ts: ts_at(ts_by_idx, g.bar, 0),
            gtype: "nwog",
            top: g.top,
            bot: g.bot,
            ce: (g.top + g.bot) / 2.0,
            mitigated: g.mitigated,
            bar_idx: g.bar,
        });
    }

    col.finaliser_session_pub();
    col.finaliser_session();
    let BarCollectors {
        atr_seuil: _,
        trend_raw,
        prem_raw,
        sess_cur: _,
        sess_boxes,
        dernier_ts: _,
        vol_raw,
        imp_raw,
        vol_buf: _,
        asian_day_key,
        asian_h,
        asian_l,
        asian_inv_up,
        asian_inv_down,
        asian_start_ts,
    } = col;

    // ── Asian High/Low (range la plus récente observée) ──
    let asian_hl = asian_day_key.map(|_| AsianHlOut {
        high: asian_h,
        low: asian_l,
        invalidated_up: asian_inv_up,
        invalidated_down: asian_inv_down,
        start_ts: asian_start_ts,
    });

    // ── Compression run-length des séries par barre ──
    let vol_fort = compress_vol(&vol_raw)
        .into_iter()
        .map(|(st, en)| VolRange {
            start_ts: st,
            end_ts: en,
        })
        .collect();
    let impulsions = runs_str(&imp_raw)
        .into_iter()
        .map(|(st, en, s)| ImpRange {
            start_ts: st,
            end_ts: en,
            impulsion: s,
        })
        .collect();
    let trend_ranges = runs_str(&trend_raw)
        .into_iter()
        .map(|(st, en, s)| crate::smc_v12_out::TrendRange {
            start_ts: st,
            end_ts: en,
            dir: s,
        })
        .collect();

    ExtendedOutputs {
        liquidites,
        eqs,
        breakers,
        propulsions,
        imbalances,
        swing_ote,
        premium_discount,
        mtf_obs,
        sessions: Vec::new(), // supprimé : uniquement session_boxes (rectangles Pine)
        trend_ranges,
        prem_ranges: runs_str(&prem_raw)
            .into_iter()
            .map(|(st, en, s)| crate::smc_v12_out::PremRange {
                start_ts: st,
                end_ts: en,
                dir: s,
            })
            .collect(),
        session_boxes: sess_boxes,
        asian_hl,
        gaps,
        vol_fort,
        impulsions,
    }
}
