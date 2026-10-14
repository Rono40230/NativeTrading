//! Structures d'analyse des performances ML par stratégie.
//! Ce module ne fait aucun accès DB — il reçoit les données de la couche API.
//! (Écart 25, 13/10 : les tranches par score/kill-zone/confiance/phase et les
//! helpers associés ont été purgés — ils lisaient les tables feedback mortes.)
use serde::{Deserialize, Serialize};

// ── Structures de données ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsGlobales {
    pub nb_trades: i64,
    pub nb_gagnants: i64,
    pub win_rate: f64, // 0.0-100.0
    pub pnl_r_moyen: f64,
}

/// Analyse globale de la stratégie SMC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmcAnalyse {
    pub global: StatsGlobales,
}

/// Analyse globale rockets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketsAnalyse {
    pub global: StatsGlobales,
}

/// Analyse globale straddle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StraddleAnalyse {
    pub global: StatsGlobales,
}

/// Vue consolidée de toutes les stratégies — réponse de /api/ml/feedback/stats.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyseGlobale {
    pub smc: Option<SmcAnalyse>,
    pub rockets: Option<RocketsAnalyse>,
    pub straddle: Option<StraddleAnalyse>,
}
