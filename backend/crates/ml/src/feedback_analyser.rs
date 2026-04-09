//! Structures d'analyse des performances ML par stratégie.
//! Ce module ne fait aucun accès DB — il reçoit les données de la couche API
//! et les structure pour le générateur de suggestions (`params_suggester`).
use serde::{Deserialize, Serialize};

// ── Structures de données ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsGlobales {
    pub nb_trades:   i64,
    pub nb_gagnants: i64,
    pub win_rate:    f64, // 0.0-100.0
    pub pnl_r_moyen: f64,
}

/// Stat pour une tranche (score, session, confiance ML…)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheStat {
    pub tranche:   String,
    pub nb_trades: i64,
    pub win_rate:  f64,
}

/// Analyse complète de la stratégie SMC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmcAnalyse {
    pub global:         StatsGlobales,
    pub par_score:      Vec<TrancheStat>, // tranches "50-65", "65-75", "75-85", "85+"
    pub par_kill_zone:  Vec<TrancheStat>, // "Kill Zone" | "Hors Kill Zone"
    pub ml_correlation: Vec<TrancheStat>, // tranches confiance ML
}

/// Vue consolidée de toutes les stratégies — input du `params_suggester`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyseGlobale {
    pub smc:      Option<SmcAnalyse>,
    pub rockets:  Option<StatsGlobales>,
    pub straddle: Option<StatsGlobales>,
}

// ── Accès rapides ─────────────────────────────────────────────────────────────

impl AnalyseGlobale {
    /// Win rate SMC quand la Kill Zone est active
    pub fn smc_win_rate_kill_zone(&self) -> Option<f64> {
        self.smc
            .as_ref()?
            .par_kill_zone
            .iter()
            .find(|t| t.tranche == "Kill Zone")
            .map(|t| t.win_rate)
    }

    /// Win rate SMC quand la Kill Zone est inactive
    pub fn smc_win_rate_hors_kill_zone(&self) -> Option<f64> {
        self.smc
            .as_ref()?
            .par_kill_zone
            .iter()
            .find(|t| t.tranche == "Hors Kill Zone")
            .map(|t| t.win_rate)
    }

    /// (nb_trades, win_rate) pour une tranche de score SMC donnée  
    pub fn smc_tranche(&self, tranche: &str) -> Option<(i64, f64)> {
        self.smc
            .as_ref()?
            .par_score
            .iter()
            .find(|t| t.tranche == tranche)
            .map(|t| (t.nb_trades, t.win_rate))
    }
}
