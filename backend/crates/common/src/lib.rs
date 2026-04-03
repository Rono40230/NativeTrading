use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Timeframe {
    M1,
    M5,
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
            Timeframe::M15 => 15,
            Timeframe::M30 => 30,
            Timeframe::H1 => 60,
            Timeframe::H4 => 240,
            Timeframe::D1 => 1440,
            Timeframe::W1 => 10080,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Direction {
    Long,
    Short,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Asset {
    // Crypto (Binance)
    BTC,
    ETH,
    SOL,
    BNB,
    XRP,
    ADA,
    DOGE,
    AVAX,
    LINK,
    DOT,
    // Métaux précieux (IB Gateway — Commodity SMART)
    XAUUSD,
    XAGUSD,
    XPTUSD,
    XPDUSD,
    // Paires Forex (IB Gateway — ForexPair IDEALPRO)
    EURUSD,
    GBPUSD,
    USDJPY,
    USDCHF,
    AUDUSD,
    USDCAD,
    NZDUSD,
    GBPJPY,
    CADJPY,
    NZDJPY,
    EURJPY,
    EURGBP,
    // Indices / CFD (IB Gateway — CFD SMART)
    DAX,
    NAS100,
    SP500,
    US30,
    FTSE100,
    CAC40,
    JP225,
}

impl TryFrom<&str> for Asset {
    type Error = TradingError;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        match s {
            "BTC" => Ok(Asset::BTC),
            "ETH" => Ok(Asset::ETH),
            "SOL" => Ok(Asset::SOL),
            "BNB" => Ok(Asset::BNB),
            "XRP" => Ok(Asset::XRP),
            "ADA" => Ok(Asset::ADA),
            "DOGE" => Ok(Asset::DOGE),
            "AVAX" => Ok(Asset::AVAX),
            "LINK" => Ok(Asset::LINK),
            "DOT" => Ok(Asset::DOT),
            "XAUUSD" => Ok(Asset::XAUUSD),
            "XAGUSD" => Ok(Asset::XAGUSD),
            "XPTUSD" => Ok(Asset::XPTUSD),
            "XPDUSD" => Ok(Asset::XPDUSD),
            "EURUSD" => Ok(Asset::EURUSD),
            "GBPUSD" => Ok(Asset::GBPUSD),
            "USDJPY" => Ok(Asset::USDJPY),
            "USDCHF" => Ok(Asset::USDCHF),
            "AUDUSD" => Ok(Asset::AUDUSD),
            "USDCAD" => Ok(Asset::USDCAD),
            "NZDUSD" => Ok(Asset::NZDUSD),
            "GBPJPY" => Ok(Asset::GBPJPY),
            "CADJPY" => Ok(Asset::CADJPY),
            "NZDJPY" => Ok(Asset::NZDJPY),
            "EURJPY" => Ok(Asset::EURJPY),
            "EURGBP" => Ok(Asset::EURGBP),
            "DAX" => Ok(Asset::DAX),
            "NAS100" => Ok(Asset::NAS100),
            "SP500" => Ok(Asset::SP500),
            "US30" => Ok(Asset::US30),
            "FTSE100" => Ok(Asset::FTSE100),
            "CAC40" => Ok(Asset::CAC40),
            "JP225" => Ok(Asset::JP225),
            other => Err(TradingError::Data(format!("Asset inconnu: {}", other))),
        }
    }
}

impl TryFrom<&str> for Timeframe {
    type Error = TradingError;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        match s {
            "M1" => Ok(Timeframe::M1),
            "M5" => Ok(Timeframe::M5),
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

impl Asset {
    /// Retourne true si l'asset est une crypto (source Binance).
    pub fn is_crypto(&self) -> bool {
        matches!(
            self,
            Asset::BTC
                | Asset::ETH
                | Asset::SOL
                | Asset::BNB
                | Asset::XRP
                | Asset::ADA
                | Asset::DOGE
                | Asset::AVAX
                | Asset::LINK
                | Asset::DOT
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Asset::BTC => "BTC",
            Asset::ETH => "ETH",
            Asset::SOL => "SOL",
            Asset::BNB => "BNB",
            Asset::XRP => "XRP",
            Asset::ADA => "ADA",
            Asset::DOGE => "DOGE",
            Asset::AVAX => "AVAX",
            Asset::LINK => "LINK",
            Asset::DOT => "DOT",
            Asset::XAUUSD => "XAUUSD",
            Asset::XAGUSD => "XAGUSD",
            Asset::XPTUSD => "XPTUSD",
            Asset::XPDUSD => "XPDUSD",
            Asset::EURUSD => "EURUSD",
            Asset::GBPUSD => "GBPUSD",
            Asset::USDJPY => "USDJPY",
            Asset::USDCHF => "USDCHF",
            Asset::AUDUSD => "AUDUSD",
            Asset::USDCAD => "USDCAD",
            Asset::NZDUSD => "NZDUSD",
            Asset::GBPJPY => "GBPJPY",
            Asset::CADJPY => "CADJPY",
            Asset::NZDJPY => "NZDJPY",
            Asset::EURJPY => "EURJPY",
            Asset::EURGBP => "EURGBP",
            Asset::DAX => "DAX",
            Asset::NAS100 => "NAS100",
            Asset::SP500 => "SP500",
            Asset::US30 => "US30",
            Asset::FTSE100 => "FTSE100",
            Asset::CAC40 => "CAC40",
            Asset::JP225 => "JP225",
        }
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
