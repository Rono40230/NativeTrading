//! Calculs de métriques financières sur une série de trades backtestés.

use crate::{ResultatTrade, StatHeure, TradeBacktest};

/// Calcule le win rate (trades gagnants / total).
/// Retourne 0.0 si aucun trade.
pub fn calculer_win_rate(trades: &[TradeBacktest]) -> f64 {
    if trades.is_empty() {
        return 0.0;
    }
    let gagnants = trades
        .iter()
        .filter(|t| {
            matches!(
                t.resultat,
                ResultatTrade::Tp1 | ResultatTrade::Tp2 | ResultatTrade::Tp3
            )
        })
        .count();
    gagnants as f64 / trades.len() as f64
}

/// Calcule le profit factor : somme des gains / somme des pertes (en R).
/// Retourne 0.0 si aucune perte.
pub fn calculer_profit_factor(trades: &[TradeBacktest]) -> f64 {
    let gains: f64 = trades
        .iter()
        .filter(|t| t.pnl_r > 0.0)
        .map(|t| t.pnl_r)
        .sum();
    let pertes: f64 = trades
        .iter()
        .filter(|t| t.pnl_r < 0.0)
        .map(|t| t.pnl_r.abs())
        .sum();
    if pertes == 0.0 {
        return 0.0;
    }
    gains / pertes
}

/// Calcule le drawdown maximum sur la série de capital.
/// `capital_serie` : capital après chaque trade (croissant ou décroissant).
pub fn calculer_drawdown_max(capital_serie: &[f64]) -> f64 {
    if capital_serie.is_empty() {
        return 0.0;
    }
    let mut pic = capital_serie[0];
    let mut drawdown_max = 0.0;
    for &val in capital_serie {
        if val > pic {
            pic = val;
        }
        let dd = (pic - val) / pic;
        if dd > drawdown_max {
            drawdown_max = dd;
        }
    }
    drawdown_max
}

/// Calcule le ratio de Sharpe annualisé sur la série de P&L par trade (en R).
/// Utilise sqrt(252) comme facteur d'annualisation (jours de trading).
/// Retourne 0.0 si écart-type nul ou moins de 2 trades.
pub fn calculer_sharpe(pnl_serie: &[f64]) -> f64 {
    if pnl_serie.len() < 2 {
        return 0.0;
    }
    let n = pnl_serie.len() as f64;
    let moyenne = pnl_serie.iter().sum::<f64>() / n;
    let variance = pnl_serie.iter().map(|x| (x - moyenne).powi(2)).sum::<f64>() / (n - 1.0);
    let ecart_type = variance.sqrt();
    if ecart_type == 0.0 {
        return 0.0;
    }
    (moyenne / ecart_type) * 252_f64.sqrt()
}

/// Calcule les statistiques par créneau horaire (0-23).
pub fn calculer_stats_par_heure(trades: &[TradeBacktest]) -> Vec<StatHeure> {
    let mut stats: Vec<StatHeure> = (0u8..24)
        .map(|h| StatHeure {
            heure: h,
            nb_trades: 0,
            win_rate: 0.0,
            pnl_r_moyen: 0.0,
        })
        .collect();

    for trade in trades {
        let s = &mut stats[trade.heure_ouverture as usize];
        s.nb_trades += 1;
        let gagnant = matches!(
            trade.resultat,
            ResultatTrade::Tp1 | ResultatTrade::Tp2 | ResultatTrade::Tp3
        );
        s.win_rate += if gagnant { 1.0 } else { 0.0 };
        s.pnl_r_moyen += trade.pnl_r;
    }

    for s in &mut stats {
        if s.nb_trades > 0 {
            let n = s.nb_trades as f64;
            s.win_rate /= n;
            s.pnl_r_moyen /= n;
        }
    }

    // Ne retourner que les créneaux ayant eu au moins un trade
    stats.into_iter().filter(|s| s.nb_trades > 0).collect()
}

/// Calcule les statistiques par jour de semaine (0=Lundi … 6=Dimanche).
pub fn calculer_stats_par_jour(trades: &[TradeBacktest]) -> Vec<crate::StatJour> {
    let noms = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];
    let mut stats: Vec<crate::StatJour> = (0u8..7)
        .map(|j| crate::StatJour {
            jour: j,
            nom: noms[j as usize].to_string(),
            nb_trades: 0,
            win_rate: 0.0,
            pnl_r_moyen: 0.0,
        })
        .collect();

    use chrono::Datelike;
    for trade in trades {
        let dow = trade.ouvert_a.weekday().num_days_from_monday() as usize;
        let s = &mut stats[dow];
        s.nb_trades += 1;
        let gagnant = matches!(
            trade.resultat,
            crate::ResultatTrade::Tp1 | crate::ResultatTrade::Tp2 | crate::ResultatTrade::Tp3
        );
        s.win_rate += if gagnant { 1.0 } else { 0.0 };
        s.pnl_r_moyen += trade.pnl_r;
    }
    for s in &mut stats {
        if s.nb_trades > 0 {
            let n = s.nb_trades as f64;
            s.win_rate /= n;
            s.pnl_r_moyen /= n;
        }
    }
    stats.into_iter().filter(|s| s.nb_trades > 0).collect()
}

/// Retourne le minimum de la série de capital.
pub fn calculer_capital_min(capital_serie: &[f64], capital_initial: f64) -> f64 {
    capital_serie.iter().copied().fold(capital_initial, f64::min)
}

/// Calcule la plus longue série de gains et de pertes consécutifs.
/// Retourne (serie_gains_max, serie_pertes_max).
pub fn calculer_series_max(trades: &[TradeBacktest]) -> (usize, usize) {
    let mut gains_max = 0usize;
    let mut pertes_max = 0usize;
    let mut gains_en_cours = 0usize;
    let mut pertes_en_cours = 0usize;
    for t in trades {
        let gagnant = matches!(
            t.resultat,
            crate::ResultatTrade::Tp1 | crate::ResultatTrade::Tp2 | crate::ResultatTrade::Tp3
        );
        if gagnant {
            gains_en_cours += 1;
            pertes_en_cours = 0;
        } else {
            pertes_en_cours += 1;
            gains_en_cours = 0;
        }
        gains_max = gains_max.max(gains_en_cours);
        pertes_max = pertes_max.max(pertes_en_cours);
    }
    (gains_max, pertes_max)
}

/// Calcule la performance annualisée à partir du capital initial/final
/// et du nombre de jours de la période.
pub fn calculer_perf_annualisee(capital_initial: f64, capital_final: f64, nb_jours: u32) -> f64 {
    if capital_initial <= 0.0 || nb_jours == 0 {
        return 0.0;
    }
    let ratio = capital_final / capital_initial;
    let facteur = 365.0 / nb_jours as f64;
    ratio.powf(facteur) - 1.0
}

/// Identifie les fenêtres horaires propices pour le Straddle :
/// créneaux avec ≥2 trades, win_rate ≥ 50%, pnl_r_moyen > 0.
/// Enrichit avec l'événement macro typique si connu.
pub fn identifier_fenetres_propices(trades: &[TradeBacktest]) -> Vec<crate::FenetrePropice> {
    let stats_heure = calculer_stats_par_heure(trades);
    let mut fenetres: Vec<crate::FenetrePropice> = stats_heure
        .iter()
        .filter(|s| s.nb_trades >= 2 && s.win_rate >= 0.50 && s.pnl_r_moyen > 0.0)
        .map(|s| crate::FenetrePropice {
            heure: s.heure,
            jour_semaine: None,
            nb_trades: s.nb_trades,
            win_rate: s.win_rate,
            pnl_r_total: s.pnl_r_moyen * s.nb_trades as f64,
            evenement_type: evenement_macro_typique(s.heure),
        })
        .collect();

    // Trier par P&L total décroissant
    fenetres.sort_by(|a, b| b.pnl_r_total.partial_cmp(&a.pnl_r_total).unwrap_or(std::cmp::Ordering::Equal));
    fenetres
}/// Retourne l'événement macro typiquement publié à cette heure UTC.
fn evenement_macro_typique(heure: u8) -> Option<String> {
    match heure {
        7 => Some("Ouverture Londres / PMI Europe".to_string()),
        8 => Some("PIB / Inflation zone euro".to_string()),
        12 => Some("CPI / PPI US · Ventes détail US".to_string()),
        13 => Some("NFP (1er vendredi) · Chômage US".to_string()),
        14 => Some("Indice ISM / Confiance consommateurs".to_string()),
        18 | 19 => Some("Décision taux FOMC / Conférence Fed".to_string()),
        _ => None,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Direction, ResultatTrade, TradeBacktest};
    use chrono::Utc;

    fn trade(pnl_r: f64, resultat: ResultatTrade) -> TradeBacktest {
        TradeBacktest {
            ouvert_a: Utc::now(),
            ferme_a: Some(Utc::now()),
            direction: Direction::Long,
            prix_entree: 100.0,
            prix_sortie: Some(101.0),
            stop_loss: 99.0,
            take_profit_1: 102.0,
            take_profit_2: None,
            take_profit_3: None,
            resultat,
            pnl_r,
            pnl_usd: pnl_r * 10.0,
            heure_ouverture: 10,
            categorie: String::new(),
        }
    }

    #[test]
    fn win_rate_3_gagnants_sur_5() {
        let trades = vec![
            trade(1.0, ResultatTrade::Tp1),
            trade(1.0, ResultatTrade::Tp1),
            trade(1.0, ResultatTrade::Tp2),
            trade(-1.0, ResultatTrade::StopLoss),
            trade(-1.0, ResultatTrade::StopLoss),
        ];
        let wr = calculer_win_rate(&trades);
        assert!((wr - 0.6).abs() < 1e-9, "win_rate={wr}");
    }

    #[test]
    fn win_rate_vide_est_zero() {
        assert_eq!(calculer_win_rate(&[]), 0.0);
    }

    #[test]
    fn profit_factor_gains_double_pertes() {
        let trades = vec![
            trade(2.0, ResultatTrade::Tp1),
            trade(2.0, ResultatTrade::Tp1),
            trade(-1.0, ResultatTrade::StopLoss),
            trade(-1.0, ResultatTrade::StopLoss),
        ];
        let pf = calculer_profit_factor(&trades);
        assert!((pf - 2.0).abs() < 1e-9, "profit_factor={pf}");
    }

    #[test]
    fn drawdown_max_serie_descendante() {
        let serie = vec![100.0, 90.0, 80.0, 85.0, 75.0];
        let dd = calculer_drawdown_max(&serie);
        // pic=100, creux=75, dd=25%
        assert!((dd - 0.25).abs() < 1e-9, "drawdown={dd}");
    }

    #[test]
    fn sharpe_serie_constante_est_zero() {
        let serie = vec![1.0, 1.0, 1.0, 1.0];
        assert_eq!(calculer_sharpe(&serie), 0.0);
    }

    #[test]
    fn sharpe_serie_positive_est_positif() {
        let serie = vec![1.0, 2.0, 1.5, 2.0, 1.8];
        let s = calculer_sharpe(&serie);
        assert!(s > 0.0, "sharpe={s}");
    }
}
