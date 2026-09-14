//! Moteur de rejeu — transcription fidèle de la définition §3-4 :
//! conditions à la clôture, exécution à l'open suivant, niveaux figés à
//! l'entrée, crosses (pas niveaux) pour TP/SL, retournement en un open commun.

use chrono::{DateTime, Utc};
use common::Candle;
use indicators::{calculer_adx, calculer_ema, calculer_halftrend, calculer_kdj, calculer_sma};

use crate::{ParamsKdj, TradeKdj, Verdict};

struct PositionOuverte {
    direction: i32,
    ts_entree: DateTime<Utc>,
    prix_entree: f64,
    niveau_tp: f64,
    niveau_sl: f64,
    risque: f64,
}

/// Rejoue la stratégie sur l'historique (bougies triées chronologiquement)
/// et retourne les trades dans l'ordre. La position éventuellement encore
/// ouverte en fin de données est renvoyée avec `Verdict::Ouvert`.
pub fn rejouer(bougies: &[Candle], params: &ParamsKdj) -> Vec<TradeKdj> {
    let n = bougies.len();
    let mut trades: Vec<TradeKdj> = Vec::new();
    if n < 2 {
        return trades;
    }

    let kdj = calculer_kdj(bougies, params.period, params.signal);
    let ht = calculer_halftrend(bougies, params.amplitude);
    let sma100 = calculer_sma(bougies, 100);
    let ema200 = calculer_ema(bougies, 200);
    // Filtre de tendance (7.E) : calculé seulement si demandé.
    let adx = params.adx_min.map(|_| calculer_adx(bougies, 14));

    let mut pos: Option<PositionOuverte> = None;
    // Ordres générés à la clôture de la barre précédente → exécution à open(i).
    let mut entree_differee: Option<i32> = None;
    let mut sortie_differee: Option<Verdict> = None;

    for i in 0..n {
        // ── Exécutions à l'open de la barre courante ──────────────────────
        if let Some(verdict) = sortie_differee.take() {
            if let Some(p) = pos.take() {
                trades.push(cloturer(p, bougies[i].open, bougies[i].timestamp, verdict));
            }
        }
        if let Some(direction) = entree_differee.take() {
            if pos.is_none() {
                pos = Some(ouvrir(direction, i, bougies, ema200[i], params.ratio_risk));
            }
        }

        // ── Évaluation à la clôture de la barre i ────────────────────────
        let long_ok = conditions_long(i, bougies, &kdj, &sma100[i], &ema200[i]);
        let short_ok = conditions_short(i, bougies, &kdj, &sma100[i], &ema200[i]);
        let tendance_ok = match params.adx_min {
            None => true,
            Some(min) => adx.as_ref().is_some_and(|a| a[i].is_finite() && a[i] >= min),
        };
        let buy = ht.arrow_up[i] && long_ok && tendance_ok;
        let sell = ht.arrow_down[i] && short_ok && tendance_ok;

        if let Some(p) = &pos {
            let (tp, sl) = crosses(i, bougies, p);
            if tp {
                sortie_differee = Some(Verdict::Tp);
            } else if sl {
                sortie_differee = Some(Verdict::Sl);
            }
            // Retournement : signal inverse complet ferme l'ancienne et
            // ouvre la nouvelle au MÊME open (modèle TV). Si un TP/SL vient
            // d'être détecté, il garde le verdict (détection la plus ancienne
            // de la même clôture) — l'entrée inverse a quand même lieu.
            if (p.direction == 1 && sell) || (p.direction == -1 && buy) {
                if sortie_differee.is_none() {
                    sortie_differee = Some(Verdict::Retournement);
                }
                entree_differee = Some(-p.direction);
            }
        } else if buy {
            entree_differee = Some(1);
        } else if sell {
            entree_differee = Some(-1);
        }
    }

    // Fin de données : position résiduelle, sans clôture ni R.
    if let Some(p) = pos {
        trades.push(TradeKdj {
            direction: p.direction,
            ts_entree: p.ts_entree,
            ts_sortie: None,
            prix_entree: p.prix_entree,
            prix_sortie: None,
            niveau_tp: p.niveau_tp,
            niveau_sl: p.niveau_sl,
            risque: p.risque,
            verdict: Verdict::Ouvert,
            r_brut: None,
            r_net: None,
        });
    }
    trades
}

/// Ouvre à open(i) ; EMA200_E = EMA200 de la barre d'ENTRÉE (piège 3 —
/// valuewhen capture la barre d'exécution, pas celle du signal).
fn ouvrir(direction: i32, i: usize, bougies: &[Candle], ema_e: f64, ratio_risk: f64) -> PositionOuverte {
    let e = bougies[i].open;
    // risque% de l'étalon : (E − EMA200_E)/E ; TP = E ± RatioRisk × (E − EMA200_E).
    let distance = e - ema_e;
    let (niveau_tp, niveau_sl, risque) = if direction == 1 {
        (e + ratio_risk * distance, ema_e, distance.abs())
    } else {
        (e - ratio_risk * (ema_e - e), ema_e, (ema_e - e).abs())
    };
    PositionOuverte {
        direction,
        ts_entree: bougies[i].timestamp,
        prix_entree: e,
        niveau_tp,
        niveau_sl,
        risque,
    }
}

fn conditions_long(
    i: usize,
    bougies: &[Candle],
    kdj: &indicators::Kdj,
    sma100: &f64,
    ema200: &f64,
) -> bool {
    sma100.is_finite()
        && ema200.is_finite()
        && kdj.j[i].is_finite()
        && kdj.d[i].is_finite()
        && *sma100 > *ema200
        && kdj.j[i] > kdj.d[i]
        && bougies[i].close > *ema200
}

fn conditions_short(
    i: usize,
    bougies: &[Candle],
    kdj: &indicators::Kdj,
    sma100: &f64,
    ema200: &f64,
) -> bool {
    sma100.is_finite()
        && ema200.is_finite()
        && kdj.j[i].is_finite()
        && kdj.d[i].is_finite()
        && *sma100 < *ema200
        && kdj.j[i] < kdj.d[i]
        && bougies[i].close < *ema200
}

/// Crosses TP/SL à la clôture de la barre i (piège 1 : seuils de détection ;
/// piège 4 : ce sont des CROISEMENTS — un low déjà sous le niveau à la barre
/// précédente ne « cross » pas).
fn crosses(i: usize, bougies: &[Candle], p: &PositionOuverte) -> (bool, bool) {
    if i == 0 {
        return (false, false);
    }
    let (h_prec, l_prec) = (bougies[i - 1].high, bougies[i - 1].low);
    let (h, l) = (bougies[i].high, bougies[i].low);
    if p.direction == 1 {
        let tp = h_prec <= p.niveau_tp && h > p.niveau_tp;
        let sl = l_prec >= p.niveau_sl && l < p.niveau_sl;
        (tp, sl)
    } else {
        let tp = l_prec >= p.niveau_tp && l < p.niveau_tp;
        let sl = h_prec <= p.niveau_sl && h > p.niveau_sl;
        (tp, sl)
    }
}

fn cloturer(
    p: PositionOuverte,
    prix: f64,
    ts: DateTime<Utc>,
    verdict: Verdict,
) -> TradeKdj {
    // Risque nul (E == EMA200_E, dégénéré) : R = 0 — le trade existe quand
    // même, compté au verdict.
    let r = if p.risque > 0.0 {
        Some(p.direction as f64 * (prix - p.prix_entree) / p.risque)
    } else {
        Some(0.0)
    };
    TradeKdj {
        direction: p.direction,
        ts_entree: p.ts_entree,
        ts_sortie: Some(ts),
        prix_entree: p.prix_entree,
        prix_sortie: Some(prix),
        niveau_tp: p.niveau_tp,
        niveau_sl: p.niveau_sl,
        risque: p.risque,
        verdict,
        r_brut: r,
        r_net: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParamsKdj;

    fn bougie_ts(ts: i64, p: f64) -> Candle {
        Candle {
            timestamp: Utc::now() + chrono::Duration::seconds(ts),
            open: p,
            high: p + 0.4,
            low: p - 0.4,
            close: p,
            volume: 1000.0,
        }
    }

    fn rampe(depart: f64, pas: f64, nb: usize, ts_debut: i64) -> Vec<Candle> {
        (0..nb)
            .map(|k| bougie_ts(ts_debut + k as i64, depart + pas * k as f64))
            .collect()
    }

    #[test]
    fn montee_pure_aucun_trade() {
        // Pas de retournement HalfTrend → pas de flèche → zéro trade.
        let b = rampe(100.0, 1.0, 400, 0);
        assert!(rejouer(&b, &ParamsKdj::default()).is_empty());
    }

    #[test]
    fn serie_courte_aucun_trade() {
        // Warmup : EMA200/SMA100 en NaN → conditions jamais réunies.
        let b = rampe(100.0, 1.0, 150, 0);
        assert!(rejouer(&b, &ParamsKdj::default()).is_empty());
    }

    #[test]
    fn fleche_sans_conditions_aucun_trade() {
        // Chute longue (flèche vendeuse probable) mais après une longue
        // montée : SMA100 > EMA200 → conditions short impossibles → rien.
        let mut b = rampe(100.0, 1.0, 260, 0);
        b.extend(rampe(360.0, -2.0, 120, 260));
        assert!(rejouer(&b, &ParamsKdj::default()).is_empty());
    }

    #[test]
    fn filtre_adx_desactive_ne_change_rien() {
        let mut b = rampe(100.0, 1.0, 300, 0);
        b.extend(rampe(400.0, -2.0, 60, 300));
        b.extend(rampe(280.0, 2.0, 150, 360));
        b.extend(rampe(578.0, -2.0, 40, 510));
        b.extend(rampe(500.0, 40.0, 12, 550));
        let sans = rejouer(&b, &ParamsKdj::default());
        let seuil0 = rejouer(&b, &ParamsKdj { adx_min: Some(0.0), ..ParamsKdj::default() });
        assert_eq!(sans.len(), seuil0.len());
        let infin = rejouer(&b, &ParamsKdj { adx_min: Some(f64::INFINITY), ..ParamsKdj::default() });
        assert!(infin.is_empty(), "ADX ∞ doit tout filtrer");
    }

    #[test]
    fn v_violent_trade_long_tp() {
        // Le nœud de la stratégie : la flèche HalfTrend arrive 2-3 barres
        // après un creux, mais le KDJ (lissé) ne s'est retourné que si le V
        // est violent ET le D écrasé par une chute longue. Scénario :
        // montée 300 → chute 60 (flèche DN sans conditions short) →
        // remontée 150 (flèche UP prématurée : close sous EMA200, rien) →
        // rechute 40 barres (D du KDJ écrasé vers 0) → V violent +40 :
        // flèche UP, KDJ retourné (k>d), close>SMA100>EMA200 → entrée long,
        // TP touché par la poursuite.
        let mut b = rampe(100.0, 1.0, 300, 0);
        b.extend(rampe(400.0, -2.0, 60, 300));
        b.extend(rampe(280.0, 2.0, 150, 360));
        b.extend(rampe(578.0, -2.0, 40, 510));
        b.extend(rampe(500.0, 40.0, 12, 550));
        let trades = rejouer(&b, &ParamsKdj::default());
        assert!(
            !trades.is_empty(),
            "au moins un trade attendu (V violent : flèche + conditions alignées)"
        );
        let t = &trades[0];
        assert_eq!(t.direction, 1);
        assert_eq!(t.verdict, Verdict::Tp);
        assert!(t.r_brut.unwrap_or(0.0) > 0.0, "TP long doit gagner : {:?}", t.r_brut);
        // Niveaux figés : SL = EMA200 de la barre d'entrée < entrée (long).
        assert!(t.niveau_sl < t.prix_entree);
        assert!(t.niveau_tp > t.prix_entree);
    }
}
