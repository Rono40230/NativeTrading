//! Parsing de la réponse IG `GET /prices/{epic}` → `Vec<Candle>`.
//!
//! Format renvoyé par IG (prix bid/ask, le worker stocke le mid) :
//!
//! ```json
//! { "prices": [ { "snapshotTime": "2026/08/14 05:00:00",
//!     "openPrice": {"bid": 1.0900, "ask": 1.0902}, … } ] }
//! ```
//!
//! Sont écartées : les bougies sans timestamp valide ou sans OHLC complet
//! (IG renvoie `null` quand le marché est fermé), et celles pas encore
//! fermées — sinon INSERT OR IGNORE figerait une bougie partielle à jamais.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use common::{Candle, Timeframe};

/// Tolérance d'horloge pour le filtre « bougie fermée » (skew hôte/IG).
const TOLERANCE_HORLOGE_SEC: i64 = 60;

/// Prix bid/ask IG — le worker stocke le mid `(bid + ask) / 2`.
#[derive(serde::Deserialize)]
pub(super) struct IgPrix {
    bid: Option<f64>,
    ask: Option<f64>,
}

impl IgPrix {
    fn mid(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(b), Some(a)) => Some((b + a) / 2.0),
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            _ => None,
        }
    }
}

/// Une entrée du tableau `prices` de la réponse IG.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct IgBougie {
    snapshot_time: Option<String>,
    /// Champ alternatif ISO 8601 renvoyé par certaines versions de l'API.
    #[serde(rename = "snapshotTimeUTC")]
    snapshot_time_utc: Option<String>,
    open_price: Option<IgPrix>,
    high_price: Option<IgPrix>,
    low_price: Option<IgPrix>,
    close_price: Option<IgPrix>,
    last_traded_volume: Option<f64>,
}

/// Réponse complète `GET /prices/{epic}`.
#[derive(serde::Deserialize)]
pub(super) struct IgReponse {
    prices: Option<Vec<IgBougie>>,
}

/// Parse un timestamp IG en Unix secondes (UTC). Formats acceptés :
///
/// - `"2026/08/14 05:00:00"` (`snapshotTime` — format IG classique)
/// - `"2026/08/14 05:00:00.123"` (idem avec millisecondes)
/// - `"2026-08-14T05:00:00Z"` (`snapshotTimeUTC` — ISO 8601)
pub(super) fn parse_ts_ig(brut: &str) -> Option<i64> {
    if let Ok(ndt) = NaiveDateTime::parse_from_str(brut, "%Y/%m/%d %H:%M:%S") {
        return Some(ndt.and_utc().timestamp());
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(brut, "%Y/%m/%d %H:%M:%S%.f") {
        return Some(ndt.and_utc().timestamp());
    }
    DateTime::parse_from_rfc3339(brut)
        .ok()
        .map(|dt| dt.timestamp())
}

/// Convertit une réponse IG en `Vec<Candle>` (prix mid). Une bougie dont la
/// clôture théorique (`ts + durée du timeframe`) est dans le futur est en
/// cours de formation → écartée : on ne stocke que des bougies définitives.
pub(super) fn convertir_bougies(reponse: &IgReponse, tf: &Timeframe) -> Vec<Candle> {
    let limite = Utc::now().timestamp() + TOLERANCE_HORLOGE_SEC;
    reponse
        .prices
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|b| {
            let brut = b.snapshot_time_utc.as_deref().or(b.snapshot_time.as_deref())?;
            let ts = parse_ts_ig(brut)?;
            if ts + tf.minutes() as i64 * 60 > limite {
                return None; // bougie encore en formation
            }
            Some(Candle {
                timestamp: Utc.timestamp_opt(ts, 0).single()?,
                open: b.open_price.as_ref()?.mid()?,
                high: b.high_price.as_ref()?.mid()?,
                low: b.low_price.as_ref()?.mid()?,
                close: b.close_price.as_ref()?.mid()?,
                volume: b.last_traded_volume.unwrap_or(0.0),
            })
        })
        .collect()
}

// ─── Tests unitaires (pas de réseau) ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamp_ig() {
        // Format spécifié par l'API : "2026/08/14 05:00:00" (UTC).
        assert_eq!(parse_ts_ig("2026/08/14 05:00:00"), Some(1_786_683_600));
        assert_eq!(parse_ts_ig("2021/06/29 08:00:00.500"), Some(1_624_953_600));
        assert_eq!(parse_ts_ig("2026-08-14T05:00:00Z"), Some(1_786_683_600));
        assert_eq!(parse_ts_ig("nawak"), None); // jamais de panic
        assert_eq!(parse_ts_ig(""), None);
        assert_eq!(parse_ts_ig("2026/13/45 99:00:00"), None);
    }

    /// Fabrique un JSON IG d'une bougie avec `snapshotTime` paramétrable.
    fn json_ig(ts: &str) -> String {
        format!(
            r#"{{"prices": [{{"snapshotTime": "{ts}",
                "openPrice": {{"bid": 1.0900, "ask": 1.0902}},
                "highPrice": {{"bid": 1.0910, "ask": 1.0912}},
                "lowPrice": {{"bid": 1.0895, "ask": 1.0897}},
                "closePrice": {{"bid": 1.0905, "ask": 1.0907}},
                "lastTradedVolume": 123.0}}]}}"#
        )
    }

    /// Formate un Unix timestamp comme IG : "2026/08/14 05:00:00".
    fn ts_format_ig(ts: i64) -> String {
        DateTime::from_timestamp(ts, 0)
            .expect("timestamp valide")
            .naive_utc()
            .format("%Y/%m/%d %H:%M:%S")
            .to_string()
    }

    #[test]
    fn convertir_reponse_ig_prix_mid() {
        // Bougie fermée depuis longtemps → conservée, prix mid.
        let reponse: IgReponse =
            serde_json::from_str(&json_ig("2021/06/29 08:00:00")).expect("JSON IG valide");
        let b = &convertir_bougies(&reponse, &Timeframe::M5)[0];
        assert_eq!(b.timestamp.timestamp(), 1_624_953_600);
        assert!((b.open - 1.0901).abs() < 1e-9, "mid open");
        assert!((b.high - 1.0911).abs() < 1e-9, "mid high");
        assert!((b.low - 1.0896).abs() < 1e-9, "mid low");
        assert!((b.close - 1.0906).abs() < 1e-9, "mid close");
        assert!((b.volume - 123.0).abs() < 1e-9);
    }

    #[test]
    fn convertir_reponse_ecarte_bougie_en_formation() {
        // M5 ouverte il y a 1 min → ferme dans ~4 min → écartée.
        let ts = ts_format_ig(Utc::now().timestamp() - 60);
        let reponse: IgReponse = serde_json::from_str(&json_ig(&ts)).expect("JSON IG valide");
        assert!(convertir_bougies(&reponse, &Timeframe::M5).is_empty());
        // M5 ouverte il y a 10 min → fermée depuis 5 min → conservée.
        let ts = ts_format_ig(Utc::now().timestamp() - 600);
        let reponse: IgReponse = serde_json::from_str(&json_ig(&ts)).expect("JSON IG valide");
        assert_eq!(convertir_bougies(&reponse, &Timeframe::M5).len(), 1);
    }

    #[test]
    fn convertir_reponse_ecarte_les_bougies_incompletes() {
        let json = r#"{"prices": [
            {"snapshotTime": "invalide",
             "openPrice": {"bid": 1.0, "ask": 1.0}, "highPrice": {"bid": 1.0, "ask": 1.0},
             "lowPrice": {"bid": 1.0, "ask": 1.0}, "closePrice": {"bid": 1.0, "ask": 1.0}},
            {"snapshotTime": "2021/06/29 08:00:00", "openPrice": null,
             "highPrice": {"bid": 1.0, "ask": 1.0}, "lowPrice": {"bid": 1.0, "ask": 1.0},
             "closePrice": {"bid": 1.0, "ask": 1.0}}]}"#;
        let reponse: IgReponse = serde_json::from_str(json).expect("JSON IG valide");
        assert!(convertir_bougies(&reponse, &Timeframe::M5).is_empty());
        // `prices` absent → vide, pas d'erreur.
        let vide: IgReponse = serde_json::from_str("{}").expect("objet vide valide");
        assert!(convertir_bougies(&vide, &Timeframe::M5).is_empty());
    }
}
