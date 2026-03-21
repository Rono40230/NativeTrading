use common::{Asset, Timeframe};

/// Parse un nom d'asset case-insensitive. Retourne None si non supporté.
pub fn parse_asset(s: &str) -> Option<Asset> {
    match s.to_uppercase().as_str() {
        // Crypto (Binance)
        "BTC"  => Some(Asset::BTC),
        "ETH"  => Some(Asset::ETH),
        "SOL"  => Some(Asset::SOL),
        "BNB"  => Some(Asset::BNB),
        "XRP"  => Some(Asset::XRP),
        "ADA"  => Some(Asset::ADA),
        "DOGE" => Some(Asset::DOGE),
        "AVAX" => Some(Asset::AVAX),
        "LINK" => Some(Asset::LINK),
        "DOT"  => Some(Asset::DOT),
        // Métaux
        "XAUUSD" | "XAU" | "GOLD"   => Some(Asset::XAUUSD),
        "XAGUSD" | "XAG" | "SILVER" => Some(Asset::XAGUSD),
        "XPTUSD" | "XPT" | "PLATINUM"  => Some(Asset::XPTUSD),
        "XPDUSD" | "XPD" | "PALLADIUM" => Some(Asset::XPDUSD),
        // Forex
        "EURUSD" => Some(Asset::EURUSD),
        "GBPUSD" => Some(Asset::GBPUSD),
        "USDJPY" => Some(Asset::USDJPY),
        "USDCHF" => Some(Asset::USDCHF),
        "AUDUSD" => Some(Asset::AUDUSD),
        "USDCAD" => Some(Asset::USDCAD),
        "NZDUSD" => Some(Asset::NZDUSD),
        "GBPJPY" => Some(Asset::GBPJPY),
        "CADJPY" => Some(Asset::CADJPY),
        "NZDJPY" => Some(Asset::NZDJPY),
        "EURJPY" => Some(Asset::EURJPY),
        "EURGBP" => Some(Asset::EURGBP),
        // Indices
        "DAX"  | "DAX40"   | "GER40"    => Some(Asset::DAX),
        "NAS100" | "NDX"   | "NASDAQ100" => Some(Asset::NAS100),
        "SP500" | "SPX"    | "SPX500"   => Some(Asset::SP500),
        "US30" | "DJ30"    | "DOW"      => Some(Asset::US30),
        "FTSE100" | "FTSE" | "UK100"    => Some(Asset::FTSE100),
        "CAC40" | "CAC"    | "FRA40"    => Some(Asset::CAC40),
        "JP225" | "NIKKEI" | "JPN225"   => Some(Asset::JP225),
        _ => None,
    }
}

/// Parse un timeframe avec M15 comme valeur par défaut.
pub fn parse_timeframe(s: &str) -> Timeframe {
    match s {
        "M1" => Timeframe::M1,
        "M5" => Timeframe::M5,
        "M15" => Timeframe::M15,
        "M30" => Timeframe::M30,
        "H1" => Timeframe::H1,
        "H4" => Timeframe::H4,
        "D1" => Timeframe::D1,
        "W1" => Timeframe::W1,
        _ => Timeframe::M15,
    }
}
