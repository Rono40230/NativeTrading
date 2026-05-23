//! Tests unitaires du moteur de recommandations.
use super::*;
use crate::{BacktestConfig, BacktestResult, StrategieType, TradeBacktest};
use chrono::Utc;
use common::{Asset, Direction, Timeframe};

fn config_straddle() -> BacktestConfig {
    BacktestConfig {
        asset: Asset::XAUUSD,
        timeframe: Timeframe::M15,
        debut: Utc::now(),
        fin: Utc::now(),
        strategie: StrategieType::Straddle,
        capital_initial: 10_000.0,
        risque_par_trade: 0.02,
    }
}

fn resultat_avec_double_sl(taux: f64) -> BacktestResult {
    let nb = 20usize;
    let nb_dsl = (nb as f64 * taux) as usize;
    let trades: Vec<TradeBacktest> = (0..nb)
        .map(|i| TradeBacktest {
            ouvert_a: Utc::now(),
            ferme_a: Some(Utc::now()),
            direction: Direction::Both,
            prix_entree: 100.0,
            prix_sortie: None,
            stop_loss: 98.0,
            take_profit_1: 104.0,
            take_profit_2: None,
            take_profit_3: None,
            resultat: if i < nb_dsl {
                crate::ResultatTrade::StopLoss
            } else {
                crate::ResultatTrade::Tp1
            },
            pnl_r: if i < nb_dsl { -2.0 } else { 1.0 },
            pnl_usd: if i < nb_dsl { -200.0 } else { 100.0 },
            heure_ouverture: 14,
            categorie: if i < nb_dsl {
                "double_sl".to_string()
            } else {
                "tp1_sl2".to_string()
            },
        })
        .collect();

    BacktestResult {
        config: config_straddle(),
        nb_trades: nb,
        win_rate: 1.0 - taux,
        profit_factor: if taux > 0.5 { 0.5 } else { 1.5 },
        sharpe: 0.8,
        drawdown_max: 0.1,
        capital_final: 9_500.0,
        pnl_total_r: -2.0,
        double_sl_rate: Some(taux),
        double_win_rate: Some(0.1),
        stats_par_heure: vec![],
        trades,
    }
}

#[test]
fn double_sl_eleve_genere_recommandation_priorite_1() {
    let result = resultat_avec_double_sl(0.35);
    let recs = analyser_recommandations(&result);
    assert!(!recs.is_empty(), "Aucune recommandation générée");
    assert_eq!(recs[0].priorite, 1, "Priorité attendue = 1");
    assert!(
        recs[0].titre.contains("double SL") || recs[0].param_cible.contains("sl"),
        "Recommandation inattendue : {}",
        recs[0].titre
    );
}

#[test]
fn resultat_parfait_liste_vide() {
    let result = BacktestResult {
        config: config_straddle(),
        nb_trades: 30,
        win_rate: 0.70,
        profit_factor: 2.5,
        sharpe: 2.1,
        drawdown_max: 0.05,
        capital_final: 12_000.0,
        pnl_total_r: 15.0,
        double_sl_rate: Some(0.05),
        double_win_rate: Some(0.45),
        stats_par_heure: vec![],
        trades: vec![],
    };
    let recs = analyser_recommandations(&result);
    assert!(
        recs.is_empty(),
        "Des recommandations inutiles ont été générées : {:?}",
        recs.iter().map(|r| &r.titre).collect::<Vec<_>>()
    );
}

#[test]
fn drawdown_eleve_genere_recommandation_risque() {
    let mut result = resultat_avec_double_sl(0.10);
    result.drawdown_max = 0.22;
    result.double_sl_rate = Some(0.10);
    let recs = analyser_recommandations(&result);
    let rec_dd = recs.iter().find(|r| r.param_cible == "risque_par_trade");
    assert!(rec_dd.is_some(), "Recommandation drawdown manquante");
    assert_eq!(rec_dd.unwrap().priorite, 1);
}
