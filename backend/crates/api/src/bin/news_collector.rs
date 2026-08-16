//! Collecteur de presse (Phase 4.1) — process SÉPARÉ, crash-isolé (gate 4).
//!
//! Cycle 30 min : lit les sources actives en DB, fetch RSS, traite (dédup +
//! scoring + classification mots-clés — cf news::presse_classif), insère via
//! le pont `db::presse::ArticleEntrant`. AUCUNE dépendance Ollama : la
//! traduction/sentiment sont à la demande côté backend. Un cycle qui échoue
//! ou panique est loggé et sauté — le process survit.
//!
//! Usage : cargo run -p api --bin news_collector   (DATABASE_PATH requis)

use std::sync::Arc;
use std::time::Duration;

use db::Database;
use news::presse_classif::ArticleCollecte;

/// Cycle de collecte : 30 minutes (spec Phase 4.1).
const CYCLE_SEC: u64 = 1800;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Sans subscriber les tracing::* seraient muets — or ce process détaché
    // ne vit QUE par ses logs (redirigés vers data/logs/news_collector.log
    // par run.sh). Même convention que main.rs : RUST_LOG sinon "info".
    let filtre = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filtre).init();

    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(Database::new(&db_path).await?);
    db.run_migrations().await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let http = Arc::new(client);

    tracing::info!("📰 Collecteur de presse démarré (cycle {CYCLE_SEC}s)");

    loop {
        // Tâche spawnée : un panic du cycle devient un JoinError attrapé au
        // lieu d'emporter le process (« loggé et sauté », cf doc module).
        match tokio::spawn(un_cycle(db.clone(), http.clone())).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::error!("Collecteur : cycle échoué ({e}) — cycle suivant"),
            Err(panique) => {
                let any = panique.into_panic();
                let msg = any
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| any.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "cause inconnue".into());
                tracing::error!("Collecteur : cycle a paniqué ({msg}) — cycle suivant");
            }
        }
        tokio::time::sleep(Duration::from_secs(CYCLE_SEC)).await;
    }
}

/// Un cycle complet : sources actives → fetch RSS → traitement → insertion.
async fn un_cycle(db: Arc<Database>, http: Arc<reqwest::Client>) -> anyhow::Result<()> {
    let sources = db.lister_sources_presse(true).await?;
    let mut total_inserees = 0u64;
    for source in &sources {
        let items = news::news_rss::fetch_rss(&http, &source.url_rss).await;
        if items.is_empty() {
            tracing::debug!("Collecteur : flux vide ou down — {}", source.nom);
            continue;
        }
        let articles = news::presse_classif::traiter_items(&items, &source.nom, source.poids_score);
        let entrants: Vec<db::presse::ArticleEntrant> =
            articles.iter().map(vers_article_entrant).collect();
        let inserees = db.inserer_articles_presse_converts(&entrants).await?;
        total_inserees += inserees;
        tracing::info!(
            "Collecteur : {} — {} items → {} articles insérés",
            source.nom, items.len(), inserees
        );
    }
    if total_inserees > 0 {
        tracing::info!("Collecteur : cycle terminé, {total_inserees} nouveaux articles");
    }
    Ok(())
}

/// Pont explicite crate news → crate db : les deux types sont identiques
/// champ à champ mais les crates ne se connaissent pas (le pont par type
/// témoin `ArticleEntrant` vit dans db::presse).
fn vers_article_entrant(a: &ArticleCollecte) -> db::presse::ArticleEntrant {
    db::presse::ArticleEntrant {
        hash_titre: a.hash_titre.clone(),
        titre: a.titre.clone(),
        url: a.url.clone(),
        source_nom: a.source_nom.clone(),
        publie_le: a.publie_le.clone(),
        score: a.score,
        theme: a.theme.clone(),
        assets_concernes: a.assets_concernes.clone(),
        impact: a.impact.clone(),
    }
}
