# Phase 1.6b — Extraction crate `llm` — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Extraire l'intégration LLM (Ollama + Anthropic, ~17 fichiers) du monolithe `api` vers un nouveau crate `llm`. C'est le **gros bloc** du démonolithage (isolation propre, aucun cycle), qui débloque 1.6c (news).

**Architecture:** Nouveau crate `llm` contenant le sous-module `ollama/` (15 fichiers, structure préservée) + `anthropic.rs` + `prompt_effectif` (déplacée de `prompts_handler.rs`). `ollama_types.rs` **reste dans `api`** (DTOs de transport HTTP, utilisé par 3 handlers, pas de dépendance depuis `ollama/`). ~18 fichiers d'`api` migrent `crate::ollama::` → `llm::`.

**Tech Stack:** Rust workspace, reqwest, tokio (LazyLock + Semaphore).

## Global Constraints

- **Zéro panic Vibe** ; limite 600 lignes ; **test avant validation** : `cargo test --workspace` = **119** (invariant) + `cargo build -p api -p llm`.
- **Commit local par tâche** ; **push = propriétaire**.
- **L'assistant pose ses questions et attend la réponse.**

### Build backend — env CUDA (avant `cargo`)
(voir plans précédents — LIBTORCH/XGBOOST/LIBCLANG/RUSTFLAGS/CC=clang)

---

## File Structure

| Action | Chemin |
|--------|--------|
| **Créer** | `backend/crates/llm/Cargo.toml` |
| **Créer** | `backend/crates/llm/src/lib.rs` (pub mod ollama; + anthropic; + prompt_effectif + re-exports) |
| **Déplacer** | `backend/crates/api/src/ollama/` (15 fichiers) → `backend/crates/llm/src/ollama/` |
| **Déplacer** | `backend/crates/api/src/anthropic.rs` → `backend/crates/llm/src/anthropic.rs` |
| **Déplacer** | fonction `prompt_effectif` (+ `OVERRIDES_PATH`, lecture fichier) de `api/src/prompts_handler.rs` → `llm/src/` (module `prompts` ou dans lib.rs) |
| **Garde dans api** | `ollama_types.rs` (transport DTOs) |
| **Modifier** | `backend/Cargo.toml` (workspace + `llm`) |
| **Modifier** | `backend/crates/api/Cargo.toml` (`llm = { path = "../llm" }`) |
| **Modifier** | `backend/crates/api/src/main.rs` (retirer `mod ollama;` `mod anthropic;`) |
| **Modifier** | ~18 fichiers api : `crate::ollama::` → `llm::` (+ `crate::prompts_handler::prompt_effectif` → `llm::prompt_effectif`) |

---

## Task 0: Branche + baseline
- [ ] Branche `phase1-6b-llm` DÉJÀ CRÉÉE (baseline 119).

## Task 1: Créer le crate `llm`

### 1.1 Cargo.toml
- [ ] `backend/crates/llm/Cargo.toml` — deps (lire les `use` des fichiers déplacés) : `common`, `reqwest`, `tokio`, `tracing`, `serde`, `serde_json`, `chrono`, `regex` (si utilisé), `db` (si ollama/ lit la DB — vérifier). Versions `{ workspace = true }`.

### 1.2 lib.rs
- [ ] `backend/crates/llm/src/lib.rs` :
```rust
//! Crate `llm` — intégration LLM (Ollama + Anthropic). Extrait du monolithe api (1.6b).
pub mod ollama;
pub mod anthropic;
pub mod prompts; // contient prompt_effectif (déplacée)

// Re-exports pour API publique ergonomique : llm::OLLAMA_HTTP_CLIENT, llm::filtrer_think, etc.
pub use ollama::*;
pub use prompts::prompt_effectif;
```

### 1.3 Workspace member
- [ ] `backend/Cargo.toml` : ajouter `"crates/llm"` aux members.

---

## Task 2: Déplacer les fichiers

- [ ] `git mv backend/crates/api/src/ollama backend/crates/llm/src/ollama`
- [ ] `git mv backend/crates/api/src/anthropic.rs backend/crates/llm/src/anthropic.rs`
- [ ] Créer `backend/crates/llm/src/prompts.rs` avec `prompt_effectif` (déplacée de `prompts_handler.rs`) + `OVERRIDES_PATH` + la lecture `std::fs`. **Garder aussi les defaults** (si prompt_effectif fallback vers des constantes de prompts qui sont dans `ollama/prompts.rs`, c'est désormais intra-llm — OK).

### 2.1 Découpler anthropic
- [ ] Dans `llm/src/anthropic.rs` : remplacer `use crate::ollama::tf_libelle;` par `use crate::ollama::tf_libelle;` (restre valide, intra-llm) ET l'appel `crate::prompts_handler::prompt_effectif(...)` par `crate::prompt_effectif(...)` (déplacée dans llm).

### 2.2 prompts_handler.rs (api) — retirer prompt_effectif
- [ ] Dans `api/src/prompts_handler.rs` : retirer la fonction `prompt_effectif` + `OVERRIDES_PATH` + le `use std::fs` associé (déplacés vers llm). L'appel interne éventuel devient `llm::prompt_effectif(...)`. Garder le reste du handler (CRUD endpoints).

### 2.3 Vérifier les imports intra-ollama
- [ ] Les fichiers `llm/src/ollama/*.rs` font `use crate::ollama::...` et `super::...` — restent valides (llm a `pub mod ollama`). Les `use crate::http_client::HTTP_CLIENT` (si ollama/ l'utilisait) → `crate::OLLAMA_HTTP_CLIENT` (llm a déjà son OLLAMA_HTTP_CLIENT dans ollama/mod.rs). Vérifier au build.

---

## Task 3: Mettre à jour `api` (~18 fichiers)

### 3.1 main.rs
- [ ] Retirer `mod ollama;` et `mod anthropic;` (garder `mod ollama_types;` et `mod prompts_handler;`).
- [ ] `ollama_handlers.rs` est une façade `pub use` (3 lignes) qui re-exporte ollama_chat_handler etc. — vérifier qu'elle compile toujours (ses `use crate::ollama_handlers::*` ou les routes pointent vers les handlers restants).

### 3.2 Cargo.toml api
- [ ] Ajouter `llm = { path = "../llm" }`.

### 3.3 Migrer les ~18 consommateurs : `crate::ollama::` → `llm::`
Fichiers (cf grep) : `news_traduction.rs`, `ollama_ajustements_handler.rs`, `ollama_chart_handler.rs`, `ollama_chat_handler.rs`, `ollama_handlers.rs`, `ollama_signal_ia_handler.rs`, `prompts_handler.rs`, `rockets_analyse_handler.rs`, `rockets_sauvegarder.rs`, `signal_engine_asset.rs`, `signal_filtre.rs`, `smc_analyse_handler.rs`, `smc_signal_ollama.rs`, `straddle_handlers.rs`, `straddle_signal_handler.rs`, `straddle_signal_ollama.rs`, `http_client.rs` (vérifier), `routes.rs` (vérifier).
- [ ] Pour chacun : `use crate::ollama::X` → `use llm::X` (grâce au re-export `pub use ollama::*`). Les types `SignalSMCCandidat`, `HistoriqueSMCSignal`, `SignalCandidat` (définis dans `ollama/{smc,rockets}_filtre.rs`, désormais dans llm) suivent le même chemin.
- [ ] `crate::prompts_handler::prompt_effectif` → `llm::prompt_effectif` partout.

> Le **build guide cette migration** : chaque `crate::ollama` oublié fait erreur compile → corriger. C'est mécanique.

---

## Task 4: Build + tests + commit

- [ ] `cargo build -p llm` → OK.
- [ ] `cargo build -p api` → OK (itérer sur les imports oubliés).
- [ ] `cargo test --workspace` → **119 verts** (invariant — aucun test dans ollama/anthropic).
- [ ] Vérifier : `grep -rn "crate::ollama" backend/crates/api/src/` → **vide** (tout migré vers llm).
- [ ] **Commit** :
```bash
git add -A && git commit -m "refactor: extrait le crate llm (Ollama + Anthropic) du monolithe api

Phase 1.6b. Déplacé ollama/ (15 fichiers) + anthropic.rs vers crate llm.
prompt_effectif déplacée vers llm (découple le cycle anthropic→prompts_handler).
ollama_types.rs reste dans api (transport DTOs). ~18 fichiers api migrés
crate::ollama → llm. Aucun test impacté (119 verts). Débloque 1.6c (news)."
```

---

## Task 5: Validation
- [ ] `cargo build --release` workspace OK.
- [ ] Grep `crate::ollama` dans api → vide ; `ollama_types` reste dans api (3 handlers).
- [ ] **Vérif runtime (reportée au propriétaire)** : lancer l'app, déclencher une analyse SMC/Straddle/Rockets + chat IA + analyse chart → confirmer que les appels LLM (Ollama + Claude) fonctionnent toujours.
- [ ] Rapport au propriétaire. **Ne pas pousser.**

---

## Self-Review
**Risques :**
- **Cycle anthropic→prompts_handler** : résolu en déplaçant `prompt_effectif` (lecture fichier pure) dans llm. prompts_handler l'importe depuis llm.
- **Dépendances llm manquantes** : si ollama/ lit la DB ou utilise une crate non listée, le build llm échoue → ajouter. Le build est le garde-fou.
- **`http_client.rs`/`routes.rs` référencent ollama** : vérifier au build (probablement un re-export ou un commentaire) — corriger en `llm::` si réel.
- **ollama_handlers.rs façade** : c'est un `pub use` re-exportant les handlers ; après extraction, vérifier qu'elle re-exporte toujours correctement (les handlers restent dans api).
- **Types mobiles** (SignalSMCCandidat etc.) : suivent ollama dans llm ; consommateurs métier (smc_signal_ollama, etc.) migrent leur import.
- **Invariant 119 tests solide** : aucun test dans les modules déplacés.
