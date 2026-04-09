//! Helpers IG Markets — mapping Asset → epic et Timeframe → resolution.
//! Utilisés par api::ig_provider (DataProvider) et api::prix_utils (prix spot).

use common::{Asset, Timeframe};

/// Retourne l'epic IG pour un asset non-crypto.
pub fn epic_pour_asset(asset: &Asset) -> Option<&'static str> {
    match asset {
        // Métaux précieux
        Asset::XAUUSD => Some("CS.D.CFDGOLD.CFDGC.IP"),
        Asset::XAGUSD => Some("CS.D.CFDSILVER.CFDSI.IP"),
        Asset::XPTUSD => Some("CS.D.PLATINUM.CFD.IP"),
        Asset::XPDUSD => Some("CS.D.PALLADIUM.CFD.IP"),
        // Paires Forex
        Asset::EURUSD => Some("CS.D.EURUSD.CFD.IP"),
        Asset::GBPUSD => Some("CS.D.GBPUSD.CFD.IP"),
        Asset::USDJPY => Some("CS.D.USDJPY.CFD.IP"),
        Asset::USDCHF => Some("CS.D.USDCHF.CFD.IP"),
        Asset::AUDUSD => Some("CS.D.AUDUSD.CFD.IP"),
        Asset::USDCAD => Some("CS.D.USDCAD.CFD.IP"),
        Asset::NZDUSD => Some("CS.D.NZDUSD.CFD.IP"),
        Asset::GBPJPY => Some("CS.D.GBPJPY.CFD.IP"),
        Asset::CADJPY => Some("CS.D.CADJPY.CFD.IP"),
        Asset::NZDJPY => Some("CS.D.NZDJPY.CFD.IP"),
        Asset::EURJPY => Some("CS.D.EURJPY.CFD.IP"),
        Asset::EURGBP => Some("CS.D.EURGBP.CFD.IP"),
        // Indices (IFD = daily rolling futures CFD)
        Asset::DAX    => Some("IX.D.DAX.IFD.IP"),
        Asset::NAS100 => Some("IX.D.NASDAQ.IFD.IP"),
        Asset::SP500  => Some("IX.D.SPTRD.IFD.IP"),
        Asset::US30   => Some("IX.D.DOW.IFD.IP"),
        Asset::FTSE100 => Some("IX.D.FTSE.IFD.IP"),
        Asset::CAC40  => Some("IX.D.CAC.IFD.IP"),
        Asset::JP225  => Some("IX.D.NIKKEI.IFD.IP"),
        // Crypto : géré par Binance
        _ => None,
    }
}

/// Convertit un Timeframe en paramètre `resolution` IG.
pub fn resolution_pour_tf(tf: &Timeframe) -> &'static str {
    match tf {
        Timeframe::M1  => "MINUTE",
        Timeframe::M5  => "MINUTE_5",
        Timeframe::M15 => "MINUTE_15",
        Timeframe::M30 => "MINUTE_30",
        Timeframe::H1  => "HOUR",
        Timeframe::H4  => "HOUR_4",
        Timeframe::D1  => "DAY",
        Timeframe::W1  => "WEEK",
    }
}

