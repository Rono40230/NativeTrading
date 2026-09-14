//! Mesures agrégées du rejeu — R moyen, profit factor, win rate, drawdown.

use crate::{TradeKdj, Verdict};

/// Frais par ordre, en montant absolu :
/// `commission` = fraction du prix (ex. 0.0005 = 0,05 %/ordre),
/// `slippage` = montant fixe par ordre (ex. 2 ticks × taille du tick).
#[derive(Debug, Clone, Copy)]
pub struct Frais {
    pub commission: f64,
    pub slippage: f64,
}

impl Default for Frais {
    fn default() -> Self {
        Frais { commission: 0.0005, slippage: 0.0 }
    }
}

/// Remplit `r_net` de chaque trade clôturé :
/// r_net = r_brut − (commission × (entrée + sortie) + 2 × slippage) / risque.
pub fn appliquer_frais(trades: &mut [TradeKdj], frais: Frais) {
    for t in trades.iter_mut() {
        let Some(sortie) = t.prix_sortie else { continue };
        if t.risque <= 0.0 || t.r_brut.is_none() {
            continue;
        }
        let cout = frais.commission * (t.prix_entree + sortie) + 2.0 * frais.slippage;
        t.r_net = t.r_brut.map(|r| r - cout / t.risque);
    }
}

/// Agrégats sur les trades CLÔTURÉS (les `Ouvert` sont comptés à part).
#[derive(Debug, Clone, Default)]
pub struct Mesures {
    pub nb_total: usize,
    pub nb_clotures: usize,
    pub nb_tp: usize,
    pub nb_sl: usize,
    pub nb_retournement: usize,
    pub nb_ouvert: usize,
    /// Part des trades clôturés avec r_net > 0 (0-1).
    pub win_rate: f64,
    /// Somme |R| des gains / somme |R| des pertes (net). ∞ si zéro perte.
    pub pf_net: f64,
    /// Profit factor brut (sans frais) — diagnostic d'écart frais.
    pub pf_brut: f64,
    pub r_moyen_net: f64,
    pub r_moyen_brut: f64,
    /// Max drawdown de la courbe cumulée des r_net (en R).
    pub dd_r: f64,
    /// P&L net moyen par trade en % du prix d'entrée — la métrique du
    /// screening TV ($ par unité), insensible aux risques quasi nuls qui
    /// font exploser les R (piège 3 : entrée collée à l'EMA200).
    pub p_moyen_pct: f64,
    /// Profit factor en % : Σ|gains %| / Σ|pertes %|.
    pub pf_pct: f64,
    /// Drawdown max de la cumule des P&L % (points de % cumulés).
    pub dd_pct: f64,
}

pub fn mesurer(trades: &[TradeKdj]) -> Mesures {
    let mut m = Mesures {
        nb_total: trades.len(),
        ..Mesures::default()
    };
    for t in trades {
        match t.verdict {
            Verdict::Ouvert => m.nb_ouvert += 1,
            Verdict::Tp => m.nb_tp += 1,
            Verdict::Sl => m.nb_sl += 1,
            Verdict::Retournement => m.nb_retournement += 1,
        }
    }
    let clots: Vec<&TradeKdj> = trades
        .iter()
        .filter(|t| t.verdict != Verdict::Ouvert)
        .collect();
    m.nb_clotures = clots.len();
    if clots.is_empty() {
        return m;
    }

    let r_de = |t: &TradeKdj| t.r_net.or(t.r_brut);
    let rs: Vec<f64> = clots.iter().filter_map(|t| r_de(t)).collect();

    let gains: f64 = rs.iter().filter(|&&r| r > 0.0).sum();
    let pertes: f64 = -rs.iter().filter(|&&r| r < 0.0).sum::<f64>();
    m.pf_net = if pertes > 0.0 { gains / pertes } else { f64::INFINITY };
    m.win_rate = rs.iter().filter(|&&r| r > 0.0).count() as f64 / rs.len() as f64;
    m.r_moyen_net = rs.iter().sum::<f64>() / rs.len() as f64;

    let bruts: Vec<f64> = clots.iter().filter_map(|t| t.r_brut).collect();
    let gains_b: f64 = bruts.iter().filter(|&&r| r > 0.0).sum();
    let pertes_b: f64 = -bruts.iter().filter(|&&r| r < 0.0).sum::<f64>();
    m.pf_brut = if pertes_b > 0.0 { gains_b / pertes_b } else { f64::INFINITY };
    m.r_moyen_brut = bruts.iter().sum::<f64>() / bruts.len() as f64;

    // Drawdown sur la cumule des R nets, dans l'ordre des trades.
    let mut cumul = 0.0_f64;
    let mut pic = 0.0_f64;
    let mut dd = 0.0_f64;
    for &r in &rs {
        cumul += r;
        pic = pic.max(cumul);
        dd = dd.max(pic - cumul);
    }
    m.dd_r = dd;

    // Métriques en % du prix (le coût est déduit de r_brut−r_net × risque).
    let mut ps: Vec<f64> = Vec::with_capacity(clots.len());
    for t in &clots {
        let Some(sortie) = t.prix_sortie else { continue };
        if t.prix_entree <= 0.0 {
            continue;
        }
        let brut = t.direction as f64 * (sortie - t.prix_entree);
        let cout = match (t.r_brut, t.r_net) {
            (Some(rb), Some(rn)) if t.risque > 0.0 => (rb - rn) * t.risque,
            _ => 0.0,
        };
        ps.push((brut - cout) / t.prix_entree * 100.0);
    }
    if !ps.is_empty() {
        let gains: f64 = ps.iter().filter(|&&p| p > 0.0).sum();
        let pertes: f64 = -ps.iter().filter(|&&p| p < 0.0).sum::<f64>();
        m.pf_pct = if pertes > 0.0 { gains / pertes } else { f64::INFINITY };
        m.p_moyen_pct = ps.iter().sum::<f64>() / ps.len() as f64;
        let mut c = 0.0_f64;
        let mut pk = 0.0_f64;
        for &p in &ps {
            c += p;
            pk = pk.max(c);
            m.dd_pct = m.dd_pct.max(pk - c);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn trade(r: f64, verdict: Verdict) -> TradeKdj {
        TradeKdj {
            direction: 1,
            ts_entree: Utc::now(),
            ts_sortie: Some(Utc::now()),
            prix_entree: 100.0,
            prix_sortie: Some(110.0),
            niveau_tp: 120.0,
            niveau_sl: 95.0,
            risque: 5.0,
            verdict,
            r_brut: Some(r),
            r_net: None,
        }
    }

    #[test]
    fn mesures_elementaires() {
        let mut trades = vec![trade(2.0, Verdict::Tp), trade(-1.0, Verdict::Sl), trade(0.5, Verdict::Tp)];
        appliquer_frais(&mut trades, Frais { commission: 0.0, slippage: 0.0 });
        let m = mesurer(&trades);
        assert_eq!(m.nb_total, 3);
        assert_eq!(m.nb_clotures, 3);
        assert_eq!(m.nb_tp, 2);
        assert_eq!(m.nb_sl, 1);
        assert!((m.win_rate - 2.0 / 3.0).abs() < 1e-9);
        assert!((m.pf_net - 2.5 / 1.0).abs() < 1e-9);
        assert!((m.r_moyen_net - 0.5).abs() < 1e-9);
        // Cumule 2, 1, 1.5 : le perdant creuse 1 R sous le pic.
        assert!((m.dd_r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ouvert_exclu_des_clotures() {
        let trades = vec![trade(1.0, Verdict::Tp), trade(0.0, Verdict::Ouvert)];
        let m = mesurer(&trades);
        assert_eq!(m.nb_total, 2);
        assert_eq!(m.nb_clotures, 1);
        assert_eq!(m.nb_ouvert, 1);
        assert!((m.win_rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn frais_reduisent_le_r() {
        // commission 0,05 %/ordre : (100+110)×0.0005 = 0.105 $ sur risque 5 $.
        let mut trades = vec![trade(2.0, Verdict::Tp)];
        appliquer_frais(&mut trades, Frais::default());
        let r_net = trades[0].r_net.expect("r_net rempli");
        assert!((r_net - (2.0 - 0.105 / 5.0)).abs() < 1e-9);
    }

    #[test]
    fn zero_perte_pf_infini() {
        let trades = vec![trade(1.0, Verdict::Tp), trade(2.0, Verdict::Tp)];
        let m = mesurer(&trades);
        assert!(m.pf_net.is_infinite());
    }
}
