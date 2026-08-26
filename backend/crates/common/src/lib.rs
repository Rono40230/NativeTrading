use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub mod time;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/Candle.ts")]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Signal de trading généré par une stratégie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: Uuid,
    pub asset: Asset,
    pub timeframe: Timeframe,
    pub direction: Direction,
    /// Score de confluence (0-100, seuil ≥70 pour SMC)
    pub score: f64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: Vec<f64>,
    pub strategie: String,
    pub cree_le: DateTime<Utc>,
}

impl Signal {
    #[allow(clippy::too_many_arguments)]
    pub fn nouveau(
        asset: Asset,
        timeframe: Timeframe,
        direction: Direction,
        score: f64,
        prix_entree: f64,
        stop_loss: f64,
        take_profit: Vec<f64>,
        strategie: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            asset,
            timeframe,
            direction,
            score,
            prix_entree,
            stop_loss,
            take_profit,
            strategie: strategie.into(),
            cree_le: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/Timeframe.ts")]
pub enum Timeframe {
    M1,
    M5,
    /// 10 minutes — VISUEL SEUL (confirmations manuelles) : aucun moteur
    /// armé, aucun stockage — agrégé depuis les M1 à la volée.
    M10,
    M15,
    M30,
    H1,
    H4,
    D1,
    W1,
}

impl Timeframe {
    pub fn as_str(&self) -> &'static str {
        match self {
            Timeframe::M1 => "M1",
            Timeframe::M5 => "M5",
            Timeframe::M10 => "M10",
            Timeframe::M15 => "M15",
            Timeframe::M30 => "M30",
            Timeframe::H1 => "H1",
            Timeframe::H4 => "H4",
            Timeframe::D1 => "D1",
            Timeframe::W1 => "W1",
        }
    }

    /// Durée d'une bougie en minutes — utilisé pour convertir un horizon temporel
    /// (ex: 30 min) en nombre de bougies selon le timeframe actif.
    pub fn minutes(&self) -> u64 {
        match self {
            Timeframe::M1 => 1,
            Timeframe::M5 => 5,
            Timeframe::M10 => 10,
            Timeframe::M15 => 15,
            Timeframe::M30 => 30,
            Timeframe::H1 => 60,
            Timeframe::H4 => 240,
            Timeframe::D1 => 1440,
            Timeframe::W1 => 10080,
        }
    }
}

impl TryFrom<&str> for Timeframe {
    type Error = TradingError;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        match s {
            "M1" => Ok(Timeframe::M1),
            "M5" => Ok(Timeframe::M5),
            "M10" => Ok(Timeframe::M10),
            "M15" => Ok(Timeframe::M15),
            "M30" => Ok(Timeframe::M30),
            "H1" => Ok(Timeframe::H1),
            "H4" => Ok(Timeframe::H4),
            "D1" => Ok(Timeframe::D1),
            "W1" => Ok(Timeframe::W1),
            other => Err(TradingError::Data(format!("Timeframe inconnu: {}", other))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/Direction.ts")]
pub enum Direction {
    Long,
    Short,
    Both,
}

/// Un asset de marché — **donnée, pas code** (décision propriétaire 2026-08-15).
///
/// N'importe quel ticker peut exister à l'exécution : la légitimité d'un
/// asset vient de la table `assets` (source, symboles, classe), jamais d'une
/// liste codée en dur. La sérialisation est la string nue (JSON/TS : `string`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/Asset.ts")]
pub struct Asset(String);

impl Asset {
    /// Construit un asset depuis un ticker (usage général, tests inclus).
    pub fn nouveau(ticker: &str) -> Self {
        Self(ticker.to_uppercase())
    }

    /// Ticker de l'asset.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Heuristique « cotable chez Bybit » (crypto + métaux USDT) — utilisée
    /// pour router vers le provider REST. La vérité reste la table `assets`
    /// (source + symbol_bybit) ; cette heuristique ne sert qu'aux chemins
    /// sans accès DB.
    pub fn est_cotable_bybit(&self) -> bool {
        let t = self.0.as_str();
        !t.contains('/')
            && !t.is_empty()
            && (t.ends_with("USDT") || {
                // Crypto (ticker court) ou métal spot (XAUUSD…)
                t.len() <= 6 && t.chars().all(|c| c.is_ascii_alphanumeric())
            })
    }
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Asset {
    fn from(s: &str) -> Self {
        Self::nouveau(s)
    }
}

impl From<String> for Asset {
    fn from(s: String) -> Self {
        Self(s.to_uppercase())
    }
}


#[derive(Debug, thiserror::Error)]
pub enum TradingError {
    #[error("Data error: {0}")]
    Data(String),
    #[error("ML error: {0}")]
    ML(String),
    #[error("Strategy error: {0}")]
    Strategy(String),
    #[error("Risk error: {0}")]
    Risk(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("API error: {0}")]
    Api(String),
}

pub type Result<T> = std::result::Result<T, TradingError>;
