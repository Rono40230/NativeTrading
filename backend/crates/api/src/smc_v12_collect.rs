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
fn kz_label(z: KillZone) -> Option<&'static str> {
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
    seuil_ib: f64,
    /// `i_atrSeuil` par asset (Pine MODULE 10 : RANGE > seuil × ATR).
    atr_seuil: f64,
    sessions_raw: Vec<(i64, Option<&'static str>)>,
    /// Tendance par barre ("bull"|"bear"|None) — bgcolor Pine MODULE 1.
    trend_raw: Vec<(i64, Option<&'static str>)>,
    /// Premium/discount par barre (Pine MODULE 4b bgcolor) : "prem"|"disc"|None.
    prem_raw: Vec<(i64, Option<&'static str>)>,
    /// Boxes de sessions complètes Paris (Pine MODULE 14) : encours + 24h.
    sess_cur: Option<(&'static str, i64, f64, f64)>, // (label, start, high, low)
    sess_boxes: Vec<crate::smc_v12_out::SessionBox>,
    dernier_ts: i64,
    vol_raw: Vec<(i64, bool)>,
    imp_raw: Vec<(i64, Option<&'static str>)>,
    vol_buf: Vec<f64>,
    zone_coeur: Vec<ZoneCoeurOut>,
    /// Timestamp de création de chaque box live (clé = (0=bull/1=bear, ob_bar)) —
    /// Pine fige la box à sa création, on mémorise la 1re barre où elle est vue.
    zc_crea: HashMap<(u8, usize), i64>,
    asian_day_key: Option<i64>,
    asian_h: f64,
    asian_l: f64,
    asian_inv_up: bool,
    asian_inv_down: bool,
    /// 1re bougie de la session Asie (bord gauche des lignes, Pine _ahStartBar).
    asian_start_ts: i64,
}

impl BarCollectors {
    pub(crate) fn new(cap: usize, seuil_ib: f64, atr_seuil: f64) -> Self {
        Self {
            seuil_ib,
            atr_seuil,
            sessions_raw: Vec::with_capacity(cap),
            trend_raw: Vec::with_capacity(cap),
            prem_raw: Vec::with_capacity(cap),
            sess_cur: None,
            sess_boxes: Vec::new(),
            dernier_ts: 0,
            vol_raw: Vec::with_capacity(cap),
            imp_raw: Vec::with_capacity(cap),
            vol_buf: Vec::with_capacity(21),
            zone_coeur: Vec::new(),
            zc_crea: HashMap::new(),
            asian_day_key: None,
            asian_h: 0.0,
            asian_l: 0.0,
            asian_start_ts: 0,
            asian_inv_up: false,
            asian_inv_down: false,
        }
    }

    /// Met à jour les collecteurs avec la sortie moteur d'une bar.
    /// Finalise la box de session en cours (si présente) et la pousse dans
    /// l'historique 24h (Pine MODULE 14 : garde-fou _24H_MS).
    fn finaliser_session(&mut self) {
        if let Some((l, start, hi, lo)) = self.sess_cur.take() {
            self.sess_boxes.push(crate::smc_v12_out::SessionBox {
                start_ts: start,
                end_ts: self.dernier_ts,
                session: l,
                high: hi,
                low: lo,
            });
            // Garde 24h : ne conserve que les boxes terminées il y a < 24h.
            let limite = self.dernier_ts - 24 * 3600;
            self.sess_boxes.retain(|b| b.end_ts >= limite);
        }
    }

    pub(crate) fn on_bar(&mut self, bar: &BarInput, out: &SmcOutput) {
        // ── Sessions (Kill Zones) ──
        // (sessions_raw supprimé : les sessions s'affichent uniquement via
        //  les rectangles session_boxes du MODULE 14 Pine.)
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

        // ── Premium/Discount par barre (Pine MODULE 4b : bgcolor
        //    inPremium/inDiscount, tolérance 0,5 % autour de l'equilibrium).
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
            (Some(l), Some((cur, start, hi, lo))) if *cur == l => {
                // étendre la box en cours
                let hi = hi.max(bar.high);
                let lo = lo.min(bar.low);
                self.sess_cur = Some((l, *start, hi, lo));
            }
            (Some(l), _) => {
                // finaliser la précédente puis ouvrir
                self.finaliser_session();
                self.sess_cur = Some((l, bar.timestamp, bar.high, bar.low));
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

        // ── Impulsion (Pine MODULE 10) : RANGE high-low > i_atrSeuil × ATR14
        //    (le corps était utilisé avant — et le mauvais seuil : atr_seuil
        //    ≠ seuil_ib ; Pine _autoAtrSeuil BTC=2.5, XAU=2.0).
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

        // ── Zone-cœur : miroir des boxes LIVE du moteur (Pine
        //    f_zoneCoeurLifecycle : supprimée dès que le setup n'est plus
        //    valable — on n'exporte que les zones vivantes, jamais un
        //    historique figé qui collerait à l'infini).
        self.zone_coeur.clear();
        for (sens, zones) in [(0u8, &out.zone_coeur.live_bull), (1u8, &out.zone_coeur.live_bear)] {
            for z in zones.iter() {
                let crea = *self.zc_crea.entry((sens, z.ob_bar)).or_insert(bar.timestamp);
                self.zone_coeur.push(ZoneCoeurOut {
                    ts: crea,
                    dir: if sens == 0 { "bull" } else { "bear" },
                    top: z.top,
                    bot: z.bot,
                    ob_bar: z.ob_bar,
                    ob_ts: 0,
                });
            }
        }

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

    // ── EQH/EQL (pool de liquidités) ──
    let eqs: Vec<EqOut> = engine
        .liquidites
        .pool()
        .iter()
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

    // ── OTE (Pine _oteBullBox/_oteBearBox, lignes 2126-2148) : box
    //    d'affichage créée au BOS (ts = bar du BOS), remplacée à chaque BOS,
    //    qui PERSISTE après expiration de la plage Fib — c'est elle qu'on
    //    exporte, pas la plage expirable (inOTE reste réservé au scoring). ──
    let mut otes: Vec<OteOut> = Vec::new();
    let ote_ev = engine.ote.last_event();
    if let Some((t, b, ts)) = ote_ev.bull_box {
        otes.push(OteOut {
            dir: "bull",
            top: t,
            bot: b,
            ts,
        });
    }
    if let Some((t, b, ts)) = ote_ev.bear_box {
        otes.push(OteOut {
            dir: "bear",
            top: t,
            bot: b,
            ts,
        });
    }

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
            mitigated: g.mitigated,
            bar_idx: g.bar,
        });
    }

    // Finaliser la dernière box de session avant collecte.
    col.finaliser_session();
    let BarCollectors {
        seuil_ib: _,
        atr_seuil: _,
        sessions_raw: _,
        trend_raw,
        prem_raw,
        sess_cur: _,
        sess_boxes,
        dernier_ts: _,
        vol_raw,
        imp_raw,
        vol_buf: _,
        mut zone_coeur,
        zc_crea: _,
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

    // ── Zone-cœur : bord gauche = bougie d'origine de l'OB parent (Pine
    //    box.new(obBullBar[_zi], …)) — pas la barre de détection.
    for z in zone_coeur.iter_mut() {
        z.ob_ts = ts_at(ts_by_idx, z.ob_bar, z.ts);
    }

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
        otes,
        zone_coeur,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_str_regroupe_plages_contigues_et_ignore_none() {
        // asie(0,1,2) | None(3) | londres(4,5) | None(6)
        let raw: Vec<(i64, Option<&str>)> = vec![
            (0, Some("asie")),
            (1, Some("asie")),
            (2, Some("asie")),
            (3, None),
            (4, Some("londres")),
            (5, Some("londres")),
            (6, None),
        ];
        let out = runs_str(&raw);
        assert_eq!(out, vec![(0, 2, "asie"), (4, 5, "londres")]);
    }

    #[test]
    fn runs_str_change_de_label_relance_une_plage() {
        // bull | bull | bear (pas de None) ⇒ deux plages collées.
        let raw: Vec<(i64, Option<&str>)> =
            vec![(10, Some("bull")), (11, Some("bull")), (12, Some("bear"))];
        let out = runs_str(&raw);
        assert_eq!(out, vec![(10, 11, "bull"), (12, 12, "bear")]);
    }

    #[test]
    fn runs_str_vide_renvoie_vide() {
        let raw: Vec<(i64, Option<&str>)> = vec![(0, None), (1, None)];
        assert!(runs_str(&raw).is_empty());
    }

    #[test]
    fn compress_vol_garde_uniquement_les_plages_fortes() {
        // fort(0,1) | faible(2) | fort(3)
        let raw: Vec<(i64, bool)> = vec![(0, true), (1, true), (2, false), (3, true)];
        let out = compress_vol(&raw);
        assert_eq!(out, vec![(0, 1), (3, 3)]);
    }

    #[test]
    fn compress_vol_aucun_fort_renvoie_vide() {
        let raw: Vec<(i64, bool)> = vec![(0, false), (1, false)];
        assert!(compress_vol(&raw).is_empty());
    }

    #[test]
    fn kz_label_regroupage_ny() {
        assert_eq!(kz_label(KillZone::Asian), Some("asie"));
        assert_eq!(kz_label(KillZone::London), Some("londres"));
        assert_eq!(kz_label(KillZone::NyAm), Some("ny"));
        assert_eq!(kz_label(KillZone::NyPm), Some("ny"));
        assert_eq!(kz_label(KillZone::None), None);
    }
}
