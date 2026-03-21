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

impl Asset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Asset::BTC    => "BTC",
            Asset::ETH    => "ETH",
            Asset::SOL    => "SOL",
            Asset::BNB    => "BNB",
            Asset::XRP    => "XRP",
            Asset::ADA    => "ADA",
            Asset::DOGE   => "DOGE",
            Asset::AVAX   => "AVAX",
            Asset::LINK   => "LINK",
            Asset::DOT    => "DOT",
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
            Asset::DAX    => "DAX",
            Asset::NAS100 => "NAS100",
            Asset::SP500  => "SP500",
            Asset::US30   => "US30",
            Asset::FTSE100 => "FTSE100",
            Asset::CAC40  => "CAC40",
            Asset::JP225  => "JP225",
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
