use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use futures_util::future;
use serde::{Deserialize, Serialize};

use news::news_rss::fetch_rss;
use news::news_scoring::{classer_theme, dedupliquer, niveau, scorer, AlertesNews, ArticleNews};
use news::news_scraper::{est_url_externe_sure, recuperer_contenu_article};
use news::news_traduction::{
    hash_titre, lire_cache, lire_sentiment_cache, traduire_avec_cache, traduire_contenu,
};
use crate::state::AppState;

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
    ("https://cryptonews.com/news/feed/", "CryptoNews", 28),
    ("https://decrypt.co/feed", "Decrypt", 30),
    ("https://www.fxstreet.com/rss/news", "FXStreet", 38),
    (
        "https://www.kitco.com/news/rss/metals-news.xml",
        "Kitco",
        38,
    ),
];

// ── Utilitaire interne ───────────────────────────────────────────────────────

fn normaliser_date(s: &str) -> String {
    chrono::DateTime::parse_from_rfc2822(s)
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|_| Utc::now().to_rfc3339())
}

// ── Handler principal ────────────────────────────────────────────────────────

/// GET /api/news/alertes
/// Agrège les flux RSS, score chaque titre par mots-clés,
/// retourne les 30 articles les mieux classés.
pub async fn get_news_alertes(state: web::Data<AppState>) -> impl Responder {
    let client = &*crate::http_client::HTTP_CLIENT;

    let taches: Vec<_> = SOURCES
        .iter()
        .map(|(url, nom, base)| {
            let url = *url;
            let nom = *nom;
            let base = *base;
            tokio::spawn(async move {
                let rss = fetch_rss(client, url).await;
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
                let date = normaliser_date(&a.date_rss);
                let score = scorer(&titre_lower, base, &date);
                ArticleNews {
                    id: hash_titre(&a.titre),
                    titre: a.titre,
                    titre_fr: None,
                    source: nom.to_string(),
                    url: a.lien,
                    date,
                    score,
                    niveau: niveau(score),
                    theme: classer_theme(&titre_lower, nom),
                    sentiment: None,
                }
            })
        })
        .collect();

    articles.sort_unstable_by_key(|b| std::cmp::Reverse(b.score));
    let articles = dedupliquer(articles);
    let mut articles = articles;
    articles.truncate(30);

    let pool = state.db.pool();
    let mut titres_a_traduire: Vec<String> = Vec::new();
    let mut titres_a_analyser: Vec<String> = Vec::new();
    for article in &mut articles {
        let h = hash_titre(&article.titre);
        match lire_cache(pool, &h).await {
            Some(t) => article.titre_fr = Some(t),
            None => titres_a_traduire.push(article.titre.clone()),
        }
        match lire_sentiment_cache(pool, &h).await {
            Some(s) => article.sentiment = Some(s),
            None => titres_a_analyser.push(article.titre.clone()),
        }
    }

    if !titres_a_traduire.is_empty() {
        let pool_bg = pool.clone();
        tokio::spawn(async move {
            for titre in titres_a_traduire {
                traduire_avec_cache(&pool_bg, &titre).await;
            }
        });
    }

    if !titres_a_analyser.is_empty() {
        let pool_bg = pool.clone();
        tokio::spawn(async move {
            for titre in titres_a_analyser {
                news::news_traduction::analyser_sentiment_avec_cache(&pool_bg, &titre).await;
            }
        });
    }

    let score_max = articles.first().map(|a| a.score).unwrap_or(0);
    state
        .signal_engine
        .mettre_a_jour_score_news(score_max as i32);

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
/// Protection SSRF : HTTPS uniquement, adresses internes bloquées.
pub async fn get_contenu_article(
    _state: web::Data<AppState>,
    params: web::Query<ContenuParams>,
) -> impl Responder {
    if !est_url_externe_sure(&params.url) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "URL non autorisée" }));
    }

    let client = &*crate::http_client::HTTP_CLIENT;

    match recuperer_contenu_article(client, &params.url).await {
        Some(texte) => HttpResponse::Ok().json(ContenuArticle { texte }),
        None => HttpResponse::UnprocessableEntity()
            .json(serde_json::json!({ "error": "Contenu non extractible" })),
    }
}

// ── Traduction à la demande ──────────────────────────────────────────────────

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
