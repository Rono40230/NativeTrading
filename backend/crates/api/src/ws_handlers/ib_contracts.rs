//! Helpers contrats IB Gateway — métaux, Forex, indices

use ibapi::contracts::{Contract, SecurityType};
use ibapi::market_data::historical::{BarSize, WhatToShow};

/// Contrat IB pour données historiques (chart).
/// Métaux → Commodity SMART | Forex → ForexPair IDEALPRO | Indices → CFD SMART
pub(super) fn ib_contrat_hist(asset: &common::Asset) -> Contract {
    match asset {
        // ── Métaux précieux ──────────────────────────────────────────────
        common::Asset::XAUUSD => Contract {
            symbol: "XAUUSD".into(),
            security_type: SecurityType::Commodity,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        common::Asset::XAGUSD => Contract {
            symbol: "XAGUSD".into(),
            security_type: SecurityType::Commodity,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        // ── Paires Forex (ForexPair IDEALPRO) ────────────────────────────
        common::Asset::EURUSD => ib_forex_pair("EUR", "USD"),
        common::Asset::GBPJPY => ib_forex_pair("GBP", "JPY"),
        common::Asset::CADJPY => ib_forex_pair("CAD", "JPY"),
        common::Asset::NZDJPY => ib_forex_pair("NZD", "JPY"),
        common::Asset::USDCAD => ib_forex_pair("USD", "CAD"),
        common::Asset::USDJPY => ib_forex_pair("USD", "JPY"),
        // ── Indices / CFD ─────────────────────────────────────────────────
        common::Asset::DAX => Contract {
            symbol: "DAX".into(),
            security_type: SecurityType::Index,
            exchange: "EUREX".into(),
            currency: "EUR".into(),
            ..Default::default()
        },
        common::Asset::NAS100 => Contract {
            symbol: "NQ".into(),
            security_type: SecurityType::ContinuousFuture,
            exchange: "CME".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        common::Asset::SP500 => Contract {
            symbol: "SPX".into(),
            security_type: SecurityType::Index,
            exchange: "CBOE".into(),
            currency: "USD".into(),
            ..Default::default()
        },
        // BTC/ETH ne passent pas par IB — cas impossible en production
        common::Asset::BTC | common::Asset::ETH => Contract {
            symbol: asset.as_str().into(),
            security_type: SecurityType::CFD,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        },
    }
}

/// Contrat pour tick-by-tick bid/ask.
/// Métaux → ForexPair IDEALPRO | Forex → même ForexPair | Indices → None
pub(super) fn ib_contrat_tick(asset: &common::Asset) -> Option<Contract> {
    match asset {
        common::Asset::XAUUSD => Some(ib_forex_pair("XAU", "USD")),
        common::Asset::XAGUSD => Some(ib_forex_pair("XAG", "USD")),
        common::Asset::EURUSD => Some(ib_forex_pair("EUR", "USD")),
        common::Asset::GBPJPY => Some(ib_forex_pair("GBP", "JPY")),
        common::Asset::CADJPY => Some(ib_forex_pair("CAD", "JPY")),
        common::Asset::NZDJPY => Some(ib_forex_pair("NZD", "JPY")),
        common::Asset::USDCAD => Some(ib_forex_pair("USD", "CAD")),
        common::Asset::USDJPY => Some(ib_forex_pair("USD", "JPY")),
        _ => None,
    }
}

/// Helper : construit un contrat ForexPair IDEALPRO
fn ib_forex_pair(symbole: &str, devise: &str) -> Contract {
    Contract {
        symbol: symbole.into(),
        security_type: SecurityType::ForexPair,
        exchange: "IDEALPRO".into(),
        currency: devise.into(),
        ..Default::default()
    }
}

/// WhatToShow selon le type d'asset.
/// Indices / Futures continus → Trades | Forex, métaux → MidPoint
pub(super) fn what_to_show_hist(asset: &common::Asset) -> WhatToShow {
    match asset {
        common::Asset::DAX | common::Asset::NAS100 | common::Asset::SP500 => WhatToShow::Trades,
        _ => WhatToShow::MidPoint,
    }
}

pub(super) fn ib_bar_size(tf: &common::Timeframe) -> BarSize {
    match tf {
        common::Timeframe::M1  => BarSize::Min,
        common::Timeframe::M5  => BarSize::Min5,
        common::Timeframe::M15 => BarSize::Min15,
        common::Timeframe::M30 => BarSize::Min30,
        common::Timeframe::H1  => BarSize::Hour,
        common::Timeframe::H4  => BarSize::Hour4,
        common::Timeframe::D1  => BarSize::Day,
        common::Timeframe::W1  => BarSize::Week,
    }
}
