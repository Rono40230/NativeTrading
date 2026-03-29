use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: i64,
    pub capital: f64,
}

/// Résultats complets d'un backtest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResults {
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f64,
    pub capital_initial: f64,
    pub capital_final: f64,
    pub roi_pct: f64,
    pub profit_net: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown_pct: f64,
    pub profit_factor: f64,
    /// Trades avec sortie partielle à TP1 (SL → BE, position réduite)
    pub nb_tp1: u32,
    /// Trades avec sortie partielle à TP2 (SL → TP1, position réduite)
    pub nb_tp2: u32,
    /// Trades stoppés (SL ou trailing après TP2)
    pub nb_sl: u32,
    /// Trades fermés à l'expiration de l'horizon (ni TP ni SL atteints)
    pub nb_expirations: u32,
    /// Nombre de Straddles posés (= total_trades / 2 car Long+Short par signal)
    pub nb_straddles: u32,
    pub equity_curve: Vec<EquityPoint>,
}

/// Données de feedback d'un trade simulé pour raffinement du pipeline ML.
/// Contient l'index de la bougie d'entrée et le résultat (gagné/perdu).
pub struct FeedbackTrade {
    pub indice_entree: usize,
    pub gagne: bool,
}
