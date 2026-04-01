use crate::calculs::{simuler_sortie_pyramidal, ParamsPyramidal, SortieType, TradeDirection};
use common::Candle;

/// Résultats du straddle hybride : jambe1 fermée en premier (SL/TP), jambe2 survivante (SMC).
pub(crate) struct ResultatStraddle {
    pub jambe1: (f64, SortieType),
    pub dir1: TradeDirection,
    pub jambe2: (f64, SortieType),
    pub dir2: TradeDirection,
}

/// Prix de sortie simple pour une jambe sur une bougie (SL ou TP, sinon close).
fn px_simple(b: &Candle, sl: f64, tp: f64, is_long: bool) -> (f64, SortieType) {
    if is_long {
        if b.low <= sl {
            return (sl, SortieType::Sl);
        }
        if b.high >= tp {
            return (tp, SortieType::Tp1);
        }
    } else {
        if b.high >= sl {
            return (sl, SortieType::Sl);
        }
        if b.low <= tp {
            return (tp, SortieType::Tp1);
        }
    }
    (b.close, SortieType::Expiration)
}

/// Paramètres pour le straddle hybride (groupés pour respecter la limite Clippy de 7 args).
pub(crate) struct ParamsStraddleHybride {
    pub pe_l: f64,
    pub tp1_l: f64,
    pub sl_l: f64,
    pub tp2_l: f64,
    pub pe_s: f64,
    pub tp1_s: f64,
    pub sl_s: f64,
    pub tp2_s: f64,
    pub atr: f64,
    pub trail: f64,
    pub vente_partielle: bool,
    pub be: Option<(f64, f64)>,
}

/// Straddle hybride : Long + Short simultanés bougie par bougie.
/// Dès qu'une jambe ferme, la survivante bascule en SMC directionnel (pyramidal + trailing).
pub(crate) fn simuler_straddle_hybride(
    bougies: &[Candle],
    p: ParamsStraddleHybride,
) -> ResultatStraddle {
    let ParamsStraddleHybride {
        pe_l,
        tp1_l,
        sl_l,
        tp2_l,
        pe_s,
        tp1_s,
        sl_s,
        tp2_s,
        atr,
        trail,
        vente_partielle,
        be,
    } = p;
    let (mut sl_lc, mut sl_sc) = (sl_l, sl_s);
    for (idx, b) in bougies.iter().enumerate() {
        // Break-even optionnel sur les deux jambes simultanément
        if let Some((av, am)) = be {
            let s = av * am;
            if b.high >= pe_l + s {
                sl_lc = sl_lc.max(pe_l);
            }
            if b.low <= pe_s - s {
                sl_sc = sl_sc.min(pe_s);
            }
        }
        let lf = b.low <= sl_lc || b.high >= tp1_l;
        let sf = b.high >= sl_sc || b.low <= tp1_s;
        if lf || sf {
            let (pxl, tyl) = px_simple(b, sl_lc, tp1_l, true);
            let (pxs, tys) = px_simple(b, sl_sc, tp1_s, false);
            // Les deux ferment sur la même bougie → pas de phase pyramidale
            if lf && sf {
                return ResultatStraddle {
                    jambe1: (pxl, tyl),
                    dir1: TradeDirection::Long,
                    jambe2: (pxs, tys),
                    dir2: TradeDirection::Short,
                };
            }
            if lf {
                // Long ferme → Short passe en SMC pyramidal
                let surv = simuler_sortie_pyramidal(
                    &bougies[idx..],
                    &TradeDirection::Short,
                    ParamsPyramidal {
                        prix_entree: pe_s,
                        tp1: tp1_s,
                        tp2: tp2_s,
                        trailing_tp3: (atr, trail),
                        sl_initial: sl_sc,
                        vente_partielle,
                    },
                );
                return ResultatStraddle {
                    jambe1: (pxl, tyl),
                    dir1: TradeDirection::Long,
                    jambe2: surv,
                    dir2: TradeDirection::Short,
                };
            }
            // Short ferme → Long passe en SMC pyramidal
            let surv = simuler_sortie_pyramidal(
                &bougies[idx..],
                &TradeDirection::Long,
                ParamsPyramidal {
                    prix_entree: pe_l,
                    tp1: tp1_l,
                    tp2: tp2_l,
                    trailing_tp3: (atr, trail),
                    sl_initial: sl_lc,
                    vente_partielle,
                },
            );
            return ResultatStraddle {
                jambe1: (pxs, tys),
                dir1: TradeDirection::Short,
                jambe2: surv,
                dir2: TradeDirection::Long,
            };
        }
    }
    // Expiration des deux jambes
    let cf = bougies.last().map(|b| b.close).unwrap_or(pe_l);
    ResultatStraddle {
        jambe1: (cf, SortieType::Expiration),
        dir1: TradeDirection::Long,
        jambe2: (cf, SortieType::Expiration),
        dir2: TradeDirection::Short,
    }
}

// ── API haut niveau pour le moteur de backtest ───────────────────────────────

/// Résultat d'une jambe calculée par `preparer_jambes_straddle`.
pub(crate) struct JambeStraddle {
    pub prix_entree: f64,
    pub prix_sortie: f64,
    pub direction: TradeDirection,
    pub sortie: SortieType,
    pub sl_ref: f64,
}

/// Niveaux issus du signal (côté Long, le Short sera mis en miroir).
pub(crate) struct NiveauxSignalStraddle {
    pub take_profit: f64,
    pub stop_loss: f64,
    pub prix_entree: f64,
    pub tp2: f64,
}

/// Paramètres du moteur nécessaires au calcul des jambes.
pub(crate) struct ParamsMoteurStraddle {
    pub open: f64,
    pub friction: f64,
    pub atr: f64,
    pub trailing_mult: f64,
    pub be: Option<(f64, f64)>,
    pub vente_partielle: bool,
}

/// Calcule les deux jambes d'un straddle hybride et retourne prix entrée/sortie + SL de référence.
/// Le calcul du PnL et la mise à jour du capital restent dans le moteur (lib.rs).
pub(crate) fn preparer_jambes_straddle(
    horizon: &[Candle],
    n: NiveauxSignalStraddle,
    p: ParamsMoteurStraddle,
) -> [JambeStraddle; 2] {
    let pe_l = p.open * (1.0 + p.friction);
    let pe_s = p.open * (1.0 - p.friction);
    let dist_tp1 = n.take_profit - n.prix_entree;
    let dist_sl_v = n.prix_entree - n.stop_loss;
    let dist_tp2 = n.tp2 - n.prix_entree;
    let tp1_l = n.take_profit;
    let sl_l = n.stop_loss;
    let tp1_s = pe_s - dist_tp1;
    let sl_s = pe_s + dist_sl_v;
    let tp2_l = pe_l + dist_tp2;
    let tp2_s = pe_s - dist_tp2;

    let res = simuler_straddle_hybride(
        horizon,
        ParamsStraddleHybride {
            pe_l,
            tp1_l,
            sl_l,
            tp2_l,
            pe_s,
            tp1_s,
            sl_s,
            tp2_s,
            atr: p.atr,
            trail: p.trailing_mult,
            vente_partielle: p.vente_partielle,
            be: p.be,
        },
    );

    let pe1 = if matches!(res.dir1, TradeDirection::Long) {
        pe_l
    } else {
        pe_s
    };
    let sl1 = if matches!(res.dir1, TradeDirection::Long) {
        sl_l
    } else {
        sl_s
    };
    let pe2 = if matches!(res.dir2, TradeDirection::Long) {
        pe_l
    } else {
        pe_s
    };
    let sl2 = if matches!(res.dir2, TradeDirection::Long) {
        sl_l
    } else {
        sl_s
    };

    [
        JambeStraddle {
            prix_entree: pe1,
            prix_sortie: res.jambe1.0,
            direction: res.dir1,
            sortie: res.jambe1.1,
            sl_ref: sl1,
        },
        JambeStraddle {
            prix_entree: pe2,
            prix_sortie: res.jambe2.0,
            direction: res.dir2,
            sortie: res.jambe2.1,
            sl_ref: sl2,
        },
    ]
}
