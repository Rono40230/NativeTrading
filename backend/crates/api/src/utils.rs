use common::{Asset, Timeframe};

/// Parse un nom d'asset case-insensitive. Retourne None si non supporté.
pub fn parse_asset(s: &str) -> Option<Asset> {
    match s.to_uppercase().as_str() {
        "BTC" => Some(Asset::BTC),
        "ETH" => Some(Asset::ETH),
        "XAUUSD" | "XAU" | "GOLD" => Some(Asset::XAUUSD),
        "XAGUSD" | "XAG" | "SILVER" => Some(Asset::XAGUSD),
        _ => None,
    }
}

/// Parse un timeframe avec M15 comme valeur par défaut.
pub fn parse_timeframe(s: &str) -> Timeframe {
    match s {
        "M1" => Timeframe::M1,
        "M5" => Timeframe::M5,
        "H1" => Timeframe::H1,
        "H4" => Timeframe::H4,
        "D1" => Timeframe::D1,
        "W1" => Timeframe::W1,
        _ => Timeframe::M15,
    }
}
