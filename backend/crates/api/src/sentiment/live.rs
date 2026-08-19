//! Collecteurs live du sentiment marché (Yahoo séance + Bitcoin DB).
//!
//! Extraits de `sentiment_handlers` (limite 600 lignes du pré-audit).

use serde::Deserialize;

use crate::sentiment_handlers::EntiteSentiment;
use crate::sentiment_handlers::{minuit_utc_ms, Database};

/// Prix + variation de SÉANCE (live) depuis Yahoo (`regularMarketPrice` vs
/// `chartPreviousClose`). Dégradation silencieuse par source.
pub(crate) async fn yahoo_live(client: &reqwest::Client, symbole: &str, nom: &str) -> Option<EntiteSentiment> {
    #[derive(serde::Deserialize)]
    struct Meta {
        #[serde(rename = "regularMarketPrice")]
        prix: Option<f64>,
        #[serde(rename = "chartPreviousClose")]
        precedente: Option<f64>,
    }
    #[derive(serde::Deserialize)]
    struct Reponse {
        chart: ChartResult,
    }
    #[derive(serde::Deserialize)]
    struct ChartResult {
        result: Option<Vec<MetaWrap>>,
    }
    #[derive(serde::Deserialize)]
    struct MetaWrap {
        meta: Meta,
    }
    let url = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=2d",
        symbole
    );
    let r: Reponse = client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let meta = r.chart.result?.into_iter().next()?.meta;
    let prix = meta.prix?;
    let precedente = meta.precedente?;
    let variation = if precedente != 0.0 {
        (prix - precedente) / precedente * 100.0
    } else {
        0.0
    };
    Some(EntiteSentiment {
        nom: nom.to_string(),
        prix,
        variation_pct: (variation * 100.0).round() / 100.0,
        variation_veille: None,
    })
}

