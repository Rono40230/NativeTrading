use serde::{Deserialize, Serialize};

// ── Requêtes / réponses ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteAnalyse {
    pub asset: String,
    pub periode: Option<String>, // "3m" | "6m" | "1a" | "2a"
}

#[derive(Deserialize)]
pub struct MaJCreneau {
    pub statut: Option<String>,
    pub backtest_winrate: Option<f64>,
    pub backtest_profit_factor: Option<f64>,
}

#[derive(Serialize)]
pub(crate) struct ReponseAnalyse {
    pub creneaux: Vec<db::straddle::StraddleCreneau>,
    pub nb_analyses: usize,
    pub nb_retenus: usize,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn periode_en_mois(p: Option<&str>) -> u32 {
    match p {
        Some("3m") => 3,
        Some("1a") => 12,
        Some("2a") => 24,
        _ => 6,
    }
}

/// Nombre de bougies H1 demandées en DB (sans plafond — le cache peut être grand).
pub fn limite_bougies(mois: u32) -> usize {
    mois as usize * 30 * 24
}

/// Plafond pour les providers réseau (Binance : max 1000 bougies par appel).
pub const MAX_BOUGIES_RESEAU: usize = 1000;
