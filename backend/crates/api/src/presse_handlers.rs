//! Endpoints de la revue de presse (Phase 4.1).

use actix_web::{web, HttpResponse};
use futures::future::join_all;
use serde::Deserialize;
use sqlx::{Row, SqlitePool};

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
        impact_min: q.get("impact_min").cloned().filter(|s| !s.is_empty()),
        tri: q.get("tri").cloned().filter(|s| !s.is_empty()),
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
                    // Résumé traduit (clé « resume:<hash> ») — cache pur.
                    if !a.resume_source.is_empty() {
                        let cle = format!("resume:{}", a.hash_titre);
                        if let Some(rfr) = news::news_traduction::lire_traduction_cle(&pool, &cle).await {
                            if rfr.trim() != a.resume_source.trim() {
                                v["resume_fr"] = serde_json::Value::String(rfr);
                            }
                        }
                    }
                }
                articles_avec_fr.push(v);
            }
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM presse_articles")
                .fetch_one(state.db.pool())
                .await
                .unwrap_or(0);
            HttpResponse::Ok().json(serde_json::json!({ "articles": articles_avec_fr, "page": page, "total": total }))
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

    // Traduction : cache → DeepL/Ollama strict → machine à états (2 échecs = suppression).
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

/// Erreurs de génération du brief presse — mapping HTTP par l'appelant.
pub(crate) enum ErreurBrief {
    AucunArticle,
    OllamaIndisponible,
    Interne(String),
}

/// Cœur de « ⚡ Générer » (Revue de presse) : sélection top 24 h, traduction
/// à la volée des non-traduits (porte d'entrée = génération, PAS de
/// suppression — la condamnation reste le privilège de la consultation),
/// synthèse LLM puis archivage. Partagé app principale + relais mobile
/// (MOB, owner 17/10 : le bouton mobile produit CE brief, pas le briefing
/// de session de l'onglet Aujourd'hui).
pub(crate) async fn generer_et_archiver_brief(
    db: &std::sync::Arc<db::Database>,
) -> Result<(i64, String, usize), ErreurBrief> {
    let articles = db
        .selection_brief_24h(8)
        .await
        .map_err(|e| ErreurBrief::Interne(format!("{e}")))?;
    if articles.is_empty() {
        return Err(ErreurBrief::AucunArticle);
    }

    // Traduire à la volée les articles du brief qui ne le sont pas (seuls eux).
    // En parallèle (join_all) : jusqu'à 15 traductions séquentielles à froid
    // (~10 s chacune) frôlaient le timeout HTTP de 180 s. Le sémaphore Ollama
    // (OLLAMA_SEMAPHORE dans traduire_avec_cache_strict) borne la concurrence
    // réelle côté LLM ; le pool SQLite (WAL + busy_timeout) tolère le reste.
    let pool: SqlitePool = db.pool().clone();
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
        return Err(ErreurBrief::OllamaIndisponible);
    };

    let maintenant = chrono::Utc::now().timestamp();
    let id = db
        .inserer_brief(maintenant - 86_400, maintenant, articles.len(), &contenu)
        .await
        .map_err(|e| ErreurBrief::Interne(format!("{e}")))?;
    Ok((id, contenu, articles.len()))
}

/// Génération à la demande du brief 24 h (voir generer_et_archiver_brief).
/// 503 propre si Ollama est indisponible.
pub async fn post_brief(state: web::Data<AppState>) -> HttpResponse {
    match generer_et_archiver_brief(&state.db).await {
        Ok((id, contenu, nb)) => HttpResponse::Ok().json(
            serde_json::json!({ "id": id, "contenu": contenu, "nb_articles": nb }),
        ),
        Err(ErreurBrief::AucunArticle) => HttpResponse::BadRequest().json(
            serde_json::json!({"erreur": "aucun article dans les dernières 24 h"}),
        ),
        Err(ErreurBrief::OllamaIndisponible) => HttpResponse::ServiceUnavailable().json(
            serde_json::json!({"erreur": "Ollama indisponible — réessayer plus tard"}),
        ),
        Err(ErreurBrief::Interne(e)) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// Les 20 derniers briefs archivés (contenu inclus).
pub async fn get_briefs(state: web::Data<AppState>) -> HttpResponse {
    match state.db.lister_briefs(20).await {
        Ok(briefs) => HttpResponse::Ok().json(briefs),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// GET /api/mobile/presse — l'onglet « Revue de presse » du relais mobile
/// (owner 19→22/10) : 4 sections (macro / cryptos / métaux / techno —
/// techno = thème base « actions_us ») × 10 meilleurs articles des
/// dernières 24 h, chaque article avec son URL (owner 22/10 : cliquable)
/// + le dernier brief archivé s'il a moins de 24 h. Titres TOUJOURS en
/// français (owner 20/10) : traduire_avec_cache sert le cache et traduit
/// à la volée les manquants (DeepL si clé, sinon Ollama) — jamais de
/// génération LLM lourde ici ; le bouton ⚡ Générer repasse par POST
/// /api/mobile/brief (qwen3:32b côté serveur).
pub async fn get_presse_mobile(
    state: web::Data<AppState>,
    requete: actix_web::HttpRequest,
) -> HttpResponse {
    if !crate::mobile_relay::autorise(&state.db, &requete).await {
        return crate::mobile_relay::non_autorise();
    }
    let pool: SqlitePool = state.db.pool().clone();
    let maintenant = chrono::Utc::now().timestamp();

    // 10 meilleurs par thème (fenêtre 24 h), ordre des sections affiché.
    let rows = sqlx::query(
        "SELECT * FROM (
            SELECT hash_titre, titre, url, score, theme, ajoute_le,
                   ROW_NUMBER() OVER (
                     PARTITION BY theme ORDER BY score DESC, ajoute_le DESC) AS rang
            FROM presse_articles
            WHERE ajoute_le >= ? AND theme IN ('macro','crypto','metaux','actions_us')
         ) WHERE rang <= 10
         ORDER BY CASE theme WHEN 'macro' THEN 1 WHEN 'crypto' THEN 2
                             WHEN 'metaux' THEN 3 ELSE 4 END, rang",
    )
    .bind(maintenant - 86_400)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let mut articles: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for r in &rows {
        let titre: String = r.try_get("titre").unwrap_or_default();
        // Titre français : cache → traduction à la volée → cache (owner
        // 20/10) ; échec = VO affichée, le collecteur retentera.
        let hash: String = r.try_get("hash_titre").unwrap_or_default();
        let fr_brut = news::news_traduction::traduire_avec_cache(&pool, &titre).await;
        // Garde-fou : traduction vide = VO affichée (même piège que le
        // briefing, owner 23/10).
        let fr = if fr_brut.trim().is_empty() { titre.clone() } else { fr_brut };
        if fr.trim() != titre.trim() {
            let _ = state.db.enregistrer_tentative_traduction(&hash, true).await;
        }
        let theme: String = r.try_get("theme").unwrap_or_default();
        let section = match theme.as_str() {
            "crypto" => "cryptos",
            "metaux" => "metaux",
            "actions_us" => "techno",
            _ => "macro",
        };
        articles.push(serde_json::json!({
            "titre": fr,
            "url": r.try_get::<String, _>("url").unwrap_or_default(),
            "score": r.try_get::<i64, _>("score").unwrap_or_default(),
            "theme": theme,
            "section": section,
            "ajoute_le": r.try_get::<i64, _>("ajoute_le").unwrap_or_default(),
        }));
    }

    // Dernier brief s'il est frais (< 24 h) — sinon le bouton Générer.
    let brief: Option<serde_json::Value> = sqlx::query(
        "SELECT contenu, nb_articles, genere_le FROM presse_briefs
         WHERE genere_le >= ? ORDER BY id DESC LIMIT 1",
    )
    .bind(maintenant - 86_400)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    .map(|r| {
        serde_json::json!({
            "contenu": r.try_get::<String, _>("contenu").unwrap_or_default(),
            "nb_articles": r.try_get::<i64, _>("nb_articles").unwrap_or_default(),
            "genere_le": r.try_get::<i64, _>("genere_le").unwrap_or_default(),
        })
    });

    HttpResponse::Ok().json(serde_json::json!({ "articles": articles, "brief": brief }))
}



