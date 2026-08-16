//! Endpoints de la revue de presse (Phase 4.1).

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::state::AppState;
use db::presse::FiltreArticles;

/// Listing de la bibliothèque : filtres optionnels + pagination (50/page).
pub async fn get_articles(
    state: web::Data<AppState>,
    q: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let page: i64 = q.get("page").and_then(|p| p.parse().ok()).unwrap_or(1).max(1);
    let filtre = FiltreArticles {
        theme: q.get("theme").cloned().filter(|s| !s.is_empty()),
        asset: q.get("asset").cloned().filter(|s| !s.is_empty()),
        source: q.get("source").cloned().filter(|s| !s.is_empty()),
        q: q.get("q").cloned().filter(|s| !s.is_empty()),
        lu: q.get("lu").map(|l| l == "1" || l == "true"),
        limite: 50,
        offset: (page - 1) * 50,
    };
    match state.db.lister_articles_presse(&filtre).await {
        Ok(articles) => HttpResponse::Ok().json(serde_json::json!({ "articles": articles, "page": page })),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// Consultation : traduction lazy (porte d'entrée), sentiment, marquage lu.
pub async fn ouvrir_article(state: web::Data<AppState>, chemin: web::Path<String>) -> HttpResponse {
    let hash = chemin.into_inner();
    let pool: SqlitePool = state.db.pool().clone();
    let article_lu = match state.db.lire_article_presse(&hash).await {
        Ok(a) => a,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
    };
    let Some(mut article) = article_lu else {
        return HttpResponse::NotFound().json(serde_json::json!({"erreur": "article inconnu"}));
    };

    // Traduction : cache → Ollama strict → machine à états (2 échecs = suppression).
    let titre_fr = if article.statut_traduction == "ok" {
        // Déjà traduit une fois : le cache suffit (pas de nouvelle tentative).
        news::news_traduction::traduire_avec_cache_strict(&pool, &article.titre).await
    } else {
        match news::news_traduction::traduire_avec_cache_strict(&pool, &article.titre).await {
            Some(t) => {
                let _ = state.db.enregistrer_tentative_traduction(&hash, true).await;
                article.statut_traduction = "ok".into();
                Some(t)
            }
            None => {
                let condamne =
                    state.db.enregistrer_tentative_traduction(&hash, false).await.unwrap_or(false);
                if condamne {
                    let _ = state.db.supprimer_articles_condamnes().await;
                    return HttpResponse::Gone().json(serde_json::json!(
                        {"erreur": "traduction impossible ×2 — article supprimé"}
                    ));
                }
                None // VO affichée, prochaine ouverture réessaiera
            }
        }
    };

    // Sentiment (non bloquant, caché côté news_sentiment).
    let sentiment = if titre_fr.is_some() {
        let s = news::news_traduction::analyser_sentiment_avec_cache(&pool, &article.titre).await;
        if s.is_empty() { None } else { Some(s) }
    } else {
        None
    };

    let _ = state.db.marquer_lu_presse(&hash).await;
    article.lu = true;
    HttpResponse::Ok().json(serde_json::json!({
        "article": article, "titre_fr": titre_fr, "sentiment": sentiment,
    }))
}

/// Toutes les sources (actives et retirées — pilotage type « assets »).
pub async fn get_sources(state: web::Data<AppState>) -> HttpResponse {
    match state.db.lister_sources_presse(false).await {
        Ok(sources) => HttpResponse::Ok().json(sources),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

#[derive(Deserialize)]
pub struct CorpsSource {
    pub nom: String,
    pub url_rss: String,
    pub poids: Option<u8>,
    pub categorie: Option<String>,
}

pub async fn post_source(state: web::Data<AppState>, corps: web::Json<CorpsSource>) -> HttpResponse {
    if corps.nom.trim().is_empty() || !corps.url_rss.starts_with("https://") {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"erreur": "nom requis, URL https:// requise"}));
    }
    match state
        .db
        .ajouter_source_presse(
            corps.nom.trim(),
            corps.url_rss.trim(),
            corps.poids.unwrap_or(30).min(50),
            corps.categorie.as_deref().unwrap_or("marches"),
        )
        .await
    {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({"id": id})),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

pub async fn delete_source(state: web::Data<AppState>, chemin: web::Path<i64>) -> HttpResponse {
    match state.db.retirer_source_presse(chemin.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"ok": true})),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}
