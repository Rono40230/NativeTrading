//! Crate `backtest` — Moteur de replay historique et recommandations post-backtest.
//!
//! # Modules
//! - `engine`         : replay bougie par bougie, dispatche vers l'adapter stratégie
//! - `metriques`      : Sharpe, drawdown, win rate, profit factor
//! - `straddle`       : adapter Straddle (double jambe LONG+SHORT)
//! - `smc`            : adapter SMC Directionnel
//! - `rockets`        : adapter Rockets VCP
//! - `recommandations`: analyse les métriques et produit des conseils actionnables

pub mod engine;
pub mod metriques;
pub mod recommandations;
pub mod rockets;
pub mod simulateur;
pub mod smc;
pub mod straddle;

use chrono::{DateTime, Utc};
use common::{Asset, Direction, Timeframe};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Types de configuration ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/StrategieType.ts")]
pub enum StrategieType {
    Straddle,
    Smc,
    Rockets,
}

/// Configuration d'un backtest — passée à `engine::rejouer()`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/BacktestConfig.ts")]
pub struct BacktestConfig {
    pub asset: Asset,
    pub timeframe: Timeframe,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub strategie: StrategieType,
    /// Capital initial en USD
    pub capital_initial: f64,
    /// Risque par trade en % du capital (ex: 0.02 = 2%)
    pub risque_par_trade: f64,
    /// Paramètres de stratégie lus depuis la DB (fidélité live)
    #[ts(skip)]
    pub params: StrategieParams,
}

/// Paramètres injectés depuis la DB pour que le backtest soit fidèle au live.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/StrategieParams.ts")]
pub enum StrategieParams {
    Straddle(ParamsStraddle),
    Smc(ParamsSmc),
    Rockets,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/ParamsStraddle.ts")]
pub struct ParamsStraddle {
    pub atr_periode:    usize,
    pub atr_seuil:      f64,
    pub tp_mult_1:      f64,
    pub tp_mult_2:      f64,
    pub tp_mult_3:      f64,
    pub sl_mult:        f64,
    pub trailing_atr:   f64,
    pub vente_partielle:  bool,
    pub pct_cloture_tp1:  f64,
    pub pct_cloture_tp2:  f64,
}

impl Default for ParamsStraddle {
    fn default() -> Self {
        Self {
            atr_periode: 14, atr_seuil: 1.5,
            tp_mult_1: 1.5, tp_mult_2: 2.5, tp_mult_3: 5.0,
            sl_mult: 0.5, trailing_atr: 1.5,
            vente_partielle: true, pct_cloture_tp1: 0.33, pct_cloture_tp2: 0.33,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/ParamsSmc.ts")]
pub struct ParamsSmc {
    pub atr_periode:       usize,
    pub score_min:         f64,
    pub atr_tp1:           f64,
    pub atr_tp2:           f64,
    pub atr_tp3:           f64,
    pub atr_sl:            f64,
    pub vente_partielle:   bool,
    pub kill_zone_filtre:  bool,
    pub pct_cloture_tp1:   f64,
    pub pct_cloture_tp2:   f64,
}

impl Default for ParamsSmc {
    fn default() -> Self {
        Self {
            atr_periode: 14, score_min: 70.0,
            atr_tp1: 2.0, atr_tp2: 3.0, atr_tp3: 5.0, atr_sl: 1.0,
            vente_partielle: true, kill_zone_filtre: true,
            pct_cloture_tp1: 0.33, pct_cloture_tp2: 0.33,
        }
    }
}

// ── Types de résultat d'un trade rejoué ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/ResultatTrade.ts")]
pub enum ResultatTrade {
    Tp1,
    Tp2,
    Tp3,
    StopLoss,
    NonFerme,
}

/// Un trade individuel tel que simulé par le moteur de replay.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/TradeBacktest.ts")]
pub struct TradeBacktest {
    pub ouvert_a: DateTime<Utc>,
    pub ferme_a: Option<DateTime<Utc>>,
    pub direction: Direction,
    pub prix_entree: f64,
    pub prix_sortie: Option<f64>,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: Option<f64>,
    pub take_profit_3: Option<f64>,
    pub resultat: ResultatTrade,
    /// Gain/perte en multiple du risque initial (1R = risque posé)
    pub pnl_r: f64,
    /// Gain/perte en USD
    pub pnl_usd: f64,
    /// Heure UTC d'ouverture (0-23) — utilisée pour l'analyse par créneau
    pub heure_ouverture: u8,
    /// Catégorie libre (ex: "double_win", "double_sl", "tp1_sl2" pour Straddle)
    pub categorie: String,
}

// ── Statistiques par créneau horaire ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/StatHeure.ts")]
pub struct StatHeure {
    pub heure: u8,
    pub nb_trades: usize,
    pub win_rate: f64,
    pub pnl_r_moyen: f64,
}

/// Statistiques par jour de la semaine (0=Lundi … 6=Dimanche).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/StatJour.ts")]
pub struct StatJour {
    pub jour: u8,          // 0=Lundi … 6=Dimanche
    pub nom: String,       // "Lundi", "Mardi"…
    pub nb_trades: usize,
    pub win_rate: f64,
    pub pnl_r_moyen: f64,
}

/// Fenêtre horaire propice identifiée pour le Straddle
/// (combinaison heure+jour avec bon profil de volatilité).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/FenetrePropice.ts")]
pub struct FenetrePropice {
    pub heure: u8,
    pub jour_semaine: Option<u8>,  // None = tous les jours de la semaine
    pub nb_trades: usize,
    pub win_rate: f64,
    pub pnl_r_total: f64,
    /// Événement macro typiquement associé à ce créneau (heuristique)
    pub evenement_type: Option<String>,
}

// ── Résultat global du backtest ───────────────────────────────────────────

/// Résultat complet d'un backtest — retourné par `engine::rejouer()`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/BacktestResult.ts")]
pub struct BacktestResult {
    pub config: BacktestConfig,
    pub trades: Vec<TradeBacktest>,
    // Métriques globales
    pub nb_trades: usize,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub sharpe: f64,
    pub drawdown_max: f64,
    pub capital_final: f64,
    pub pnl_total_r: f64,
    /// R moyen par trade
    pub pnl_r_moyen: f64,
    /// Performance annualisée (ex: 0.32 = +32%)
    pub perf_annualisee: f64,
    /// Capital minimum atteint pendant la période
    pub capital_min: f64,
    /// Plus longue série de pertes consécutives
    pub serie_pertes_max: usize,
    /// Plus longue série de gains consécutifs
    pub serie_gains_max: usize,
    // Métriques spécifiques Straddle (None si stratégie différente)
    pub double_sl_rate: Option<f64>,
    pub double_win_rate: Option<f64>,
    // Statistiques par créneau horaire (pour détecter les heures perdantes)
    pub stats_par_heure: Vec<StatHeure>,
    // Statistiques par jour de semaine
    pub stats_par_jour: Vec<StatJour>,
    // Courbe d'équité (capital après chaque trade)
    pub equity_curve: Vec<f64>,
    // Fenêtres propices Straddle (None si autre stratégie)
    pub fenetres_propices: Option<Vec<FenetrePropice>>,
}
