//! Cours US en direct via Yahoo Finance (décision propriétaire 06/09 —
//! même source que le Journal de Trading, testée et approuvée ; Finnhub
//! écarté après soucis).
//!
//! Pattern éprouvé du journal (post-verrouillage 2023 de l'API v7) :
//! 1. client reqwest dédié avec User-Agent navigateur + cookie_store,
//! 2. préchauffe du cookie sur fc.yahoo.com,
//! 3. crumb via /v1/test/getcrumb (lié au cookie — le même client sert
//!    les deux, crumb mis en cache 30 min),
//! 4. GET /v7/finance/quote?symbols=…&crumb=… → prix, high/low du jour,
//!    variation du jour.
//! Hors session US, regularMarketPrice est la dernière clôture (ou le
//! post-market) — le « live » devient la dernière valeur connue.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const TTL_CRUMB: Duration = Duration::from_secs(30 * 60);

fn client() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(UA)
            .cookie_store(true)
            .build()
            .unwrap_or_default()
    })
}

static CRUMB: OnceLock<RwLock<Option<(Instant, String)>>> = OnceLock::new();
fn crumb_slot() -> &'static RwLock<Option<(Instant, String)>> {
    CRUMB.get_or_init(|| RwLock::new(None))
}

/// Crumb valide (cache 30 min) — chaîne vide si Yahoo le refuse (le journal
/// tente alors la quote sans crumb, on fait pareil).
async fn crumb() -> String {
    if let Some((t, c)) = crumb_slot().read().await.as_ref() {
        if t.elapsed() < TTL_CRUMB {
            return c.clone();
        }
    }
    // Préchauffe du cookie puis crumb — le même client garde les deux liés.
    let _ = client().get("https://fc.yahoo.com").send().await;
    let valeur = match client()
        .get("https://query1.finance.yahoo.com/v1/test/getcrumb")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        _ => String::new(),
    };
    *crumb_slot().write().await = Some((Instant::now(), valeur.clone()));
    valeur
}

/// Un cours Yahoo US : dernier prix (session ou post-market), variation du
/// jour en %, plus haut/bas DU JOUR (la « bougie en cours » pour la
/// gestion 30 s des positions actions).
#[derive(Debug, Clone, Copy)]
pub struct QuoteYahoo {
    pub prix: f64,
    pub variation_jour_pct: f64,
    pub haut_jour: f64,
    pub bas_jour: f64,
}

/// Cours d'un lot de symboles US. Ceux en échec sont absents du résultat.
pub async fn quotes(symboles: &[String]) -> HashMap<String, QuoteYahoo> {
    let mut out = HashMap::new();
    if symboles.is_empty() {
        return out;
    }
    let liste = symboles.join(",");
    let crumb = crumb().await;
    let url = format!(
        "https://query1.finance.yahoo.com/v7/finance/quote?symbols={liste}&crumb={crumb}"
    );
    let Ok(rep) = client().get(&url).send().await else {
        return out;
    };
    if !rep.status().is_success() {
        tracing::warn!("⚠️ Yahoo quote HTTP {} (crumb {} car.)", rep.status(), crumb.len());
        return out;
    }
    let Ok(json) = rep.json::<serde_json::Value>().await else {
        return out;
    };
    if let Some(resultats) = json["quoteResponse"]["result"].as_array() {
        for q in resultats {
            let Some(symbole) = q["symbol"].as_str() else { continue };
            // Session → regularMarketPrice, sinon post-market (journal idem).
            let Some(prix) = q["regularMarketPrice"]
                .as_f64()
                .or_else(|| q["postMarketPrice"].as_f64())
            else {
                continue;
            };
            out.insert(
                symbole.to_string(),
                QuoteYahoo {
                    prix,
                    variation_jour_pct: q["regularMarketChangePercent"].as_f64().unwrap_or(0.0),
                    haut_jour: q["regularMarketDayHigh"].as_f64().unwrap_or(prix),
                    bas_jour: q["regularMarketDayLow"].as_f64().unwrap_or(prix),
                },
            );
        }
    }
    out
}
