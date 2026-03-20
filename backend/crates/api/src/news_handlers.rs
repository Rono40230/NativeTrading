use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use futures_util::future;
use serde::{Deserialize, Serialize};

use crate::news_rss::fetch_rss;
use crate::news_scraper::{est_url_externe_sure, recuperer_contenu_article};
use crate::news_traduction::{traduire_avec_cache, traduire_contenu};
use crate::state::AppState;

// ── Types de sortie ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ArticleNews {
    pub id: String,
    pub titre: String,
    pub titre_fr: Option<String>,
    pub source: String,
    pub url: String,
    pub date: String,
    pub score: u8,
    pub niveau: &'static str,
}

#[derive(Serialize)]
pub struct AlertesNews {
    pub articles: Vec<ArticleNews>,
    pub score_max: u8,
    pub mis_a_jour: String,
}

// ── Sources RSS publiques ────────────────────────────────────────────────────

/// (url, nom_source, score_base 0-40)
const SOURCES: &[(&str, &str, u8)] = &[
    (
        "https://feeds.reuters.com/reuters/businessNews",
        "Reuters Business",
        40,
    ),
    (
        "https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=10000664",
        "CNBC Markets",
        35,
    ),
    (
        "https://feeds.marketwatch.com/marketwatch/marketpulse/",
        "MarketWatch",
        35,
    ),
    ("https://cointelegraph.com/rss", "CoinTelegraph", 30),
    (
        "https://finance.yahoo.com/news/rssindex",
        "Yahoo Finance",
        28,
    ),
];

// ── Mots-clés avec leur poids ────────────────────────────────────────────────

const MOTS_MACRO: &[(&str, u8)] = &[
    ("federal reserve", 15),
    ("rate hike", 15),
    ("rate cut", 15),
    ("nfp", 14),
    ("cpi", 14),
    ("inflation", 12),
    ("payroll", 12),
    ("recession", 13),
    ("debt ceiling", 13),
    ("crisis", 12),
    ("crash", 12),
    ("circuit breaker", 14),
    ("gdp", 10),
    ("ecb", 10),
    ("fed ", 10),
    ("bce", 8),
];

const MOTS_MARCHES: &[(&str, u8)] = &[
    ("bitcoin", 10),
    ("btc", 10),
    ("selloff", 10),
    ("sell-off", 10),
    ("liquidation", 10),
    ("crypto", 7),
    ("sec ", 8),
    ("halving", 9),
    ("stablecoin", 7),
    ("etf", 6),
    ("earnings", 6),
    ("volatility", 6),
];

// ── Scoring et classification ────────────────────────────────────────────────

fn scorer(titre_lower: &str, base: u8) -> u8 {
    let mut bonus: u8 = 0;
    for (mot, poids) in MOTS_MACRO {
        if titre_lower.contains(mot) {
            bonus = bonus.saturating_add(*poids);
        }
    }
    for (mot, poids) in MOTS_MARCHES {
        if titre_lower.contains(mot) {
            bonus = bonus.saturating_add(*poids);
        }
    }
    base.saturating_add(bonus).min(100)
}

fn niveau(score: u8) -> &'static str {
    match score {
        80..=100 => "critique",
        60..=79 => "important",
        40..=59 => "modere",
        _ => "veille",
    }
}

/// Hash DJB2 du titre pour un ID stable et déterministe.
fn hash_titre(titre: &str) -> String {
    let mut h: u64 = 5381;
    for b in titre.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    format!("{h:016x}")
}

fn normaliser_date(s: &str) -> String {
    chrono::DateTime::parse_from_rfc2822(s)
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|_| Utc::now().to_rfc3339())
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/news/alertes
/// Agrège 5 flux RSS publics, score chaque titre par mots-clés,
/// retourne les 20 articles les mieux classés.
/// Dégradation silencieuse par source — timeout global 10s.
pub async fn get_news_alertes(state: web::Data<AppState>) -> impl Responder {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client HTTP news: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    // Fetch toutes les sources en parallèle via tokio::spawn
    let taches: Vec<_> = SOURCES
        .iter()
        .map(|(url, nom, base)| {
            let c = client.clone();
            let url = *url;
            let nom = *nom;
            let base = *base;
            tokio::spawn(async move {
                let rss = fetch_rss(&c, url).await;
                (rss, nom, base)
            })
        })
        .collect();

    let resultats = future::join_all(taches).await;

    let mut articles: Vec<ArticleNews> = resultats
        .into_iter()
        .filter_map(|r| r.ok())
        .flat_map(|(rss_articles, nom, base)| {
            rss_articles.into_iter().map(move |a| {
                let titre_lower = a.titre.to_lowercase();
                let score = scorer(&titre_lower, base);
                ArticleNews {
                    id: hash_titre(&a.titre),
                    titre: a.titre,
                    titre_fr: None,
                    source: nom.to_string(),
                    url: a.lien,
                    date: normaliser_date(&a.date_rss),
                    score,
                    niveau: niveau(score),
                }
            })
        })
        .collect();

    articles.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    articles.truncate(20);

    // Traduction des titres via cache SQLite + Ollama (en arrière-plan)
    let pool = state.db.pool();
    for article in &mut articles {
        article.titre_fr = Some(traduire_avec_cache(pool, &article.titre).await);
    }

    let score_max = articles.first().map(|a| a.score).unwrap_or(0);

    HttpResponse::Ok().json(AlertesNews {
        articles,
        score_max,
        mis_a_jour: Utc::now().to_rfc3339(),
    })
}

// ── Contenu Article ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ContenuParams {
    pub url: String,
}

#[derive(Serialize)]
pub struct ContenuArticle {
    pub texte: String,
}

/// GET /api/news/contenu?url=...
/// Scrape et retourne le texte lisible d'un article externe.
/// Protection SSRF : HTTPS uniquement, adresses internes bloquées.
pub async fn get_contenu_article(
    _state: web::Data<AppState>,
    params: web::Query<ContenuParams>,
) -> impl Responder {
    if !est_url_externe_sure(&params.url) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "URL non autorisée" }));
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client HTTP contenu article: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Client HTTP indisponible" }));
        }
    };

    match recuperer_contenu_article(&client, &params.url).await {
        Some(texte) => HttpResponse::Ok().json(ContenuArticle { texte }),
        None => HttpResponse::UnprocessableEntity()
            .json(serde_json::json!({ "error": "Contenu non extractible" })),
    }
}

// ── Traduction à la demande ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TraductionParams {
    pub texte: String,
    #[serde(default)]
    pub long: bool,
}

#[derive(Serialize)]
pub struct TraductionReponse {
    pub texte_fr: String,
}

/// GET /api/news/traduire?texte=...&long=true
/// Traduit un texte via Ollama. long=true pour les corps d'articles.
/// Dégradation silencieuse : retourne le texte original si Ollama est absent.
pub async fn get_traduire(
    state: web::Data<AppState>,
    params: web::Query<TraductionParams>,
) -> impl Responder {
    let texte_fr = if params.long {
        traduire_contenu(&params.texte).await
    } else {
        traduire_avec_cache(state.db.pool(), &params.texte).await
    };
    HttpResponse::Ok().json(TraductionReponse { texte_fr })
}
