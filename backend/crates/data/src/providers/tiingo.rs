//! Provider Tiingo — prix D1 actions US (étape A Rockets actions, 31/08).
//! Clé gratuite (1 000 requêtes/jour), volume RÉEL de l'échange (fini le
//! proxy tick volume MT5). Endpoint EOD :
//!   GET https://api.tiingo.com/tiingo/daily/{ticker}/prices?token=KEY&startDate=YYYY-MM-DD
//! Les prix utilisés sont BRUTS (`close`, pas `adjClose`) : les niveaux
//! techniques (pivot, stop) doivent être ceux vus sur les graphiques.

use common::{Result, TradingError};

/// Bougie D1 actions (ts secondes UTC).
#[derive(Debug, Clone, PartialEq)]
pub struct BougieEod {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

pub struct TiingoProvider {
    cle: String,
    client: reqwest::Client,
}

impl TiingoProvider {
    pub fn nouveau(cle: impl Into<String>) -> Self {
        Self {
            cle: cle.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// EOD d'un ticker depuis une date ISO (YYYY-MM-DD). Retour vide si
    /// ticker inconnu (réponse JSON vide) — pas une erreur.
    pub async fn eod(&self, ticker: &str, depuis: &str) -> Result<Vec<BougieEod>> {
        let url = format!(
            "https://api.tiingo.com/tiingo/daily/{}/prices?token={}&startDate={}&sort=date",
            ticker, self.cle, depuis
        );
        let rep = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| TradingError::Api(e.to_string()))?;
        if !rep.status().is_success() {
            return Err(TradingError::Api(format!(
                "Tiingo HTTP {} pour {}",
                rep.status(),
                ticker
            )));
        }
        let corps = rep
            .text()
            .await
            .map_err(|e| TradingError::Api(e.to_string()))?;
        parser_eod(&corps)
    }
}

/// Parse la réponse JSON Tiingo en bougies. Pur (testable sans réseau).
/// Format : [{"date":"2026-08-28T00:00:00.000Z","close":230.1,...,"volume":41234567}, ...]
pub fn parser_eod(corps: &str) -> Result<Vec<BougieEod>> {
    let brut: Vec<serde_json::Value> =
        serde_json::from_str(corps).map_err(|e| TradingError::Api(e.to_string()))?;

    let mut out = Vec::with_capacity(brut.len());
    for b in brut {
        let date = b
            .get("date")
            .and_then(|d| d.as_str())
            .ok_or_else(|| TradingError::Api("Tiingo: date absente".into()))?;
        // "2026-08-28T00:00:00.000Z" → 2026-08-28 → epoch secondes
        let jour = &date[..10.min(date.len())];
        let ts = date_vers_epoch(jour)?;
        let f = |champ: &str| -> Result<f64> {
            b.get(champ)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| TradingError::Api(format!("Tiingo: champ {champ} absent")))
        };
        out.push(BougieEod {
            ts,
            open: f("open")?,
            high: f("high")?,
            low: f("low")?,
            close: f("close")?,
            volume: b.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0),
        });
    }
    Ok(out)
}

/// "YYYY-MM-DD" → secondes UTC (midi pour éviter les ambiguïtés de fuseau).
fn date_vers_epoch(jour: &str) -> Result<i64> {
    use chrono::TimeZone;
    let naive = chrono::NaiveDate::parse_from_str(jour, "%Y-%m-%d")
        .map_err(|e| TradingError::Api(format!("Tiingo: date invalide {jour}: {e}")))?;
    let dt = chrono::Utc
        .from_utc_datetime(&naive.and_hms_opt(12, 0, 0).unwrap_or_default());
    Ok(dt.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"[
        {"date":"2026-08-28T00:00:00.000Z","open":229.5,"high":231.2,"low":228.9,"close":230.1,"adjClose":228.3,"volume":41234567,"symbol":"AAPL"},
        {"date":"2026-08-29T00:00:00.000Z","open":230.2,"high":232.0,"low":229.8,"close":231.7,"adjClose":229.9,"volume":38111222}
    ]"#;

    #[test]
    fn parse_deux_bougies_avec_volume_reel() {
        let v = parser_eod(FIXTURE).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].close, 230.1);
        assert_eq!(v[0].volume, 41234567.0);
        assert_eq!(v[1].high, 232.0);
    }

    #[test]
    fn parse_utilise_close_brut_pas_adjclose() {
        let v = parser_eod(FIXTURE).unwrap();
        assert_eq!(v[0].close, 230.1, "close brut attendu, pas adjClose 228.3");
    }

    #[test]
    fn date_convertie_en_epoch_utc() {
        let v = parser_eod(FIXTURE).unwrap();
        // 2026-08-28 12:00 UTC
        assert_eq!(v[0].ts, 1787918400);
    }

    #[test]
    fn reponse_vide_renvoie_vecteur_vide() {
        assert!(parser_eod("[]").unwrap().is_empty());
    }

    #[test]
    fn json_invalide_est_une_erreur() {
        assert!(parser_eod("<html>bloqué</html>").is_err());
    }
}
