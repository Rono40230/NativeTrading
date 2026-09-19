//! Bandeau sentiment du dashboard (15/09) — les vraies données de sentiment
//! au-dessus du tableau de cours :
//! - Peur & Appêt crypto : alternative.me (gratuit, sans clé), cache 6 h ;
//! - Positioning futures : Bybit v5 PUBLIC (ratio long/short comptes +
//!   funding des perp BTC/ETH), cache 15 min ;
//! - Breadth maison : part des actifs au-dessus de leur MM50 (D1 en base —
//!   seuls les univers avec ≥ 50 bougies D1 comptent, forex exclus jusqu'à
//!   l'historique EA) ;
//! - Bias presse IA : répartition haussier/neutre/baissier des 48 dernières
//!   heures (table news_sentiment — pas de nouvel appel LLM).
//! Dégradation silencieuse par source : un fetch raté laisse son champ vide.

use std::sync::Arc;
use std::time::{Duration, Instant};

use db::Database;

/// Univers du breadth maison (asset ids de la table assets). La garde
/// « ≥ 50 bougies D1 » exclut d'elle-même les actifs sans historique.
const UNIVERS_BREADTH: &[(&str, &[&str])] = &[
    ("Crypto", &["BTC", "ETH", "SOL", "ADA", "AVAX", "BNB", "DOGE", "DOT", "LINK", "LTC", "XRP"]),
    ("Métaux", &["XAUUSD", "XAGUSD", "XPTUSD"]),
    ("Indices", &["DAX", "NAS100", "SP500"]),
    ("Forex", &["EURUSD", "GBPUSD", "USDJPY", "AUDUSD", "USDCAD", "NZDUSD", "EURJPY", "GBPJPY", "NZDJPY"]),
];

/// Péripéréries du positioning (perp Bybit linear, tout public).
const PERPS: &[(&str, &str)] = &[("BTC", "BTCUSDT"), ("ETH", "ETHUSDT")];

const CACHE_FNG: Duration = Duration::from_secs(6 * 3600);
const CACHE_POSITIONING: Duration = Duration::from_secs(15 * 60);

#[derive(serde::Serialize, Clone)]
pub struct Fng {
    pub valeur: i32,
    pub classe: String,
    /// Variation vs la veille (points d'indice).
    pub delta_veille: i32,
}

#[derive(serde::Serialize, Clone)]
pub struct Positioning {
    pub asset: String,
    /// Part des comptes longs (0-1).
    pub ratio_long: f64,
    /// Part des comptes courts (0-1).
    pub ratio_short: f64,
    /// ratio_long / ratio_short (1,0 = équilibre).
    pub ls: f64,
    /// Funding annuel-reporté au taux courant, en % (négatif = shorts paient).
    pub funding_pct: f64,
}

#[derive(serde::Serialize, Clone)]
pub struct Breadth {
    pub univers: String,
    pub au_dessus: i32,
    pub total: i32,
}

#[derive(serde::Serialize, Clone)]
pub struct PresseBias {
    pub haussier: i32,
    pub neutre: i32,
    pub baissier: i32,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct BandeauSentiment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fng: Option<Fng>,
    pub positioning: Vec<Positioning>,
    pub breadth: Vec<Breadth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presse: Option<PresseBias>,
    /// Unix (s) de l'assemblage — l'âge du fetch est visible côté front.
    pub maj_le: i64,
}

async fn client_http() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()
}

// ── Peur & Appêt (alternative.me) ────────────────────────────────────────────

static CACHE_FNG_MEM: std::sync::OnceLock<tokio::sync::Mutex<Option<(Instant, Fng)>>> =
    std::sync::OnceLock::new();

async fn fear_greed() -> Option<Fng> {
    let cache = CACHE_FNG_MEM.get_or_init(|| tokio::sync::Mutex::new(None));
    if let Some((t, fng)) = cache.lock().await.as_ref() {
        if t.elapsed() < CACHE_FNG {
            return Some(fng.clone());
        }
    }
    #[derive(serde::Deserialize)]
    struct Reponse {
        data: Vec<EntreeFng>,
    }
    #[derive(serde::Deserialize)]
    struct EntreeFng {
        value: String,
        value_classification: String,
    }
    let raw: Reponse = client_http()
        .await?
        .get("https://api.alternative.me/fng/?limit=2")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let premier = raw.data.first()?;
    let valeur = premier.value.parse::<i32>().ok()?;
    let veille = raw.data.get(1).and_then(|e| e.value.parse::<i32>().ok());
    let fng = Fng {
        valeur,
        classe: premier.value_classification.clone(),
        delta_veille: veille.map(|v| valeur - v).unwrap_or(0),
    };
    *cache.lock().await = Some((Instant::now(), fng.clone()));
    Some(fng)
}

// ── Positioning Bybit (public) ───────────────────────────────────────────────

static CACHE_POS_MEM: std::sync::OnceLock<tokio::sync::Mutex<Option<(Instant, Vec<Positioning>)>>> =
    std::sync::OnceLock::new();

async fn ratio_comptes(client: &reqwest::Client, perp: &str) -> Option<(f64, f64)> {
    #[derive(serde::Deserialize)]
    struct Reponse {
        result: Resultat,
    }
    #[derive(serde::Deserialize)]
    struct Resultat {
        list: Vec<Ligne>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Ligne {
        buy_ratio: String,
        sell_ratio: String,
    }
    let r: Reponse = client
        .get(format!(
            "https://api.bybit.com/v5/market/account-ratio?category=linear&symbol={perp}&period=1d&limit=1"
        ))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let l = r.result.list.into_iter().next()?;
    Some((l.buy_ratio.parse().ok()?, l.sell_ratio.parse().ok()?))
}

async fn funding(client: &reqwest::Client, perp: &str) -> Option<f64> {
    #[derive(serde::Deserialize)]
    struct Reponse {
        result: Resultat,
    }
    #[derive(serde::Deserialize)]
    struct Resultat {
        list: Vec<Ligne>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Ligne {
        funding_rate: String,
    }
    let r: Reponse = client
        .get(format!(
            "https://api.bybit.com/v5/market/tickers?category=linear&symbol={perp}"
        ))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let taux: f64 = r.result.list.into_iter().next()?.funding_rate.parse().ok()?;
    Some(taux * 100.0)
}

async fn positioning() -> Vec<Positioning> {
    let cache = CACHE_POS_MEM.get_or_init(|| tokio::sync::Mutex::new(None));
    if let Some((t, pos)) = cache.lock().await.as_ref() {
        if t.elapsed() < CACHE_POSITIONING {
            return pos.clone();
        }
    }
    let Some(client) = client_http().await else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (asset, perp) in PERPS {
        let Some((long, short)) = ratio_comptes(&client, perp).await else {
            continue;
        };
        let ls = if short > 0.0 { long / short } else { 0.0 };
        out.push(Positioning {
            asset: (*asset).to_string(),
            ratio_long: long,
            ratio_short: short,
            ls,
            funding_pct: funding(&client, perp).await.unwrap_or(0.0),
        });
    }
    if !out.is_empty() {
        *cache.lock().await = Some((Instant::now(), out.clone()));
    }
    out
}

// ── Breadth maison (MM50, D1 en base) ───────────────────────────────────────

async fn breadth(db: &Arc<Database>) -> Vec<Breadth> {
    let mut out = Vec::new();
    for (univers, assets) in UNIVERS_BREADTH {
        let mut au_dessus = 0i32;
        let mut total = 0i32;
        for id in *assets {
            let asset = common::Asset::from(*id);
            let Ok(bougies) = db.obtenir_bougies(&asset, &common::Timeframe::D1, 50).await else {
                continue;
            };
            if bougies.len() < 50 {
                continue;
            }
            let mm50: f64 = bougies.iter().map(|b| b.close).sum::<f64>() / bougies.len() as f64;
            let dernier = bougies.last().map(|b| b.close).unwrap_or(0.0);
            total += 1;
            if dernier > mm50 {
                au_dessus += 1;
            }
        }
        if total > 0 {
            out.push(Breadth {
                univers: (*univers).to_string(),
                au_dessus,
                total,
            });
        }
    }
    out
}

// ── Bias presse (notations LLM existantes, 48 h) ────────────────────────────

async fn presse_bias(db: &Arc<Database>) -> Option<PresseBias> {
    let limite = chrono::Utc::now().timestamp() - 48 * 3600;
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT sentiment, COUNT(*) FROM news_sentiment WHERE analyse_le >= ? GROUP BY sentiment",
    )
    .bind(limite)
    .fetch_all(db.pool())
    .await
    .ok()?;
    let mut b = PresseBias { haussier: 0, neutre: 0, baissier: 0 };
    for (sentiment, n) in lignes {
        match sentiment.as_str() {
            "haussier" => b.haussier = n as i32,
            "neutre" => b.neutre = n as i32,
            "baissier" => b.baissier = n as i32,
            _ => {}
        }
    }
    if b.haussier + b.neutre + b.baissier > 0 {
        Some(b)
    } else {
        None
    }
}

/// Assemble le bandeau complet (appelé par GET /api/sentiment).
pub async fn collecter(db: &Arc<Database>) -> BandeauSentiment {
    BandeauSentiment {
        fng: fear_greed().await,
        positioning: positioning().await,
        breadth: breadth(db).await,
        presse: presse_bias(db).await,
        maj_le: chrono::Utc::now().timestamp(),
    }
}
