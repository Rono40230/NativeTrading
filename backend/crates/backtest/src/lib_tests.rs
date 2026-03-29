use super::*;
use chrono::Utc;
use common::Candle;

fn bougie(close: f64) -> Candle {
    Candle {
        timestamp: Utc::now(),
        open: close,
        high: close * 1.01,
        low: close * 0.99,
        close,
        volume: 1000.0,
    }
}

struct StrategieVide;
impl strategies::Strategy for StrategieVide {
    fn analyze(&self, _: &[Candle]) -> common::Result<Option<strategies::Signal>> {
        Ok(None)
    }
}

#[test]
fn backtest_sans_trades_retourne_capital_initial() {
    let bougies: Vec<Candle> = (1..=70).map(|i| bougie(i as f64 * 100.0)).collect();
    let engine = BacktestEngine::new(2000.0);
    let resultats = engine.run(&bougies, &StrategieVide).unwrap();
    assert_eq!(resultats.total_trades, 0);
    assert!((resultats.capital_final - 2000.0).abs() < 1e-10);
    assert!((resultats.roi_pct).abs() < 1e-10);
}

#[test]
fn backtest_peu_de_bougies_retourne_vide() {
    let bougies: Vec<Candle> = (1..=10).map(|i| bougie(i as f64 * 100.0)).collect();
    let engine = BacktestEngine::new(2000.0);
    let resultats = engine.run(&bougies, &StrategieVide).unwrap();
    assert_eq!(resultats.total_trades, 0);
}
