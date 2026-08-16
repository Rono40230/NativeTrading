# Revue de Presse — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bibliothèque de presse consultable (process collecteur séparé 30 min, IA à la demande, brief 24 h) selon la spec `docs/superpowers/specs/2026-08-15-revue-presse-design.md`.

**Architecture:** Un binaire collecteur (api crate, `bin/news_collector.rs`) écrit dans des tables presse dédiées ; le backend expose `/api/presse/*` avec traduction/sentiment Ollama paresseux (cache, échec ×2 = suppression) et génération de brief à la demande ; vue Vue.js dédiée + extrait dashboard.

**Tech Stack:** Rust (workspace existant : crates db, news, api), SQLite (sqlx), Ollama local (qwen2.5:3b pour traduire — déjà en place), Vue 3 + TypeScript.

## Global Constraints

- Langue : code et commentaires en français (convention du repo).
- Tests : `cargo test -p <crate>` — base `:memory:` via `Database::new(":memory:")` + `run_migrations()`. Aucun test ne doit toucher le réseau ni Ollama.
- Migrations : numérotation séquentielle — la dernière existante est `0070_runtime_emissions.sql`, la presse commence à `0071`.
- Règle L2 (ROADMAP « Leçons durables ») : toute édition programmatique vérifiée appliquée (assert/grep), pas de replace silencieux.
- Amendement à la spec : le lu/non-lu vit dans une colonne `presse_articles.lu` (jointure avec `news_lus` évitée — plus simple, même comportement).
- Contrat Ollama : `traduire()` rend le texte ORIGINAL en cas d'échec (vérifié dans `news_traduction.rs:70-80`) — c'est le signal d'échec.

---

### Task 1: Migration 0071 — tables presse

**Files:**
- Create: `backend/crates/db/migrations/0071_presse.sql`

**Interfaces:**
- Produces: tables `presse_sources`, `presse_articles`, `presse_briefs` + clé config `retention_presse_mois` — utilisées par tous les tasks suivants.

- [ ] **Step 1: Écrire la migration**

```sql
-- 0071 — Revue de presse (Phase 4.1, spec 2026-08-15).
--
-- presse_sources : flux RSS pilotables par l'utilisateur (comme les assets).
-- presse_articles : la bibliothèque. statut_traduction : 'non_tente' | 'ok' ;
--   les échecs ×2 mènent à la SUPPRESSION de la ligne (règle "porte d'entrée").
-- presse_briefs : briefs générés à la demande (aucun écrasement).

CREATE TABLE presse_sources (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    nom         TEXT NOT NULL,
    url_rss     TEXT NOT NULL UNIQUE,
    poids_score INTEGER NOT NULL DEFAULT 30,
    categorie   TEXT NOT NULL DEFAULT 'marches',
    actif       INTEGER NOT NULL DEFAULT 1,
    cree_le     INTEGER NOT NULL
);

CREATE TABLE presse_articles (
    hash_titre          TEXT PRIMARY KEY,
    titre               TEXT NOT NULL,
    url                 TEXT NOT NULL,
    source_nom          TEXT NOT NULL,
    publie_le           TEXT NOT NULL,
    score               INTEGER NOT NULL,
    theme               TEXT NOT NULL,
    assets_concernes    TEXT NOT NULL DEFAULT '[]',
    impact              TEXT NOT NULL DEFAULT 'faible',
    statut_traduction   TEXT NOT NULL DEFAULT 'non_tente',
    tentatives_traduction INTEGER NOT NULL DEFAULT 0,
    lu                  INTEGER NOT NULL DEFAULT 0,
    ajoute_le           INTEGER NOT NULL
);
CREATE INDEX idx_presse_articles_ajoute ON presse_articles(ajoute_le DESC);
CREATE INDEX idx_presse_articles_theme ON presse_articles(theme);

CREATE TABLE presse_briefs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    genere_le   INTEGER NOT NULL,
    fenetre_de  INTEGER NOT NULL,
    fenetre_a   INTEGER NOT NULL,
    nb_articles INTEGER NOT NULL,
    contenu     TEXT NOT NULL
);

-- Point de départ : les 9 flux historiques (modifiables/supprimables ensuite)
INSERT INTO presse_sources (nom, url_rss, poids_score, categorie, cree_le) VALUES
('Reuters Business',  'https://feeds.reuters.com/reuters/businessNews', 40, 'marches', strftime('%s','now')),
('CNBC Markets',      'https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=10000664', 35, 'marches', strftime('%s','now')),
('MarketWatch',       'https://feeds.marketwatch.com/marketwatch/marketpulse/', 35, 'marches', strftime('%s','now')),
('FXStreet',          'https://www.fxstreet.com/rss/news', 38, 'forex', strftime('%s','now')),
('Kitco Métaux',      'https://www.kitco.com/rss/KitcoNewsRSS.xml', 38, 'metaux', strftime('%s','now')),
('CoinTelegraph',     'https://cointelegraph.com/rss', 30, 'crypto', strftime('%s','now')),
('CryptoNews',        'https://cryptonews.com/news/feed', 28, 'crypto', strftime('%s','now')),
('Decrypt',           'https://decrypt.co/feed', 30, 'crypto', strftime('%s','now')),
('Yahoo Finance',     'https://finance.yahoo.com/news/rssindex', 28, 'marches', strftime('%s','now'));

INSERT INTO configuration (cle, valeur, maj_le) VALUES
('retention_presse_mois', '12', strftime('%s','now'))
ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur, maj_le = excluded.maj_le;
```

- [ ] **Step 2: Vérifier que la migration s'applique**

Run: `cd backend && cargo test -p db --lib -- --test-threads=1 2>&1 | tail -3`
Expected: tous les tests db passent (les tests `db_test()` exécutent `run_migrations` — la migration doit être valide).

- [ ] **Step 3: Commit**

```bash
git add backend/crates/db/migrations/0071_presse.sql
git commit -m "feat(presse): migration 0071 — sources, articles, briefs"
```

---

### Task 2: db::presse — sources et articles

**Files:**
- Create: `backend/crates/db/src/presse.rs`
- Modify: `backend/crates/db/src/lib.rs` (ajouter `pub mod presse;` après `pub mod runtime_observation;`)

**Interfaces:**
- Consumes: tables du Task 1.
- Produces (utilisées par Tasks 3, 5, 6, 7, 8, 9) :
  - `pub struct PresseSource { pub id: i64, pub nom: String, pub url_rss: String, pub poids_score: u8, pub categorie: String, pub actif: bool }`
  - `pub struct PresseArticle { pub hash_titre: String, pub titre: String, pub url: String, pub source_nom: String, pub publie_le: String, pub score: u8, pub theme: String, pub assets_concernes: String, pub impact: String, pub statut_traduction: String, pub tentatives_traduction: u8, pub lu: bool, pub ajoute_le: i64 }`
  - `impl Database` : `lister_sources_presse(&self, actives_seules: bool) -> anyhow::Result<Vec<PresseSource>>`, `ajouter_source_presse(&self, nom: &str, url: &str, poids: u8, categorie: &str) -> anyhow::Result<i64>`, `retirer_source_presse(&self, id: i64) -> anyhow::Result<()>`, `inserer_articles_presse(&self, articles: &[PresseArticle]) -> anyhow::Result<u64>` (INSERT OR IGNORE, retourne le nombre réellement insérées), `lister_articles_presse(&self, filtre: &FiltreArticles) -> anyhow::Result<Vec<PresseArticle>>`, `marquer_lu_presse(&self, hash: &str)`, `lire_article_presse(&self, hash: &str) -> Option<PresseArticle>`
  - `pub struct FiltreArticles { pub theme: Option<String>, pub asset: Option<String>, pub source: Option<String>, pub q: Option<String>, pub lu: Option<bool>, pub limite: i64, pub offset: i64 }`

- [ ] **Step 1: Écrire les tests (fichier de tests dans presse.rs)**

```rust
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
}
```

- [ ] **Step 2: Exécuter — doit échouer (module absent)**

Run: `cd backend && cargo test -p db presse 2>&1 | tail -3`
Expected: erreur de compilation (presse.rs inexistant).

- [ ] **Step 3: Écrire presse.rs**

```rust
//! Revue de presse — sources pilotables + bibliothèque d'articles
//! (Phase 4.1, spec 2026-08-15).

use serde::Serialize;

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

    pub async fn retirer_source_presse(&self, id: i64) -> anyhow::Result<()> {
        sqlx::query("UPDATE presse_sources SET actif = 0 WHERE id = ?1")
            .bind(id).execute(self.pool()).await?;
        Ok(())
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
                     assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'non_tente', 0, 0, ?10)",
            )
            .bind(&a.hash_titre).bind(&a.titre).bind(&a.url).bind(&a.source_nom)
            .bind(&a.publie_le).bind(a.score as i64).bind(&a.theme)
            .bind(&a.assets_concernes).bind(&a.impact)
            .bind(chrono::Utc::now().timestamp())
            .execute(&mut *tx).await?;
            inseres += r.rows_affected();
        }
        tx.commit().await?;
        Ok(inseres)
    }

    /// Listing filtré — paramètres TOUJOURS liés (jamais de formatage de
    /// chaîne dans le SQL) ; la clause asset cherche dans le JSON textuel
    /// (`assets LIKE '%"BTC"%'` : suffisant pour des tickers sans ambigüité).
    pub async fn lister_articles_presse(&self, f: &FiltreArticles) -> anyhow::Result<Vec<PresseArticle>> {
        let mut sql = String::from(
            "SELECT hash_titre, titre, url, source_nom, publie_le, score, theme,
                    assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le
             FROM presse_articles WHERE 1=1",
        );
        let mut q = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, String, i64, i64, i64)>(&sql);
        if let Some(t) = &f.theme { sql.push_str(" AND theme = ?"); q = q.bind(t.clone()); }
        if let Some(a) = &f.asset {
            sql.push_str(" AND assets_concernes LIKE ?");
            q = q.bind(format!("%\"{a}\"%"));
        }
        if let Some(src) = &f.source { sql.push_str(" AND source_nom = ?"); q = q.bind(src.clone()); }
        if let Some(qs) = &f.q { sql.push_str(" AND LOWER(titre) LIKE ?"); q = q.bind(format!("%{}%", qs.to_lowercase())); }
        if let Some(lu) = f.lu { sql.push_str(if lu { " AND lu = 1" } else { " AND lu = 0" }); }
        sql.push_str(" ORDER BY ajoute_le DESC LIMIT ? OFFSET ?");
        let q = q.bind(f.limite).bind(f.offset);
        let lignes = q.fetch_all(self.pool()).await?;
        Ok(lignes.into_iter().map(|l| PresseArticle {
            hash_titre: l.0, titre: l.1, url: l.2, source_nom: l.3, publie_le: l.4,
            score: l.5 as u8, theme: l.6, assets_concernes: l.7, impact: l.8,
            statut_traduction: l.9, tentatives_traduction: l.10 as u8, lu: l.11 != 0, ajoute_le: l.12,
        }).collect())
    }

    pub async fn lire_article_presse(&self, hash: &str) -> anyhow::Result<Option<PresseArticle>> {
        let f = FiltreArticles { theme: None, asset: None, source: None, q: None, lu: None, limite: 1, offset: 0 };
        // Pas de filtre hash dans FiltreArticles : requête dédiée.
        let sql = "SELECT hash_titre, titre, url, source_nom, publie_le, score, theme,
                          assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le
                   FROM presse_articles WHERE hash_titre = ?1";
        let ligne = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, String, i64, i64, i64)>(sql)
            .bind(hash).fetch_optional(self.pool()).await?;
        Ok(ligne.map(|l| PresseArticle {
            hash_titre: l.0, titre: l.1, url: l.2, source_nom: l.3, publie_le: l.4,
            score: l.5 as u8, theme: l.6, assets_concernes: l.7, impact: l.8,
            statut_traduction: l.9, tentatives_traduction: l.10 as u8, lu: l.11 != 0, ajoute_le: l.12,
        }))
    }

    pub async fn marquer_lu_presse(&self, hash: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE presse_articles SET lu = 1 WHERE hash_titre = ?1")
            .bind(hash).execute(self.pool()).await?;
        Ok(())
    }
}
```

Déclarer le module dans `lib.rs` : ajouter `pub mod presse;` après la ligne `pub mod runtime_observation;`.

- [ ] **Step 4: Exécuter les tests**

Run: `cd backend && cargo test -p db presse 2>&1 | tail -3`
Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add backend/crates/db/src/presse.rs backend/crates/db/src/lib.rs
git commit -m "feat(presse): db — sources pilotables + articles filtrables"
```

---

### Task 3: db::presse — machine à états traduction, briefs, rétention

**Files:**
- Modify: `backend/crates/db/src/presse.rs` (ajouts)

**Interfaces:**
- Consumes: Task 2.
- Produces (Tasks 6, 7, 8) :
  - `pub async fn enregistrer_tentative_traduction(&self, hash: &str, reussie: bool) -> anyhow::Result<bool>` — retourne `true` si l'article doit être SUPPRIMÉ (2 échecs atteints) ; incrémente `tentatives_traduction`, pose `statut_traduction='ok'` si réussie.
  - `pub async fn selection_brief_24h(&self, limite: i64) -> anyhow::Result<Vec<PresseArticle>>` — top score des dernières 24 h (sur `ajoute_le`).
  - `pub async fn inserer_brief(&self, fenetre_de: i64, fenetre_a: i64, nb_articles: usize, contenu: &str) -> anyhow::Result<i64>`
  - `pub struct PresseBrief { pub id: i64, pub genere_le: i64, pub fenetre_de: i64, pub fenetre_a: i64, pub nb_articles: i64, pub contenu: String }`
  - `pub async fn lister_briefs(&self, limite: i64) -> anyhow::Result<Vec<PresseBrief>>` (sans `contenu` pour la liste ? Non — garde tout, simple)
  - `pub async fn purger_presse_expiree(&self, mois: i64) -> anyhow::Result<u64>` — supprime articles ET briefs au-delà de N mois.

- [ ] **Step 1: Tests**

```rust
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
        db.supprimer_articles_condamnes().await.unwrap(); // voir Step 3 : suppression immédiate
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
```

- [ ] **Step 2: Run — échec attendu (fonctions absentes)**

Run: `cd backend && cargo test -p db presse 2>&1 | tail -3` → erreurs de compilation.

- [ ] **Step 3: Implémentation (ajouts dans `impl Database` de presse.rs)**

```rust
    /// Machine à états de la traduction (porte d'entrée de la bibliothèque).
    /// Retourne true = 2 échecs atteints → l'appelant doit supprimer l'article.
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

    pub async fn selection_brief_24h(&self, limite: i64) -> anyhow::Result<Vec<PresseArticle>> {
        let cutoff = chrono::Utc::now().timestamp() - 86_400;
        let sql = "SELECT hash_titre, titre, url, source_nom, publie_le, score, theme,
                          assets_concernes, impact, statut_traduction, tentatives_traduction, lu, ajoute_le
                   FROM presse_articles WHERE ajoute_le >= ?1
                   ORDER BY score DESC, ajoute_le DESC LIMIT ?2";
        let lignes = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, String, i64, i64, i64)>(sql)
            .bind(cutoff).bind(limite).fetch_all(self.pool()).await?;
        Ok(lignes.into_iter().map(|l| PresseArticle {
            hash_titre: l.0, titre: l.1, url: l.2, source_nom: l.3, publie_le: l.4,
            score: l.5 as u8, theme: l.6, assets_concernes: l.7, impact: l.8,
            statut_traduction: l.9, tentatives_traduction: l.10 as u8, lu: l.11 != 0, ajoute_le: l.12,
        }).collect())
    }

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

    pub async fn purger_presse_expiree(&self, mois: i64) -> anyhow::Result<u64> {
        if mois <= 0 { return Ok(0); }
        let cutoff = chrono::Utc::now().timestamp() - mois * 30 * 86_400;
        let a = sqlx::query("DELETE FROM presse_articles WHERE ajoute_le < ?1")
            .bind(cutoff).execute(self.pool()).await?.rows_affected();
        let b = sqlx::query("DELETE FROM presse_briefs WHERE genere_le < ?1")
            .bind(cutoff).execute(self.pool()).await?.rows_affected();
        Ok(a + b)
    }
```

Plus la déclaration du struct `PresseBrief` près des autres structs.

- [ ] **Step 4: Run**

`cd backend && cargo test -p db presse 2>&1 | tail -3` → `test result: ok. 7 passed` (3 anciens + 4 nouveaux).

- [ ] **Step 5: Commit**

```bash
git add backend/crates/db/src/presse.rs
git commit -m "feat(presse): db — machine traduction, briefs, rétention"
```

---

### Task 4: news::presse_classif — attribution assets/impact + traitement de flux (pur)

**Files:**
- Create: `backend/crates/news/src/presse_classif.rs`
- Modify: `backend/crates/news/src/lib.rs` (ajouter `pub mod presse_classif;`)

**Interfaces:**
- Consumes: `news_rss::ArticleRss` (champs `titre`, `lien`, `date_rss`), `news_scoring::{scorer, classer_theme, jaccard_bigrammes}`, `news_traduction::hash_titre`.
- Produces (Task 5) :
  - `pub fn assets_concernes(titre_lower: &str) -> Vec<&'static str>`
  - `pub fn impact(score: u8) -> &'static str` — `>=60` "fort", `>=35` "moyen", sinon "faible"
  - `pub struct ArticleCollecte { pub hash_titre: String, pub titre: String, pub url: String, pub source_nom: String, pub publie_le: String, pub score: u8, pub theme: String, pub assets_concernes: String, pub impact: String }`
  - `pub fn traiter_items(items: &[ArticleRss], source_nom: &str, poids_source: u8) -> Vec<ArticleCollecte>` — dédoublonnage jaccard interne + scoring + classification.

- [ ] **Step 1: Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::news_rss::ArticleRss;

    fn rss(titre: &str) -> ArticleRss {
        ArticleRss { titre: titre.into(), lien: "https://x.fr".into(), date_rss: "Sat, 15 Aug 2026 10:00:00 GMT".into() }
    }

    #[test]
    fn assets_par_mots_cles() {
        let a = assets_concernes("bitcoin surges as fed cuts rates, gold slips");
        assert!(a.contains(&"BTC"));
        assert!(a.contains(&"XAUUSD"));
        // "fed"/"rates" → macro sans asset dédié : pas de faux positif
        assert!(!a.contains(&"DAX"));
        let b = assets_concernes("dax rallies with nasdaq");
        assert!(b.contains(&"DAX"));
        assert!(b.contains(&"NAS100"));
    }

    #[test]
    fn impact_par_score() {
        assert_eq!(impact(75), "fort");
        assert_eq!(impact(40), "moyen");
        assert_eq!(impact(10), "faible");
    }

    #[test]
    fn traiter_items_deduplique_et_classe() {
        let items = vec![
            rss("Fed cuts rates: bitcoin surges, gold slips"),
            rss("Fed cuts rates: bitcoin surges, gold slips"), // doublon exact
            rss("Unrelated earnings report from acme corp"),
        ];
        let res = traiter_items(&items, "Reuters Business", 40);
        // Le doublon jaccard ~1.0 est éliminé
        assert_eq!(res.len(), 2);
        let premier = res.iter().find(|a| a.titre.contains("Fed")).unwrap();
        assert_eq!(premier.source_nom, "Reuters Business");
        assert!(premier.score > 0);
        assert!(!premier.hash_titre.is_empty());
        assert!(premier.assets_concernes.contains("BTC"));
    }
}
```

- [ ] **Step 2: Run — échec attendu.** `cd backend && cargo test -p news 2>&1 | tail -3`

- [ ] **Step 3: Implémentation**

```rust
//! Classification presse v1 : attribution d'assets et d'impact par MOTS-CLÉS
//! (gratuit, immédiat — l'affinage LLM reste une évolution post-v1, cf spec).

use crate::news_rss::ArticleRss;
use crate::news_scoring::{classer_theme, jaccard_bigrammes, scorer};
use crate::news_traduction::hash_titre;

/// Mot-clé → asset du pipeline (12 assets). Ordre : spécifique avant générique.
const MOTS_ASSETS: &[(&str, &str)] = &[
    ("bitcoin", "BTC"), ("btc", "BTC"),
    ("ethereum", "ETH"), ("ether", "ETH"),
    ("gold", "XAUUSD"), ("xau", "XAUUSD"),
    ("silver", "XAGUSD"), ("xag", "XAGUSD"),
    ("euro/dollar", "EURUSD"), ("eur/usd", "EURUSD"), ("euro dollar", "EURUSD"),
    ("yen", "USDJPY"), ("japanese yen", "USDJPY"),
    ("dax", "DAX"), ("german index", "DAX"),
    ("nasdaq", "NAS100"), ("nas 100", "NAS100"),
    ("s&p", "SP500"), ("s&p 500", "SP500"),
];

/// Assets du pipeline cités dans le titre (dédoublonnés, ordre stable).
pub fn assets_concernes(titre_lower: &str) -> Vec<&'static str> {
    let mut trouves: Vec<&'static str> = Vec::new();
    for (mot, asset) in MOTS_ASSETS {
        if titre_lower.contains(mot) && !trouves.contains(asset) {
            trouves.push(asset);
        }
    }
    trouves
}

/// Impact dérivé du score (mots-clés × poids source).
pub fn impact(score: u8) -> &'static str {
    if score >= 60 { "fort" } else if score >= 35 { "moyen" } else { "faible" }
}

/// Article prêt pour l'insertion DB (cf db::presse::PresseArticle).
pub struct ArticleCollecte {
    pub hash_titre: String,
    pub titre: String,
    pub url: String,
    pub source_nom: String,
    pub publie_le: String,
    pub score: u8,
    pub theme: String,
    pub assets_concernes: String,
    pub impact: String,
}

/// Traite les items d'UN flux : dédoublonnage interne (jaccard ≥ 0.8),
/// scoring, thème, assets, impact. Fonction pure — aucune I/O.
pub fn traiter_items(items: &[ArticleRss], source_nom: &str, poids_source: u8) -> Vec<ArticleCollecte> {
    let mut retenus: Vec<ArticleCollecte> = Vec::new();
    for item in items {
        let titre_lower = item.titre.to_lowercase();
        // Doublon interne au flux ?
        if retenus.iter().any(|r| jaccard_bigrammes(&r.titre, &item.titre) >= 0.8) {
            continue;
        }
        let score = scorer(&titre_lower, poids_source, &item.date_rss);
        let assets = assets_concernes(&titre_lower);
        retenus.push(ArticleCollecte {
            hash_titre: hash_titre(&item.titre),
            titre: item.titre.clone(),
            url: item.lien.clone(),
            source_nom: source_nom.to_string(),
            publie_le: item.date_rss.clone(),
            score,
            theme: classer_theme(&titre_lower, source_nom).to_string(),
            assets_concernes: serde_json::to_string(&assets).unwrap_or_else(|_| "[]".into()),
            impact: impact(score).to_string(),
        });
    }
    retenus
}
```

- [ ] **Step 4: Run** → `cd backend && cargo test -p news 2>&1 | tail -3` — tous verts (existants + 3).

- [ ] **Step 5: Commit**

```bash
git add backend/crates/news/src/presse_classif.rs backend/crates/news/src/lib.rs
git commit -m "feat(presse): classification par mots-clés + traitement de flux pur"
```

---

### Task 5: Le collecteur — binaire `news_collector`

**Files:**
- Create: `backend/crates/api/src/bin/news_collector.rs`
- Modify: `scripts/run.sh` (lancement du collecteur, hors watchdog)

**Interfaces:**
- Consumes: Tasks 2 (`inserer_articles_presse`, `lister_sources_presse`), 4 (`traiter_items`), `news_rss::fetch_rss`.
- Produces: le process qui remplit la bibliothèque (cycle 30 min).

- [ ] **Step 1: Écrire le binaire**

```rust
//! Collecteur de presse (Phase 4.1) — process SÉPARÉ, crash-isolé (gate 4).
//!
//! Cycle 30 min : lit les sources actives en DB, fetch RSS, traite (dédup +
//! scoring + classification mots-clés — cf news::presse_classif), insère.
//! AUCUNE dépendance Ollama : la traduction/sentiment sont à la demande
//! côté backend. Un cycle qui panique est loggé et sauté.
//!
//! Usage : cargo run -p api --bin news_collector   (DATABASE_PATH requis)

use std::sync::Arc;
use std::time::Duration;

use db::Database;

const CYCLE_SEC: u64 = 1800; // 30 minutes

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(Database::new(&db_path).await?);
    db.run_migrations().await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let http = std::sync::Arc::new(client);

    tracing::info!("📰 Collecteur de presse démarré (cycle {CYCLE_SEC}s)");

    loop {
        if let Err(e) = un_cycle(&db, &http).await {
            tracing::error!("Collecteur : cycle échoué ({e}) — cycle suivant");
        }
        tokio::time::sleep(Duration::from_secs(CYCLE_SEC)).await;
    }
}

async fn un_cycle(db: &Arc<Database>, http: &Arc<reqwest::Client>) -> anyhow::Result<()> {
    let sources = db.lister_sources_presse(true).await?;
    let mut total_inserees = 0u64;
    for source in &sources {
        let items = news::news_rss::fetch_rss(http, &source.url_rss).await;
        if items.is_empty() {
            tracing::debug!("Collecteur : flux vide ou down — {}", source.nom);
            continue;
        }
        let articles = news::presse_classif::traiter_items(&items, &source.nom, source.poids_score);
        let entrants: Vec<db::presse::ArticleEntrant> = articles.iter().map(|a| db::presse::ArticleEntrant {
            hash_titre: a.hash_titre.clone(), titre: a.titre.clone(), url: a.url.clone(),
            source_nom: a.source_nom.clone(), publie_le: a.publie_le.clone(),
            score: a.score, theme: a.theme.clone(),
            assets_concernes: a.assets_concernes.clone(), impact: a.impact.clone(),
        }).collect();
        let inserees = db.inserer_articles_presse_convertis(&entrants).await?;
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
```

- [ ] **Step 2: Pont ArticleCollecte → PresseArticle dans db::presse**

Le binaire appelle `inserer_articles_presse_convertis` (conversion du type news vers le type db). Ajouter dans `db/src/presse.rs` (le crate db ne dépend pas de news — le pont se fait par un type témoin) :

```rust
/// Article brut fourni par le collecteur (découplé du crate news).
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
}

impl Database {
    pub async fn inserer_articles_presse_convertis(&self, articles: &[crate::presse::ArticleEntrant]) -> anyhow::Result<u64> {
        let convertis: Vec<PresseArticle> = articles.iter().map(|a| PresseArticle {
            hash_titre: a.hash_titre.clone(), titre: a.titre.clone(), url: a.url.clone(),
            source_nom: a.source_nom.clone(), publie_le: a.publie_le.clone(),
            score: a.score, theme: a.theme.clone(), assets_concernes: a.assets_concernes.clone(),
            impact: a.impact.clone(), statut_traduction: "non_tente".into(),
            tentatives_traduction: 0, lu: false, ajoute_le: 0, // posé par le INSERT
        }).collect();
        self.inserer_articles_presse(&convertis).await
    }
}
```

Et dans le binaire, mapper `ArticleCollecte` → `db::presse::ArticleEntrant` avant l'appel (code trivial, à écrire avec le binaire).

- [ ] **Step 3: Compiler**

`cd backend && cargo build --bin news_collector 2>&1 | tail -2` → sans erreur.

- [ ] **Step 4: Test manuel sur DB temporaire (réseau réel autorisé ici, hors cargo test)**

```bash
rm -f /tmp/presse.db && DATABASE_PATH=/tmp/presse.db timeout 40 \
  LIBTORCH=/mnt/IA/libtorch XGBOOST_LIB_DIR=/home/rono/.local/lib/python3.14/site-packages/xgboost/lib \
  LD_LIBRARY_PATH=/mnt/IA/libtorch/lib:/home/rono/.local/lib/python3.14/site-packages/xgboost/lib:/run/host/usr/lib64 \
  ./target/debug/news_collector 2>&1 | grep -E "Collecteur" | head -12
sqlite3 /tmp/presse.db "SELECT COUNT(*), MIN(theme), MAX(score) FROM presse_articles;"
```
Expected: lignes "Collecteur : <source> — N items → M articles insérés" et un COUNT > 0 (selon la disponibilité des flux).

- [ ] **Step 5: run.sh — lancement hors watchdog**

Dans `scripts/run.sh`, juste après le démarrage du backend (bloc `echo "🔌 Backend API → port 8080"`), ajouter :

```bash
# ── Collecteur de presse (process séparé, hors watchdog : sa mort n'arrête
# rien — gate 4. Rejoint la flotte des producteurs isolés.)
echo "📰 Collecteur de presse"
DATABASE_PATH="$ROOT_DIR/data/trading.db" \
  "$ROOT_DIR/backend/target/release/news_collector" \
  > "$LOG_DIR/news_collector.log" 2>&1 &
COLLECTOR_PID=$!
```

(Ne PAS ajouter COLLECTOR_PID au `cleanup` ni au watchdog — c'est le sens de la gate 4. Un `pkill -x news_collector` avant lancement évite les doublons, selon la règle L1 : par nom exact.)

- [ ] **Step 6: Commit**

```bash
git add backend/crates/api/src/bin/news_collector.rs backend/crates/db/src/presse.rs scripts/run.sh
git commit -m "feat(presse): collecteur séparé 30 min + lancement run.sh"
```

---

### Task 6: Traduction stricte + brief — extensions du crate news

**Files:**
- Modify: `backend/crates/news/src/news_traduction.rs`

**Interfaces:**
- Consumes: comportement existant (`traduire` rend l'original si échec — `news_traduction.rs:70-80`).
- Produces (Tasks 7, 8) :
  - `pub async fn traduire_avec_cache_strict(pool: &SqlitePool, titre: &str) -> Option<String>` — cache d'abord ; sinon Ollama ; **ne cache que les succès** (None = échec).
  - `pub async fn generer_brief_llm(entree: &str) -> Option<String>` — un appel Ollama, entrée = titres+scores compilés, sortie = markdown.

- [ ] **Step 1: Test du pur (évaluation succès/échec)** — extraire d'abord une fonction pure :

Dans news_traduction.rs, ajouter :

```rust
/// Une traduction est réussie si elle diffère de l'original (contrat :
/// `traduire` rend le texte original en cas d'échec).
pub fn traduction_reussie(original: &str, traduit: &str) -> bool {
    !traduit.trim().is_empty() && traduit.trim() != original.trim()
}

#[cfg(test)]
mod tests_strict {
    use super::*;

    #[test]
    fn eval_traduction() {
        assert!(traduction_reussie("Fed cuts rates", "La Fed baisse ses taux"));
        assert!(!traduction_reussie("Fed cuts rates", "Fed cuts rates"));
        assert!(!traduction_reussie("Fed cuts rates", "  "));
    }
}
```

- [ ] **Step 2: Run** → `cargo test -p news 2>&1 | tail -3` — vert.

- [ ] **Step 3: Les deux fonctions async (non testées — réseau Ollama)**

```rust
/// Traduction STRICTE pour la presse : None = échec (rien n'est caché).
/// Diffère de `traduire_avec_cache` qui cache aussi les échecs (rend
/// l'original) — inacceptable pour la règle « porte d'entrée ».
pub async fn traduire_avec_cache_strict(pool: &SqlitePool, titre: &str) -> Option<String> {
    let hash = hash_titre(titre);
    if let Some(cached) = lire_cache(pool, &hash).await {
        return Some(cached);
    }
    let traduit = traduire(titre).await;
    if traduction_reussie(titre, &traduit) {
        ecrire_cache(pool, &hash, &traduit).await;
        Some(traduit)
    } else {
        None
    }
}

/// Génération du brief : un appel Ollama, entrée compilée par l'appelant
/// (titres FR + scores + thèmes), sortie markdown brute.
pub async fn generer_brief_llm(entree: &str) -> Option<String> {
    let prompt = format!(
        "Tu es analyste financier. Rédige en français un brief matinal en markdown :\n\
         ## Contexte marché\n3 lignes sur les thèmes dominants.\n\n\
         ## Articles marquants\nPour chaque article : 2-3 lignes (impact, assets).\n\n\
         Articles des dernières 24h (score sur 100, thème) :\n{entree}"
    );
    let corps = serde_json::json!({
        "model": MODELE_TRADUCTION,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    });
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());
    let _permit = llm::OLLAMA_SEMAPHORE.acquire().await.ok();
    let res = llm::OLLAMA_HTTP_CLIENT.post(&url).json(&corps).send().await;
    match res {
        Ok(r) if r.status().is_success() => r
            .json::<llm::ReponseOllama>().await.ok()
            .map(|r| r.message.content.trim().to_string())
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}
```

- [ ] **Step 4: Compiler + tests**

`cd backend && cargo test -p news 2>&1 | tail -3` → verts.

- [ ] **Step 5: Commit**

```bash
git add backend/crates/news/src/news_traduction.rs
git commit -m "feat(presse): traduction stricte (cache succès uniquement) + brief LLM"
```

---

### Task 7: API `/api/presse` — articles, ouvrir, sources

**Files:**
- Create: `backend/crates/api/src/presse_handlers.rs`
- Modify: `backend/crates/api/src/main.rs` (`mod presse_handlers;`), `backend/crates/api/src/routes.rs`

**Interfaces:**
- Consumes: Tasks 2, 3 (db), 6 (traduction stricte).
- Produces (Task 9 UI) :
  - `GET /api/presse/articles?theme=&asset=&source=&q=&lu=&page=` → `{articles: [...], total_page: N}`
  - `POST /api/presse/articles/{hash}/ouvrir` → `{titre, titre_fr: Option, sentiment: Option, article}` — traduction lazy (cache → Ollama strict → 2 échecs = suppression), sentiment si titre FR dispo, marquage lu
  - `GET /api/presse/sources` / `POST /api/presse/sources {nom, url_rss, poids, categorie}` / `DELETE /api/presse/sources/{id}`

- [ ] **Step 1: Handlers**

```rust
//! Endpoints de la revue de presse (Phase 4.1).

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::state::AppState;
use db::presse::{FiltreArticles, PresseSource};

pub async fn get_articles(state: web::Data<AppState>, q: web::Query<std::collections::HashMap<String, String>>) -> HttpResponse {
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
    let Some(mut article) = match state.db.lire_article_presse(&hash).await {
        Ok(a) => a,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
    } else {
        return HttpResponse::NotFound().json(serde_json::json!({"erreur": "article inconnu"}));
    };

    // Traduction : cache → Ollama strict → machine à états (2 échecs = suppression).
    let titre_fr = if article.statut_traduction == "ok" {
        news::news_traduction::traduire_avec_cache_strict(&pool, &article.titre).await
    } else {
        match news::news_traduction::traduire_avec_cache_strict(&pool, &article.titre).await {
            Some(t) => {
                let _ = state.db.enregistrer_tentative_traduction(&hash, true).await;
                article.statut_traduction = "ok".into();
                Some(t)
            }
            None => {
                let condamne = state.db.enregistrer_tentative_traduction(&hash, false).await.unwrap_or(false);
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
    } else { None };

    let _ = state.db.marquer_lu_presse(&hash).await;
    article.lu = true;
    HttpResponse::Ok().json(serde_json::json!({
        "article": article, "titre_fr": titre_fr, "sentiment": sentiment,
    }))
}

pub async fn get_sources(state: web::Data<AppState>) -> HttpResponse {
    match state.db.lister_sources_presse(false).await {
        Ok(sources: Vec<PresseSource>) => HttpResponse::Ok().json(sources),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

#[derive(Deserialize)]
pub struct CorpsSource { pub nom: String, pub url_rss: String, pub poids: Option<u8>, pub categorie: Option<String> }

pub async fn post_source(state: web::Data<AppState>, corps: web::Json<CorpsSource>) -> HttpResponse {
    if corps.nom.trim().is_empty() || !corps.url_rss.starts_with("https://") {
        return HttpResponse::BadRequest().json(serde_json::json!({"erreur": "nom requis, URL https:// requise"}));
    }
    match state.db.ajouter_source_presse(corps.nom.trim(), corps.url_rss.trim(),
        corps.poids.unwrap_or(30).min(50), corps.categorie.as_deref().unwrap_or("marches")).await
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
```

Note : `let Some(mut article) = ... else` avec match/Ok — écrire la forme compile-clean (deux étapes : `let lu = ...; let Some(article) = lu else {...}`) selon le compilateur ; l'intention prime.

- [ ] **Step 2: Routes** (routes.rs, après le bloc runtime) :

```rust
        // ── Revue de presse (Phase 4.1) ────────────────────────────────────────
        .route("/api/presse/articles", web::get().to(crate::presse_handlers::get_articles))
        .route("/api/presse/articles/{hash}/ouvrir", web::post().to(crate::presse_handlers::ouvrir_article))
        .route("/api/presse/sources", web::get().to(crate::presse_handlers::get_sources))
        .route("/api/presse/sources", web::post().to(crate::presse_handlers::post_source))
        .route("/api/presse/sources/{id}", web::delete().to(crate::presse_handlers::delete_source))
```

Et `mod presse_handlers;` dans main.rs (ordre alphabétique, après `mod prealerte_handlers;`).

- [ ] **Step 3: Compiler + tests workspace**

`cd backend && cargo check --workspace 2>&1 | grep -cE "^error"` → 0.

- [ ] **Step 4: Test manuel de bout en bout (backend local, DB de prod, SANS Ollama obligatoire)**

```bash
# injecter un article de test puis l'ouvrir deux fois sans Ollama → 2e = 410 Gone
sqlite3 data/trading.db "INSERT OR IGNORE INTO presse_articles (hash_titre,titre,url,source_nom,publie_le,score,theme,assets_concernes,impact,ajoute_le) VALUES ('test1','Fed Test Title Unique','https://x.fr','Test','x',45,'macro','[]','moyen',strftime('%s','now'));"
curl -s -X POST http://localhost:8080/api/presse/articles/test1/ouvrir | head -c 200; echo
curl -s -X POST http://localhost:8080/api/presse/articles/test1/ouvrir | head -c 200; echo  # → Gone si Ollama down
sqlite3 data/trading.db "SELECT COUNT(*) FROM presse_articles WHERE hash_titre='test1';"  # → 0
```

- [ ] **Step 5: Commit**

```bash
git add backend/crates/api/src/presse_handlers.rs backend/crates/api/src/main.rs backend/crates/api/src/routes.rs
git commit -m "feat(presse): API articles/ouvrir/sources — traduction porte d'entrée"
```

---

### Task 8: API brief + rétention presse

**Files:**
- Modify: `backend/crates/api/src/presse_handlers.rs` (briefs), `backend/crates/api/src/retention_job.rs`, `backend/crates/db/src/presse.rs` (lecture rétention presse)

**Interfaces:**
- Consumes: Task 3 (`selection_brief_24h`, `inserer_brief`, `lister_briefs`), Task 6 (`generer_brief_llm`, `traduire_avec_cache_strict`).
- Produces: `POST /api/presse/brief` → `{id, contenu}` ; `GET /api/presse/briefs` ; `GET /api/presse/briefs/{id}` ; purge presse dans le job rétention.

- [ ] **Step 1: Handlers brief**

```rust
pub async fn post_brief(state: web::Data<AppState>) -> HttpResponse {
    let articles = match state.db.selection_brief_24h(15).await {
        Ok(a) => a,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
    };
    if articles.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"erreur": "aucun article dans les dernières 24 h"}));
    }
    // Traduire à la volée les articles du brief qui ne le sont pas (seuls eux).
    let pool: SqlitePool = state.db.pool().clone();
    let mut entree = String::new();
    for a in &articles {
        let fr = if a.statut_traduction == "ok" {
            news::news_traduction::traduire_avec_cache_strict(&pool, &a.titre).await
                .unwrap_or_else(|| a.titre.clone())
        } else {
            match news::news_traduction::traduire_avec_cache_strict(&pool, &a.titre).await {
                Some(t) => { let _ = state.db.enregistrer_tentative_traduction(&a.hash_titre, true).await; t }
                None => a.titre.clone(), // VO dans le brief — pas de suppression ici (porte d'entrée = consultation)
            }
        };
        entree.push_str(&format!("- [{:3}/100|{}] {} ({})\n", a.score, a.theme, fr, a.source_nom));
    }
    let Some(contenu) = news::news_traduction::generer_brief_llm(&entree).await else {
        return HttpResponse::ServiceUnavailable().json(
            serde_json::json!({"erreur": "Ollama indisponible — réessayer plus tard"})
        );
    };
    let maintenant = chrono::Utc::now().timestamp();
    let id = match state.db.inserer_brief(maintenant - 86_400, maintenant, articles.len(), &contenu).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
    };
    HttpResponse::Ok().json(serde_json::json!({"id": id, "contenu": contenu, "nb_articles": articles.len()}))
}

pub async fn get_briefs(state: web::Data<AppState>) -> HttpResponse {
    match state.db.lister_briefs(20).await {
        Ok(briefs) => HttpResponse::Ok().json(briefs),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

pub async fn get_brief(state: web::Data<AppState>, chemin: web::Path<i64>) -> HttpResponse {
    match state.db.lister_briefs(1000).await {
        Ok(briefs) => match briefs.into_iter().find(|b| b.id == *chemin) {
            Some(b) => HttpResponse::Ok().json(b),
            None => HttpResponse::NotFound().body("brief inconnu"),
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}
```

Routes (ajoutées aux précédentes) :

```rust
        .route("/api/presse/brief", web::post().to(crate::presse_handlers::post_brief))
        .route("/api/presse/briefs", web::get().to(crate::presse_handlers::get_briefs))
        .route("/api/presse/briefs/{id}", web::get().to(crate::presse_handlers::get_brief))
```

- [ ] **Step 2: Rétention — dans `retention_job.rs` passer()**

Après `purger_emissions_expiree` :

```rust
    // 4. Presse (mois) — clé retention_presse_mois, défaut 12.
    let mois_presse: i64 = state_lue_presse; // lire via db.lire_config("retention_presse_mois") → 12
```
Concrètement : ajouter dans `db::presse` :
```rust
    pub async fn lire_retention_presse(&self) -> i64 {
        match self.lire_config("retention_presse_mois").await {
            Ok(Some(v)) => v.trim().parse().unwrap_or(12),
            _ => 12,
        }
    }
```
et dans `retention_job.rs::passer()` :
```rust
    let presse_supprimees = db.purger_presse_expiree(db.lire_retention_presse().await).await?;
```
(+ intégrer `presse_supprimees` au total du VACUUM conditionnel et au log de fin.)

- [ ] **Step 3: Tests db** — couverts au Task 3 (`purger_presse_expiree`, `lire_retention` trivial). Compiler + `cargo test -p db -p api --lib` → verts.

- [ ] **Step 4: Test manuel du brief (Ollama requis)**

```bash
curl -s -X POST http://localhost:8080/api/presse/brief --max-time 120 | head -c 400
```
Expected: JSON `{id, contenu: "## Contexte marché\n..."}` ou 503 propre si Ollama down.

- [ ] **Step 5: Commit**

```bash
git add backend/crates/api/src/presse_handlers.rs backend/crates/api/src/routes.rs backend/crates/api/src/retention_job.rs backend/crates/db/src/presse.rs
git commit -m "feat(presse): brief 24h à la demande + rétention intégrée"
```

---

### Task 9: UI — vue Revue de presse + extrait dashboard

**Files:**
- Create: `frontend/src/views/PresseView.vue`, `frontend/src/services/api.presse.ts`
- Modify: `frontend/src/router/index.ts`, `frontend/src/components/common/SideBar.vue` (groupe Presse), `frontend/src/services/api.service.ts` (spread presseApi), `frontend/src/components/common/NewsFeed.vue` (bouton « Tout voir » → `/presse`)

**Interfaces:**
- Consumes: API du Tasks 7-8.
- Produces: la vue `/presse`.

- [ ] **Step 1: Service `api.presse.ts`**

```typescript
import { http } from './http.client'

export interface ArticlePresse {
  hash_titre: string; titre: string; url: string; source_nom: string
  publie_le: string; score: number; theme: string; assets_concernes: string
  impact: string; statut_traduction: string; lu: boolean; ajoute_le: number
}
export interface BriefPresse {
  id: number; genere_le: number; fenetre_de: number; fenetre_a: number
  nb_articles: number; contenu: string
}

export const presseApi = {
  async articles(filtres: Partial<{ theme: string; asset: string; source: string; q: string; lu: string; page: number }> = {}): Promise<ArticlePresse[]> {
    const res = await http.get('/api/presse/articles', { params: filtres })
    return res.data.articles
  },
  async ouvrir(hash: string): Promise<{ article: ArticlePresse; titre_fr: string | null; sentiment: string | null }> {
    const res = await http.post(`/api/presse/articles/${hash}/ouvrir`, null, { timeout: 60_000 })
    return res.data
  },
  async genererBrief(): Promise<{ id: number; contenu: string; nb_articles: number }> {
    const res = await http.post('/api/presse/brief', null, { timeout: 180_000 })
    return res.data
  },
  async briefs(): Promise<BriefPresse[]> {
    const res = await http.get('/api/presse/briefs')
    return res.data
  },
  async sources(): Promise<{ id: number; nom: string; url_rss: string; poids_score: number; categorie: string; actif: boolean }[]> {
    const res = await http.get('/api/presse/sources')
    return res.data
  },
  async ajouterSource(nom: string, url: string, poids: number, categorie: string): Promise<void> {
    await http.post('/api/presse/sources', { nom, url_rss: url, poids, categorie })
  },
  async retirerSource(id: number): Promise<void> {
    await http.delete(`/api/presse/sources/${id}`)
  },
}
```

- [ ] **Step 2: Vue `PresseView.vue`** — quatre sections, style glass-card du projet :

```vue
<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">📰 Revue de presse</h1>

    <!-- Brief -->
    <div class="glass-card p-5 space-y-3">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Brief 24 h</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 disabled:opacity-40"
          :disabled="enBrief" @click="genererBrief()"
        >{{ enBrief ? '⏳ Génération…' : '⚡ Générer le brief' }}</button>
      </div>
      <div v-if="dernierBrief" class="text-sm text-gray-200 whitespace-pre-line">{{ dernierBrief.contenu }}</div>
      <p v-else class="text-sm text-gray-500">Aucun brief — clique « Générer » (Ollama, ~1 min).</p>
    </div>

    <!-- Filtres -->
    <div class="glass-card p-4 flex flex-wrap gap-3 items-center">
      <input v-model="filtre.q" placeholder="Recherche…" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" @keyup.enter="charger()" />
      <select v-model="filtre.theme" class="bg-white text-black rounded-lg px-2 py-1.5 text-sm" @change="charger()">
        <option value="">Tous thèmes</option>
        <option v-for="t in themes" :key="t" :value="t">{{ t }}</option>
      </select>
      <select v-model="filtre.asset" class="bg-white text-black rounded-lg px-2 py-1.5 text-sm" @change="charger()">
        <option value="">Tous assets</option>
        <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
      </select>
      <select v-model="filtre.lu" class="bg-white text-black rounded-lg px-2 py-1.5 text-sm" @change="charger()">
        <option value="">Lu + non lus</option><option value="true">Non lus</option><option value="false">Lus</option>
      </select>
      <span class="text-xs text-gray-500">{{ articles.length }} articles</span>
    </div>

    <!-- Bibliothèque -->
    <div class="glass-card p-2 divide-y divide-white/5">
      <button v-for="a in articles" :key="a.hash_titre" class="w-full text-left px-3 py-2.5 hover:bg-white/5 transition" @click="ouvrir(a)">
        <div class="flex items-center justify-between gap-3">
          <span class="text-sm" :class="a.lu ? 'text-gray-500' : 'text-white font-medium'">{{ a.titre }}</span>
          <span class="text-xs text-gray-500 shrink-0">{{ a.source_nom }}</span>
        </div>
        <div class="flex gap-2 mt-1 text-[10px]">
          <span class="px-1.5 py-0.5 rounded" :class="a.impact === 'fort' ? 'bg-red-500/15 text-red-300' : a.impact === 'moyen' ? 'bg-yellow-500/15 text-yellow-300' : 'bg-white/10 text-gray-400'">{{ a.impact }}</span>
          <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ a.theme }}</span>
          <span v-for="asset in parseAssets(a)" :key="asset" class="px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-300">{{ asset }}</span>
        </div>
      </button>
      <p v-if="articles.length === 0" class="text-sm text-gray-500 p-4">Bibliothèque vide — le collecteur remplit au prochain cycle (30 min).</p>
    </div>

    <!-- Sources -->
    <div class="glass-card p-5 space-y-3">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Sources RSS</h2>
      <div v-for="s in sources" :key="s.id" class="flex items-center justify-between text-sm">
        <span :class="s.actif ? 'text-gray-300' : 'text-gray-600 line-through'">{{ s.nom }} <span class="text-xs text-gray-500">(poids {{ s.poids_score }})</span></span>
        <button class="text-red-400 hover:text-red-300 text-xs" @click="retirerSource(s.id)">Retirer</button>
      </div>
      <div class="flex gap-2 pt-2 border-t border-white/5">
        <input v-model="nouvelleSource.nom" placeholder="Nom" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
        <input v-model="nouvelleSource.url" placeholder="https://flux.example/rss" class="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
        <button class="px-3 py-1.5 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm" @click="ajouterSource()">+ Ajouter</button>
      </div>
    </div>

    <!-- Modal article (opaque) -->
    <div v-if="articleOuvert" class="fixed inset-0 z-50 flex items-center justify-center bg-black/30" @click.self="articleOuvert = null">
      <div class="w-full max-w-lg p-6 space-y-3 rounded-2xl border border-white/10 bg-[#16181d] shadow-2xl">
        <h3 class="font-bold text-white">{{ articleOuvert.titre_fr || articleOuvert.article.titre }}</h3>
        <p v-if="!articleOuvert.titre_fr" class="text-xs text-yellow-400">Traduction indisponible (réessai à la prochaine ouverture)</p>
        <p v-if="articleOuvert.sentiment" class="text-sm text-gray-400">Sentiment : {{ articleOuvert.sentiment }}</p>
        <a :href="articleOuvert.article.url" target="_blank" class="text-sm text-blue-400 hover:underline">Lire l'article source ↗</a>
        <div class="flex justify-end"><button class="px-4 py-2 rounded-lg bg-white/5 text-gray-300 text-sm" @click="articleOuvert = null">Fermer</button></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { presseApi, type ArticlePresse } from '@/services/api.presse'

const articles = ref<ArticlePresse[]>([])
const sources = ref<Awaited<ReturnType<typeof presseApi.sources>>>([])
const enBrief = ref(false)
const dernierBrief = ref<Awaited<ReturnType<typeof presseApi.briefs>>[number] | null>(null)
const articleOuvert = ref<{ article: ArticlePresse; titre_fr: string | null; sentiment: string | null } | null>(null)
const filtre = reactive({ q: '', theme: '', asset: '', lu: '' })
const nouvelleSource = reactive({ nom: '', url: '' })
const themes = ['macro', 'crypto', 'metaux', 'forex', 'marches']
const assets = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'USDJPY', 'DAX', 'NAS100', 'SP500']

function parseAssets(a: ArticlePresse): string[] {
  try { return JSON.parse(a.assets_concernes) } catch { return [] }
}

async function charger() {
  articles.value = await presseApi.articles({
    q: filtre.q || undefined, theme: filtre.theme || undefined,
    asset: filtre.asset || undefined, lu: filtre.lu || undefined,
  })
}

async function ouvrir(a: ArticlePresse) {
  articleOuvert.value = await presseApi.ouvrir(a.hash_titre)
  await charger() // rafraîchir lu/badges
}

async function genererBrief() {
  enBrief.value = true
  try {
    await presseApi.genererBrief()
    dernierBrief.value = (await presseApi.briefs())[0] ?? null
  } finally { enBrief.value = false }
}

async function ajouterSource() {
  if (!nouvelleSource.nom || !nouvelleSource.url.startsWith('https://')) return
  await presseApi.ajouterSource(nouvelleSource.nom, nouvelleSource.url, 30, 'marches')
  nouvelleSource.nom = ''; nouvelleSource.url = ''
  sources.value = await presseApi.sources()
}

async function retirerSource(id: number) {
  await presseApi.retirerSource(id)
  sources.value = await presseApi.sources()
}

onMounted(async () => {
  await charger()
  sources.value = await presseApi.sources()
  dernierBrief.value = (await presseApi.briefs())[0] ?? null
})
</script>
```

- [ ] **Step 3: Router + sidebar** :

```typescript
// router/index.ts
{ path: '/presse', component: () => import('../views/PresseView.vue') },
```
```typescript
// SideBar.vue — après le groupe Outils IA (ou avant, selon goût) :
  {
    groupe: 'Presse', icone: '📰',
    liens: [
      { to: '/presse', icone: '📚', label: 'Bibliothèque' },
    ]
  },
```

- [ ] **Step 4: Dashboard — extrait** : dans NewsFeed.vue, en pied de panneau : `<RouterLink to="/presse" class="text-xs text-blue-400 hover:underline">Tout voir →</RouterLink>`.

- [ ] **Step 5: Build**

`cd frontend && npm run build 2>&1 | tail -2` → `✓ built`.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/views/PresseView.vue frontend/src/services/api.presse.ts frontend/src/router/index.ts frontend/src/components/common/SideBar.vue frontend/src/services/api.service.ts frontend/src/components/common/NewsFeed.vue
git commit -m "feat(presse): vue bibliothèque + brief + extrait dashboard"
```

---

### Task 10: Vérification finale — isolation gate 4 + journalisation roadmap

**Files:**
- Modify: `docs/ROADMAP.md` (journal)

- [ ] **Step 1: Test d'isolation (le kill -9)** — app lancée via run.sh :

```bash
pkill -9 -x news_collector
curl -s http://localhost:8080/health           # → ok
curl -s "http://localhost:8080/api/runtime/concordance?heures=1" | head -c 120  # → conforme
```
Expected: l'app ne s'aperçoit de rien (health ok, runtime intact) — la gate 4 est prouvée.

- [ ] **Step 2: Test de dégradation Ollama** : `systemctl --user stop ollama` (ou couper le process), cliquer un article → VO affichée, 2e ouverture ×2 articles → supprimés (410) ; bouton brief → 503 propre. Rétablir Ollama.

- [ ] **Step 3: Journal ROADMAP** — entrée datée résumant la livraison + les preuves (isolation, dégradation, compteurs).

- [ ] **Step 4: Commit final**

```bash
git add docs/ROADMAP.md
git commit -m "docs(presse): journal — livraison phase 4.1, preuves gate 4"
```
