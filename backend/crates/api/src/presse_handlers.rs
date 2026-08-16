//! Endpoints de la revue de presse (Phase 4.1).

use actix_web::{web, HttpResponse};
use futures::future::join_all;
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
        Ok(articles) => {
            // Peupler `titre_fr` depuis le cache pour les articles déjà traduits
            // (statut « ok »). Lecture de cache PURE — jamais d'appel Ollama
            // dans un listing de 50 articles : les traductions manquantes se
            // font aux consultations (ouvrir) et par le collecteur (max 5/cycle).
            let pool = state.db.pool().clone();
            let mut articles_avec_fr: Vec<serde_json::Value> = Vec::with_capacity(articles.len());
            for a in &articles {
                let mut v = serde_json::to_value(a).unwrap_or_default();
                if a.statut_traduction == "ok" {
                    let hash = news::news_traduction::hash_titre(&a.titre);
                    if let Some(fr) = news::news_traduction::cache_valide(
                        &a.titre,
                        news::news_traduction::lire_cache(&pool, &hash).await,
                    ) {
                        v["titre_fr"] = serde_json::Value::String(fr);
                    }
                }
                articles_avec_fr.push(v);
            }
            HttpResponse::Ok().json(serde_json::json!({ "articles": articles_avec_fr, "page": page }))
        }
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
    let mut echec_traduction = false;
    let mut titre_fr = if article.statut_traduction == "ok" {
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
                echec_traduction = true;
                None // VO affichée, prochaine ouverture réessaiera
            }
        }
    };

    // Filtre anti-chinois : du CJK dans le titre — VO issue d'une source
    // chinoise OU traduction corrompue par le modèle — = article non
    // traduisible. Même mécanique que la traduction impossible : échec
    // enregistré, 2e offense = suppression.
    if !echec_traduction
        && (news::news_traduction::contient_cjk(&article.titre)
            || titre_fr.as_deref().map_or(false, news::news_traduction::contient_cjk))
    {
        echec_traduction = true;
        titre_fr = None;
    }

    // Condamnation partagée (traduction impossible ×2 OU CJK ×2) : le 2e
    // échec déclenche la suppression effective et un 410 au client.
    if echec_traduction {
        let condamne =
            state.db.enregistrer_tentative_traduction(&hash, false).await.unwrap_or(false);
        if condamne {
            let _ = state.db.supprimer_articles_condamnes().await;
            return HttpResponse::Gone().json(serde_json::json!(
                {"erreur": "traduction impossible ×2 — article supprimé"}
            ));
        }
    }

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
        // Résumé RSS de la collecte — LE contenu affiché par la liseuse
        // (option A : plus aucun scrape d'article complet).
        "resume_source": article.resume_source,
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

    // VALIDATION AU MOMENT DE L'AJOUT : fetch immédiat du flux pour vérifier
    // (a) qu'il répond et contient des items, (b) si les items incluent une
    // <description> (résumé RSS). Un flux sans description n'affichera que
    // les titres dans la liseuse (le résumé RSS EST le contenu, option A) —
    // l'utilisateur doit le savoir AVANT.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    let items = news::news_rss::fetch_rss(&client, corps.url_rss.trim()).await;
    if items.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "erreur": "Flux RSS injoignable ou vide — vérifie l'URL (le flux doit répondre à un GET simple, sans JavaScript)"
        }));
    }
    let avec_description = items.iter().filter(|i| !i.resume.trim().is_empty()).count();
    let description_incluse = avec_description > 0;

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
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({
            "id": id,
            "description_incluse": description_incluse,
            "items_testes": items.len(),
            "items_avec_description": avec_description,
            "avertissement": if description_incluse { None } else {
                Some("Ce flux n'inclut pas de description dans ses items — les articles n'afficheront que leur titre (aucun résumé disponible). Considère un flux alternatif.")
            }
        })),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

pub async fn delete_source(state: web::Data<AppState>, chemin: web::Path<i64>) -> HttpResponse {
    match state.db.retirer_source_presse(chemin.into_inner()).await {
        Ok(articles_supprimes) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "articles_supprimes": articles_supprimes,
        })),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// Génération à la demande du brief 24 h : sélection top 15, traduction à la
/// volée des non-traduits (porte d'entrée = génération, PAS de suppression —
/// la condamnation reste le privilège de la consultation), synthèse LLM puis
/// archivage. 503 propre si Ollama est indisponible.
pub async fn post_brief(state: web::Data<AppState>) -> HttpResponse {
    let articles = match state.db.selection_brief_24h(8).await {
        Ok(a) => a,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
    };
    if articles.is_empty() {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"erreur": "aucun article dans les dernières 24 h"}),
        );
    }

    // Traduire à la volée les articles du brief qui ne le sont pas (seuls eux).
    // En parallèle (join_all) : jusqu'à 15 traductions séquentielles à froid
    // (~10 s chacune) frôlaient le timeout HTTP de 180 s. Le sémaphore Ollama
    // (OLLAMA_SEMAPHORE dans traduire_avec_cache_strict) borne la concurrence
    // réelle côté LLM ; le pool SQLite (WAL + busy_timeout) tolère le reste.
    let pool: SqlitePool = state.db.pool().clone();
    let db = state.db.clone(); // Arc<Database> pour la closure
    let taches = articles.iter().map(|a| {
        let pool = pool.clone(); // handle Arc interne — cloné par tâche
        let db = db.clone();
        async move {
            let titre_affiche = if a.statut_traduction == "ok" {
                // Déjà traduit : le cache suffit (pas de nouvelle tentative).
                news::news_traduction::traduire_avec_cache_strict(&pool, &a.titre)
                    .await
                    .unwrap_or_else(|| a.titre.clone())
            } else {
                match news::news_traduction::traduire_avec_cache_strict(&pool, &a.titre).await {
                    Some(t) => {
                        let _ = db.enregistrer_tentative_traduction(&a.hash_titre, true).await;
                        t
                    }
                    None => a.titre.clone(), // VO conservée — pas de suppression ici
                }
            };
            format!("- [{:3}/100|{}] {} ({})\n", a.score, a.theme, titre_affiche, a.source_nom)
        }
    });
    let lignes = join_all(taches).await;
    let entree = lignes.join("");

    let Some(contenu) = news::news_traduction::generer_brief_llm(&entree).await else {
        return HttpResponse::ServiceUnavailable().json(
            serde_json::json!({"erreur": "Ollama indisponible — réessayer plus tard"}),
        );
    };

    let maintenant = chrono::Utc::now().timestamp();
    let id = match state
        .db
        .inserer_brief(maintenant - 86_400, maintenant, articles.len(), &contenu)
        .await
    {
        Ok(id) => id,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
    };
    HttpResponse::Ok().json(serde_json::json!({
        "id": id, "contenu": contenu, "nb_articles": articles.len(),
    }))
}

/// Les 20 derniers briefs archivés (contenu inclus).
pub async fn get_briefs(state: web::Data<AppState>) -> HttpResponse {
    match state.db.lister_briefs(20).await {
        Ok(briefs) => HttpResponse::Ok().json(briefs),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// Un brief par identifiant (récupération large puis filtre : volume faible).
pub async fn get_brief(state: web::Data<AppState>, chemin: web::Path<i64>) -> HttpResponse {
    match state.db.lister_briefs(1000).await {
        Ok(briefs) => match briefs.into_iter().find(|b| b.id == *chemin) {
            Some(b) => HttpResponse::Ok().json(b),
            None => HttpResponse::NotFound().body("brief inconnu"),
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}
