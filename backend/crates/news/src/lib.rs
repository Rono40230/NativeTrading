//! Crate `news` — logique de veille presse (RSS, scraping, scoring, traduction/sentiment LLM).
//! Extrait du monolithe api (phase 1.6c). Dépend de `llm` pour la traduction/sentiment.
pub mod news_rss;
pub mod news_scraper;
pub mod news_scoring;
pub mod news_traduction;
pub mod presse_classif;
