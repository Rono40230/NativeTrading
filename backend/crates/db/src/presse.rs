//! Revue de presse — sources pilotables + bibliothèque d'articles
//! (Phase 4.1, spec 2026-08-15).

use serde::Serialize;

use crate::Database;

/// Flux RSS pilotable (comme les assets : actif/inactif, jamais supprimé).
#[derive(Debug, Serialize)]
pub struct PresseSource {
    pub id: i64,
    pub nom: String,
    pub url_rss: String,
    pub poids_score: u8,
    pub categorie: String,
    pub actif: bool,
}

/// Article de la bibliothèque. `assets_concernes` est un JSON array
/// (`["BTC","XAUUSD"]`), attribué par mots-clés au moment de la collecte.
#[derive(Debug, Clone, Serialize)]
pub struct PresseArticle {
    pub hash_titre: String,
    pub titre: String,
    pub url: String,
    pub source_nom: String,
    pub publie_le: String,
    pub score: u8,
    pub theme: String,
    pub assets_concernes: String,
    pub impact: String,
    pub statut_traduction: String,
    pub tentatives_traduction: u8,
    pub lu: bool,
    pub ajoute_le: i64,
    /// Résumé du flux RSS capté à la collecte — socle d'affichage quand le
    /// scraper échoue (sites rendus en JavaScript). Vide pour les lignes
    /// antérieures à la migration 0072.
    pub resume_source: String,
}

/// Article brut fourni par le collecteur (type témoin : le crate db ne
/// dépend pas du crate news — la conversion `ArticleCollecte` → `ArticleEntrant`
/// se fait chez l'appelant, champ à champ).
pub struct ArticleEntrant {
    pub hash_titre: String,
    pub titre: String,
    pub url: String,
    pub source_nom: String,
    pub publie_le: String,
    pub score: u8,
    pub theme: String,
    pub assets_concernes: String,
    pub impact: String,
    pub resume_source: String,
}

/// Filtres de listing de la bibliothèque (tous optionnels).
pub struct FiltreArticles {
    pub theme: Option<String>,
    pub asset: Option<String>,
    pub source: Option<String>,
    pub q: Option<String>,
    pub lu: Option<bool>,
    pub limite: i64,
    pub offset: i64,
}

/// Brief markdown généré périodiquement à partir de la sélection 24 h.
#[derive(Debug, Serialize)]
pub struct PresseBrief {
    pub id: i64,
    pub genere_le: i64,
    pub fenetre_de: i64,
    pub fenetre_a: i64,
    pub nb_articles: i64,
    pub contenu: String,
}

impl Database {
    pub async fn lister_sources_presse(&self, actives_seules: bool) -> anyhow::Result<Vec<PresseSource>> {
        let sql = if actives_seules {
            "SELECT id, nom, url_rss, poids_score, categorie, actif FROM presse_sources WHERE actif = 1 ORDER BY id"
        } else {
            "SELECT id, nom, url_rss, poids_score, categorie, actif FROM presse_sources ORDER BY id"
        };
        let lignes = sqlx::query_as::<_, (i64, String, String, i64, String, i64)>(sql)
            .fetch_all(self.pool()).await?;
        Ok(lignes.into_iter().map(|l| PresseSource {
            id: l.0, nom: l.1, url_rss: l.2, poids_score: l.3 as u8,
            categorie: l.4, actif: l.5 != 0,
        }).collect())
    }

    pub async fn ajouter_source_presse(&self, nom: &str, url: &str, poids: u8, categorie: &str) -> anyhow::Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO presse_sources (nom, url_rss, poids_score, categorie, actif, cree_le)
             VALUES (?1, ?2, ?3, ?4, 1, ?5) RETURNING id",
        )
        .bind(nom).bind(url).bind(poids as i64).bind(categorie)
        .bind(chrono::Utc::now().timestamp())
        .fetch_one(self.pool()).await?;
        Ok(id)
    }

    /// Suppression DÉFINITIVE d'une source : la ligne ET tous les articles
    /// de cette source (décision propriétaire — un flux retiré ne doit
    /// plus rien laisser dans la bibliothèque).
    pub async fn retirer_source_presse(&self, id: i64) -> anyhow::Result<u64> {
        // Nom de la source pour cibler les articles (clé = source_nom).
        let nom: Option<String> = sqlx::query_scalar(
            "SELECT nom FROM presse_sources WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        let mut supprimes = 0u64;
        if let Some(nom) = nom {
            let articles = sqlx::query("DELETE FROM presse_articles WHERE source_nom = ?1")
                .bind(&nom)
                .execute(self.pool())
                .await?
                .rows_affected();
            supprimes += articles;
        }
        sqlx::query("DELETE FROM presse_sources WHERE id = ?1")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(supprimes)
    }

    /// Insertion en masse avec dédoublonnage par hash (UNIQUE PK).
    /// Retourne le nombre d'articles réellement insérés.
    pub async fn inserer_articles_presse(&self, articles: &[PresseArticle]) -> anyhow::Result<u64> {
        let mut tx = self.pool().begin().await?;
        let mut inseres = 0u64;
        for a in articles {
            let r = sqlx::query(
                "INSERT OR IGNORE INTO presse_articles
                    (hash_titre, titre, url, source_nom, publie_le, score, theme,
                     assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le,
                     resume_source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'non_tente', 0, 0, ?10, ?11)",
            )
            .bind(&a.hash_titre).bind(&a.titre).bind(&a.url).bind(&a.source_nom)
            .bind(&a.publie_le).bind(a.score as i64).bind(&a.theme)
            .bind(&a.assets_concernes).bind(&a.impact)
            .bind(a.ajoute_le)
            .bind(&a.resume_source)
            .execute(&mut *tx).await?;
            inseres += r.rows_affected();
        }
        tx.commit().await?;
        Ok(inseres)
    }

    /// Pont collecteur → bibliothèque : convertit des `ArticleEntrant` en
    /// `PresseArticle` puis délègue à `inserer_articles_presse` (dédup par
    /// hash). `ajoute_le` = date de COLLECTE posée ici (horloge serveur, un
    /// timestamp unique pour tout le lot) : le champ n'existe pas côté
    /// `ArticleEntrant` car il n'a pas de sens à la collecte — revue tâche 3,
    /// `publie_le` est la date de publication de la source (parfois ancienne),
    /// `ajoute_le` la date d'entrée en bibliothèque (base de la sélection
    /// 24 h et de la purge). Le INSERT sous-jacent LIANT `a.ajoute_le`, la
    /// valeur doit être l'horodatage réel — jamais 0.
    pub async fn inserer_articles_presse_converts(&self, articles: &[ArticleEntrant]) -> anyhow::Result<u64> {
        let collecte_le = chrono::Utc::now().timestamp();
        let convertis: Vec<PresseArticle> = articles.iter().map(|a| PresseArticle {
            hash_titre: a.hash_titre.clone(),
            titre: a.titre.clone(),
            url: a.url.clone(),
            source_nom: a.source_nom.clone(),
            publie_le: a.publie_le.clone(),
            score: a.score,
            theme: a.theme.clone(),
            assets_concernes: a.assets_concernes.clone(),
            impact: a.impact.clone(),
            statut_traduction: "non_tente".into(),
            tentatives_traduction: 0,
            lu: false,
            ajoute_le: collecte_le,
            resume_source: a.resume_source.clone(),
        }).collect();
        self.inserer_articles_presse(&convertis).await
    }

    /// Listing filtré — paramètres TOUJOURS liés (jamais de formatage de
    /// chaîne dans le SQL) ; la clause asset cherche dans le JSON textuel
    /// (`assets LIKE '%"BTC"%'` : suffisant pour des tickers sans ambigüité).
    pub async fn lister_articles_presse(&self, f: &FiltreArticles) -> anyhow::Result<Vec<PresseArticle>> {
        // Phase 1 : construction de la requête (clauses conditionnelles).
        let mut sql = String::from(
            "SELECT hash_titre, titre, url, source_nom, publie_le, score, theme,
                    assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le,
                    resume_source
             FROM presse_articles WHERE 1=1",
        );
        if f.theme.is_some() { sql.push_str(" AND theme = ?"); }
        if f.asset.is_some() { sql.push_str(" AND assets_concernes LIKE ?"); }
        if f.source.is_some() { sql.push_str(" AND source_nom = ?"); }
        if f.q.is_some() { sql.push_str(" AND LOWER(titre) LIKE ?"); }
        if let Some(lu) = f.lu { sql.push_str(if lu { " AND lu = 1" } else { " AND lu = 0" }); }
        sql.push_str(" ORDER BY ajoute_le DESC LIMIT ? OFFSET ?");

        // Phase 2 : requête figée, puis binds dans le même ordre (le borrow
        // checker impose de ne muter `sql` qu'avant la création de la requête).
        let mut q = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, String, i64, i64, i64, String)>(&sql);
        if let Some(t) = &f.theme { q = q.bind(t.clone()); }
        if let Some(a) = &f.asset { q = q.bind(format!("%\"{a}\"%")); }
        if let Some(src) = &f.source { q = q.bind(src.clone()); }
        if let Some(qs) = &f.q { q = q.bind(format!("%{}%", qs.to_lowercase())); }
        q = q.bind(f.limite).bind(f.offset);
        let lignes = q.fetch_all(self.pool()).await?;
        Ok(lignes.into_iter().map(|l| PresseArticle {
            hash_titre: l.0, titre: l.1, url: l.2, source_nom: l.3, publie_le: l.4,
            score: l.5 as u8, theme: l.6, assets_concernes: l.7, impact: l.8,
            statut_traduction: l.9, tentatives_traduction: l.10 as u8, lu: l.11 != 0, ajoute_le: l.12,
            resume_source: l.13,
        }).collect())
    }

    pub async fn lire_article_presse(&self, hash: &str) -> anyhow::Result<Option<PresseArticle>> {
        // Pas de filtre hash dans FiltreArticles : requête dédiée.
        let sql = "SELECT hash_titre, titre, url, source_nom, publie_le, score, theme,
                          assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le,
                          resume_source
                   FROM presse_articles WHERE hash_titre = ?1";
        let ligne = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, String, i64, i64, i64, String)>(sql)
            .bind(hash).fetch_optional(self.pool()).await?;
        Ok(ligne.map(|l| PresseArticle {
            hash_titre: l.0, titre: l.1, url: l.2, source_nom: l.3, publie_le: l.4,
            score: l.5 as u8, theme: l.6, assets_concernes: l.7, impact: l.8,
            statut_traduction: l.9, tentatives_traduction: l.10 as u8, lu: l.11 != 0, ajoute_le: l.12,
            resume_source: l.13,
        }))
    }

    pub async fn marquer_lu_presse(&self, hash: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE presse_articles SET lu = 1 WHERE hash_titre = ?1")
            .bind(hash).execute(self.pool()).await?;
        Ok(())
    }

    /// Machine à états de la traduction (porte d'entrée de la bibliothèque).
    /// Retourne true = 2 échecs atteints → l'appelant doit condamner
    /// (puis supprimer) l'article.
    pub async fn enregistrer_tentative_traduction(&self, hash: &str, reussie: bool) -> anyhow::Result<bool> {
        if reussie {
            sqlx::query("UPDATE presse_articles SET statut_traduction = 'ok' WHERE hash_titre = ?1")
                .bind(hash).execute(self.pool()).await?;
            return Ok(false);
        }
        let tentatives: i64 = sqlx::query_scalar(
            "UPDATE presse_articles SET tentatives_traduction = tentatives_traduction + 1
             WHERE hash_titre = ?1 RETURNING tentatives_traduction",
        )
        .bind(hash).fetch_one(self.pool()).await?;
        Ok(tentatives >= 2)
    }

    /// Suppression effective des articles condamnés (appelée après le 2e échec).
    pub async fn supprimer_articles_condamnes(&self) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM presse_articles WHERE tentatives_traduction >= 2")
            .execute(self.pool()).await?;
        Ok(())
    }

    /// Sélection pour le brief : top score des dernières 24 h (sur `ajoute_le`).
    pub async fn selection_brief_24h(&self, limite: i64) -> anyhow::Result<Vec<PresseArticle>> {
        let cutoff = chrono::Utc::now().timestamp() - 86_400;
        let sql = "SELECT hash_titre, titre, url, source_nom, publie_le, score, theme,
                          assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le,
                          resume_source
                   FROM presse_articles WHERE ajoute_le >= ?1
                   ORDER BY score DESC, ajoute_le DESC LIMIT ?2";
        let lignes = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, String, i64, i64, i64, String)>(sql)
            .bind(cutoff).bind(limite).fetch_all(self.pool()).await?;
        Ok(lignes.into_iter().map(|l| PresseArticle {
            hash_titre: l.0, titre: l.1, url: l.2, source_nom: l.3, publie_le: l.4,
            score: l.5 as u8, theme: l.6, assets_concernes: l.7, impact: l.8,
            statut_traduction: l.9, tentatives_traduction: l.10 as u8, lu: l.11 != 0, ajoute_le: l.12,
            resume_source: l.13,
        }).collect())
    }

    /// Insère un brief généré ; `genere_le` est posé ici (horloge serveur).
    pub async fn inserer_brief(&self, fenetre_de: i64, fenetre_a: i64, nb_articles: usize, contenu: &str) -> anyhow::Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO presse_briefs (genere_le, fenetre_de, fenetre_a, nb_articles, contenu)
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )
        .bind(chrono::Utc::now().timestamp())
        .bind(fenetre_de).bind(fenetre_a).bind(nb_articles as i64).bind(contenu)
        .fetch_one(self.pool()).await?;
        Ok(id)
    }

    /// Briefs les plus récents en premier (contenu inclus : simple).
    pub async fn lister_briefs(&self, limite: i64) -> anyhow::Result<Vec<PresseBrief>> {
        let lignes = sqlx::query_as::<_, (i64, i64, i64, i64, i64, String)>(
            "SELECT id, genere_le, fenetre_de, fenetre_a, nb_articles, contenu
             FROM presse_briefs ORDER BY id DESC LIMIT ?1",
        )
        .bind(limite).fetch_all(self.pool()).await?;
        Ok(lignes.into_iter().map(|l| PresseBrief {
            id: l.0, genere_le: l.1, fenetre_de: l.2, fenetre_a: l.3, nb_articles: l.4, contenu: l.5,
        }).collect())
    }

    /// Rétention presse en mois (clé `retention_presse_mois`). Absence ou
    /// valeur invalide → 12 mois par défaut (la presse pèse peu, mais la
    /// bibliothèque n'a pas vocation à croître indéfiniment).
    pub async fn lire_retention_presse(&self) -> i64 {
        match self.lire_config("retention_presse_mois").await {
            Ok(Some(v)) => v.trim().parse().unwrap_or(12),
            _ => 12,
        }
    }

    /// Rétention : supprime articles ET briefs au-delà de N mois
    /// (mois approximé à 30 jours, suffisant pour de la rétention).
    /// Retourne le total de lignes supprimées.
    pub async fn purger_presse_expiree(&self, mois: i64) -> anyhow::Result<u64> {
        if mois <= 0 { return Ok(0); }
        let cutoff = chrono::Utc::now().timestamp() - mois * 30 * 86_400;
        let a = sqlx::query("DELETE FROM presse_articles WHERE ajoute_le < ?1")
            .bind(cutoff).execute(self.pool()).await?.rows_affected();
        let b = sqlx::query("DELETE FROM presse_briefs WHERE genere_le < ?1")
            .bind(cutoff).execute(self.pool()).await?.rows_affected();
        Ok(a + b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    fn article(hash: &str, theme: &str, assets: &str, score: u8) -> PresseArticle {
        PresseArticle {
            hash_titre: hash.into(),
            titre: format!("Titre {hash}"),
            url: format!("https://exemple.fr/{hash}"),
            source_nom: "Test".into(),
            publie_le: "2026-08-15T10:00:00Z".into(),
            score,
            theme: theme.into(),
            assets_concernes: assets.into(),
            impact: "moyen".into(),
            statut_traduction: "non_tente".into(),
            tentatives_traduction: 0,
            lu: false,
            ajoute_le: 1_786_700_000,
            resume_source: String::new(),
        }
    }

    #[tokio::test]
    async fn sources_pre_remplies_et_pilotables() {
        let db = db_test().await;
        let toutes = db.lister_sources_presse(false).await.unwrap();
        assert_eq!(toutes.len(), 9, "9 flux pré-remplis par la migration");
        assert!(toutes.iter().all(|s| s.actif));

        let id = db.ajouter_source_presse("Test", "https://x.fr/rss", 20, "marches").await.unwrap();
        db.retirer_source_presse(id).await.unwrap();
        let actives = db.lister_sources_presse(true).await.unwrap();
        assert_eq!(actives.len(), 9, "retirée = désactivée, pas supprimée");
    }

    #[tokio::test]
    async fn insertion_deduplique_et_filtres() {
        let db = db_test().await;
        let a1 = article("h1", "crypto", r#"["BTC"]"#, 80);
        let a2 = article("h2", "macro", r#"["XAUUSD"]"#, 40);
        let n = db.inserer_articles_presse(&[a1.clone(), a2]).await.unwrap();
        assert_eq!(n, 2);
        // Doublon par hash → ignoré
        let n = db.inserer_articles_presse(&[a1]).await.unwrap();
        assert_eq!(n, 0);

        let filtre = FiltreArticles {
            theme: Some("crypto".into()), asset: Some("BTC".into()),
            source: None, q: None, lu: None, limite: 50, offset: 0,
        };
        let res = db.lister_articles_presse(&filtre).await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].hash_titre, "h1");

        // Recherche texte
        let filtre_q = FiltreArticles {
            theme: None, asset: None, source: None,
            q: Some("titre h2".into()), lu: None, limite: 50, offset: 0,
        };
        assert_eq!(db.lister_articles_presse(&filtre_q).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn convertis_pose_la_date_de_collecte() {
        let db = db_test().await;
        let e = ArticleEntrant {
            hash_titre: "c1".into(),
            titre: "Titre c1".into(),
            url: "https://exemple.fr/c1".into(),
            source_nom: "Test".into(),
            publie_le: "2026-08-15T10:00:00Z".into(),
            score: 50,
            theme: "macro".into(),
            assets_concernes: "[]".into(),
            impact: "moyen".into(),
            resume_source: "Résumé RSS capté à la collecte.".into(),
        };
        assert_eq!(db.inserer_articles_presse_converts(&[e]).await.unwrap(), 1);
        let insere = db.lire_article_presse("c1").await.unwrap().unwrap();
        // ajoute_le = date de COLLECTE (jamais 0 — sinon sélection 24 h et
        // purge seraient faussées), même si publie_le est ancien.
        assert!(insere.ajoute_le >= chrono::Utc::now().timestamp() - 60, "ajoute_le = {}", insere.ajoute_le);
        assert_eq!(insere.statut_traduction, "non_tente");
        // Le résumé RSS survit au aller-retour collecteur → DB → lecture
        // (c'est lui qui alimente la modal quand le scraper échoue).
        assert_eq!(insere.resume_source, "Résumé RSS capté à la collecte.");
        assert_eq!(db.selection_brief_24h(10).await.unwrap()[0].resume_source, "Résumé RSS capté à la collecte.");
    }

    #[tokio::test]
    async fn deux_echecs_traduction_suppriment_larticle() {
        let db = db_test().await;
        db.inserer_articles_presse(&[article("h1", "macro", "[]", 50)]).await.unwrap();
        // 1er échec : garde
        let suppr = db.enregistrer_tentative_traduction("h1", false).await.unwrap();
        assert!(!suppr);
        assert!(db.lire_article_presse("h1").await.unwrap().is_some());
        // 2e échec : supprimé
        let suppr = db.enregistrer_tentative_traduction("h1", false).await.unwrap();
        assert!(suppr);
        db.supprimer_articles_condamnes().await.unwrap(); // suppression immédiate après condamnation
        assert!(db.lire_article_presse("h1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn succes_traduction_pose_statut_ok() {
        let db = db_test().await;
        db.inserer_articles_presse(&[article("h2", "macro", "[]", 50)]).await.unwrap();
        let suppr = db.enregistrer_tentative_traduction("h2", true).await.unwrap();
        assert!(!suppr);
        let a = db.lire_article_presse("h2").await.unwrap().unwrap();
        assert_eq!(a.statut_traduction, "ok");
    }

    #[tokio::test]
    async fn briefs_et_selection_24h() {
        let db = db_test().await;
        let maintenant = chrono::Utc::now().timestamp();
        let vieux = PresseArticle { ajoute_le: maintenant - 86_400 * 3, ..article("vieux", "macro", "[]", 90) };
        let recent = PresseArticle { ajoute_le: maintenant - 3_600, ..article("recent", "crypto", "[]", 70) };
        db.inserer_articles_presse(&[vieux, recent]).await.unwrap();
        let sel = db.selection_brief_24h(15).await.unwrap();
        assert_eq!(sel.len(), 1, "seul l'article des 24 dernières heures");
        assert_eq!(sel[0].hash_titre, "recent");

        let id = db.inserer_brief(maintenant - 86_400, maintenant, 1, "# Brief").await.unwrap();
        let briefs = db.lister_briefs(10).await.unwrap();
        assert_eq!(briefs.len(), 1);
        assert_eq!(briefs[0].id, id);
    }

    #[tokio::test]
    async fn purge_presse_par_mois() {
        let db = db_test().await;
        let vieux = PresseArticle { ajoute_le: chrono::Utc::now().timestamp() - 86_400 * 400, ..article("vieux", "macro", "[]", 50) };
        db.inserer_articles_presse(&[vieux, article("recent", "macro", "[]", 50)]).await.unwrap();
        assert_eq!(db.purger_presse_expiree(12).await.unwrap(), 1, "400 jours > 12 mois");
        assert!(db.lire_article_presse("recent").await.unwrap().is_some());
    }
}
