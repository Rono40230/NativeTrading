# Phase 1.6c — Extraction crate `news` — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Extraire la logique news (4 fichiers purs) du monolithe `api` vers un crate `news`. Petit bloc, débloqué par l'extraction llm (1.6b) puisque `news_traduction` utilise `llm::`.

**Architecture:** Nouveau crate `news` = `news_rss` + `news_scraper` + `news_scoring` + `news_traduction`. Les 3 premiers sont **purs** (reqwest/serde/chrono, 0 dépendance api). `news_traduction` dépend de `llm`. Seul consommateur : `news_handlers.rs` (api). `news_fear_greed.rs` est un handler → reste dans api.

## Global Constraints
- Zéro panic Vibe ; limite 600 ; **test = 119** (invariant) + `cargo build -p api -p news`.
- Commit local ; push = propriétaire ; l'assistant pose ses questions et attend.

### Build env CUDA (voir plans précédents).

---

## Task 0: Branche + baseline
- [ ] Branche `phase1-6c-news` DÉJÀ CRÉÉE (baseline 119).

## Task 1: Créer le crate `news`
- [ ] `backend/crates/news/Cargo.toml` — deps : `reqwest`, `serde`, `serde_json`, `chrono`, `llm = { path = "../llm" }` (+ `tokio`, `tracing` si utilisés — vérifier les `use`). Versions `{ workspace = true }`. (Si `news_scoring` ou un autre utilise `db`/`common`, ajouter — build le révélera.)
- [ ] `backend/crates/news/src/lib.rs` :
```rust
//! Crate `news` — logique de veille presse (RSS, scraping, scoring, traduction/sentiment LLM).
//! Extrait du monolithe api (1.6c). Dépend de `llm` pour la traduction/sentiment.
pub mod news_rss;
pub mod news_scraper;
pub mod news_scoring;
pub mod news_traduction;
```
- [ ] `backend/Cargo.toml` : ajouter `"crates/news"` aux members.

## Task 2: Déplacer les 4 fichiers
- [ ] `git mv` des 4 fichiers de `api/src/` vers `news/src/`.
- [ ] Dans `news_traduction.rs` : `use llm::ollama;` reste valide (news dépend de llm). Aucun changement d'import intra (les 4 n'ont pas de `use crate::`).

## Task 3: Mettre à jour `api`
- [ ] `api/Cargo.toml` : ajouter `news = { path = "../news" }`.
- [ ] `main.rs` : retirer `mod news_rss; mod news_scraper; mod news_scoring; mod news_traduction;` (garder `mod news_handlers;`, `mod news_context_handler;`, `mod news_lus_handlers;`, `mod news_fear_greed;`).
- [ ] `news_handlers.rs` : migrer `use crate::news_rss::...` / `crate::news_scraper::...` / `crate::news_scoring::...` / `crate::news_traduction::...` → `use news::...` (ou `news::news_rss::...`). Le build guide.

## Task 4: Build + tests + commit
- [ ] `cargo build -p news` OK ; `cargo build -p api` OK ; `cargo test --workspace` = **119**.
- [ ] `grep -rn "crate::news_rss\|crate::news_scraper\|crate::news_scoring\|crate::news_traduction" backend/crates/api/src/` → vide.
- [ ] Commit : `refactor: extrait le crate news (RSS/scraper/scoring/traduction) du monolithe api`

## Task 5: Validation
- [ ] `cargo build --release` OK. Vérif runtime (reportée) : page News / revue de presse fonctionne (RSS + traduction + sentiment via llm).
- [ ] Rapport au propriétaire. Ne pas pousser.

## Self-Review
**Risques** : très faibles (3 fichiers purs + 1 dépendant de llm déjà migré). Seul piège : une dépendance cachée (db/common) dans un des 4 → build le révèle. Invariant 119 solide (aucun test dans ces modules).
