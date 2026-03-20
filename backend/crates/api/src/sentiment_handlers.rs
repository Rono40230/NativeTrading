use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Serialize, Clone)]
pub struct EntiteSentiment {
    pub nom: String,
    pub prix: f64,
    pub variation_pct: f64,
}

#[derive(Serialize)]
pub struct SentimentMarche {
    pub date: String,
    pub usa: Vec<EntiteSentiment>,
    pub europe: Vec<EntiteSentiment>,
    pub matieres_premieres: Vec<EntiteSentiment>,
    pub cryptos: Vec<EntiteSentiment>,
    pub vix: Option<f64>,
}

// ── Désérialisation Yahoo Finance v8 ────────────────────────────────────────

#[derive(Deserialize)]
struct YahooMeta {
    #[serde(rename = "regularMarketPrice")]
    prix: Option<f64>,
    #[serde(rename = "chartPreviousClose")]
    cloture_precedente: Option<f64>,
}

#[derive(Deserialize)]
struct YahooResultItem {
    meta: YahooMeta,
}

#[derive(Deserialize)]
struct YahooChartResult {
    result: Option<Vec<YahooResultItem>>,
}

#[derive(Deserialize)]
struct YahooResponse {
    chart: YahooChartResult,
}

// ── Désérialisation Binance ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct BinanceTicker {
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "priceChangePercent")]
    price_change_percent: String,
}

// ── Fonctions fetch ──────────────────────────────────────────────────────────

async fn fetch_yahoo(
    client: &reqwest::Client,
    symbole: &str,
    nom: &str,
) -> Option<EntiteSentiment> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=2d",
        symbole
    );
    let resp: YahooResponse = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    let meta = resp.chart.result?.into_iter().next()?.meta;
    let prix = meta.prix?;
    let prev = meta.cloture_precedente.unwrap_or(prix);
    let variation = if prev != 0.0 {
        (prix - prev) / prev * 100.0
    } else {
        0.0
    };

    Some(EntiteSentiment {
        nom: nom.to_string(),
        prix,
        variation_pct: (variation * 100.0).round() / 100.0,
    })
}

async fn fetch_binance(
    client: &reqwest::Client,
    symbole: &str,
    nom: &str,
) -> Option<EntiteSentiment> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/24hr?symbol={}",
        symbole
    );
    let resp: BinanceTicker = client
        .get(&url)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    let prix = resp.last_price.parse::<f64>().ok()?;
    let variation = resp.price_change_percent.parse::<f64>().ok()?;

    Some(EntiteSentiment {
        nom: nom.to_string(),
        prix,
        variation_pct: (variation * 100.0).round() / 100.0,
    })
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/sentiment/marche
/// Retourne les prix + variation % des principaux marchés mondiaux.
/// Sources : Yahoo Finance (indices, matières premières), Binance (crypto).
/// Timeout 10s par source — dégradation silencieuse si une source échoue.
pub async fn get_sentiment_marche(_state: web::Data<AppState>) -> impl Responder {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client HTTP sentiment: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    // Toutes les requêtes en parallèle — dégradation silencieuse par source
    let (sp500, nasdaq, dji, n100, dax, cac, or_, petrole, agri, btc, eth, vix_raw) =
        tokio::join!(
            fetch_yahoo(&client, "%5EGSPC", "S&P500"),
            fetch_yahoo(&client, "%5EIXIC", "Nasdaq"),
            fetch_yahoo(&client, "%5EDJI", "Dow Jones"),
            fetch_yahoo(&client, "%5EN100", "Euronext 100"),
            fetch_yahoo(&client, "%5EGDAXI", "Dax"),
            fetch_yahoo(&client, "%5EFCHI", "Cac 40"),
            fetch_yahoo(&client, "GC%3DF", "Or"),
            fetch_yahoo(&client, "CL%3DF", "Pétrole"),
            fetch_yahoo(&client, "ZC%3DF", "Agriculture"),
            fetch_binance(&client, "BTCUSDT", "Bitcoin"),
            fetch_binance(&client, "ETHUSDT", "Ethereum"),
            fetch_yahoo(&client, "%5EVIX", "VIX"),
        );

    let sentiment = SentimentMarche {
        date: Utc::now().format("%Y-%m-%d").to_string(),
        usa: [sp500, nasdaq, dji].into_iter().flatten().collect(),
        europe: [n100, dax, cac].into_iter().flatten().collect(),
        matieres_premieres: [or_, petrole, agri].into_iter().flatten().collect(),
        cryptos: [btc, eth].into_iter().flatten().collect(),
        vix: vix_raw.map(|v| (v.prix * 10.0).round() / 10.0),
    };

    HttpResponse::Ok().json(sentiment)
}
