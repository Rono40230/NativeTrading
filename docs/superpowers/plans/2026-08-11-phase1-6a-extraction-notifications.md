# Phase 1.6a — Extraction crate `notifications` — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans.

**Goal:** Extraire le cluster Telegram (3 fichiers) du monolithe `api` vers un nouveau crate `notifications`. C'est la **preuve de concept** du démonolithage 1.6 : valider la mécanique (créer un crate → déplacer des fichiers → casser les imports → build/test) avec un risque minimal, car le cluster est fermé (0 consommateur externe).

**Architecture:** Nouveau crate `notifications` contenant `telegram`, `telegram_formatage`, `telegram_worker`. Dépendances : `db`, `reqwest`, `sqlx`, `tokio`, `tracing`. Le client HTTP partagé est recréé localement (LazyLock, 12 lignes) pour éviter une dépendance `api→notifications`. Le worker reste spawné dans `main.rs` mais préfixé `notifications::`.

**Tech Stack:** Rust workspace, Actix (api), SQLx, reqwest, tokio.

## Global Constraints

- **Zéro panic Vibe** ; limite 600 lignes (D0) ; **test avant validation** : `cargo test --workspace` = **119 verts** (invariant — aucun test dans le cluster Telegram) + `cargo build -p api -p notifications`.
- **Commit local par tâche** ; **push = propriétaire**.
- **L'assistant pose ses questions et attend la réponse.**

### Build backend — env CUDA (avant `cargo`)

```bash
cd /mnt/IA/native-trading-ai/backend
export LIBTORCH=/mnt/IA/libtorch
export XGBOOST_LIB_DIR=/home/rono/.local/lib/python3.14/site-packages/xgboost/lib
export LIBCLANG_PATH=/run/host/usr/lib64
export BINDGEN_EXTRA_CLANG_ARGS="-I/run/host/usr/lib/clang/21/include -I/run/host/usr/include"
export LD_LIBRARY_PATH=$LIBTORCH/lib:$XGBOOST_LIB_DIR:/run/host/usr/lib64:$LD_LIBRARY_PATH
export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
export RUSTFLAGS="-L $(pwd)/../.cargo-fake-libs ${RUSTFLAGS:-}"
export CC=clang CXX=clang++
[ -f ../.cargo-fake-libs/libstdc++fs.a ] || ar rcs ../.cargo-fake-libs/libstdc++fs.a
```

---

## File Structure

| Action | Chemin |
|--------|--------|
| **Créer** | `backend/crates/notifications/Cargo.toml` |
| **Créer** | `backend/crates/notifications/src/lib.rs` (pub mod + LazyLock HTTP local) |
| **Déplacer** | `backend/crates/api/src/telegram.rs` → `backend/crates/notifications/src/telegram.rs` |
| **Déplacer** | `backend/crates/api/src/telegram_formatage.rs` → `backend/crates/notifications/src/telegram_formatage.rs` |
| **Déplacer** | `backend/crates/api/src/telegram_worker.rs` → `backend/crates/notifications/src/telegram_worker.rs` |
| **Modifier** | `backend/Cargo.toml` (ajouter `"crates/notifications"` au workspace) |
| **Modifier** | `backend/crates/api/Cargo.toml` (ajouter `notifications = { path = "../notifications" }`) |
| **Modifier** | `backend/crates/api/src/main.rs` (retirer 3 `mod telegram*;`, préfixer le spawn) |

> Les imports internes au cluster (`use crate::telegram::...` dans `telegram_formatage`/`telegram_worker`) **restent `crate::`** car les 3 fichiers sont désormais dans le MÊME crate `notifications`. Rien à changer pour eux.

---

## Task 0: Branche + baseline

- [ ] `git checkout main && git checkout -b phase1-6a-notifications`
- [ ] Baseline : `cargo test --workspace` = **119**.

---

## Task 1: Créer le crate `notifications`

### 1.1 Cargo.toml
- [ ] Créer `backend/crates/notifications/Cargo.toml` :
```toml
[package]
name = "notifications"
version.workspace = true
edition.workspace = true

[dependencies]
db = { path = "../db" }
reqwest = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
```
> Vérifier les versions workspace dans `backend/Cargo.toml [workspace.dependencies]`. Ne pas fixer de version hardcodée — utiliser `{ workspace = true }` (comme les autres crates). Ajouter tout `use` que les 3 fichiers telegram utilisent réellement (lire leurs `use` en tête de fichier : chrono, serde, etc.).

### 1.2 lib.rs (avec HTTP client local)
- [ ] Créer `backend/crates/notifications/src/lib.rs` :
```rust
//! Crate `notifications` — envoi de notifications (Telegram, et futur email).
//! Extrait du monolithe `api` (phase 1.6a). Cluster fermé : aucune dépendance
//! vers le métier ; consommé uniquement via `notifications::telegram_worker`.
pub mod telegram;
pub mod telegram_formatage;
pub mod telegram_worker;

use std::sync::LazyLock;

/// Client HTTP partagé du crate notifications (Telegram Bot API, timeout 10 s).
/// Recréé localement pour éviter une dépendance api→notifications.
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
```

### 1.3 Workspace member
- [ ] `backend/Cargo.toml` : ajouter `    "crates/notifications",` au `[workspace] members` (après `"crates/common"` par ex.).

### 1.4 Vérifier que le crate (vide de logique) compile
- [ ] `cargo build -p notifications` → OK (le crate compile avec ses 3 modules encore vides/à déplacer).

---

## Task 2: Déplacer les 3 fichiers

- [ ] Avec `git mv` (préserve l'historique) :
```bash
cd backend/crates/api/src
git mv telegram.rs ../../notifications/src/telegram.rs
git mv telegram_formatage.rs ../../notifications/src/telegram_formatage.rs
git mv telegram_worker.rs ../../notifications/src/telegram_worker.rs
```

### 2.1 Casser la dépendance `crate::http_client` dans `telegram.rs`
- [ ] Dans `notifications/src/telegram.rs`, remplacer `use crate::http_client::HTTP_CLIENT;` (ou les usages `crate::http_client::HTTP_CLIENT`) par `use crate::HTTP_CLIENT;` (le LazyLock local défini dans lib.rs). Vérifier les lignes (~11, ~105). Le timeout 10 s correspond à l'ancien comportement Telegram.

### 2.2 Imports internes — vérifier (rien à faire normalement)
- [ ] Vérifier que `telegram_formatage.rs` et `telegram_worker.rs` utilisent `use crate::telegram::...` / `crate::telegram_formatage::...` — ces chemins **restent valides** (intra-crate notifications). Si l'un pointait vers `crate::http_client` (peu probable), appliquer la même correction qu'en 2.1.

---

## Task 3: Mettre à jour `api`

### 3.1 Cargo.toml de api
- [ ] `backend/crates/api/Cargo.toml` : ajouter `notifications = { path = "../notifications" }` dans `[dependencies]`.

### 3.2 main.rs
- [ ] Retirer les 3 déclarations : `mod telegram;`, `mod telegram_formatage;`, `mod telegram_worker;` (~lignes 100-102 — confirmer en lisant).
- [ ] Repérer le spawn du worker (~ligne 177 : `telegram_worker::demarrer_worker_telegram(pool_telegram)`) et le préfixer : `notifications::telegram_worker::demarrer_worker_telegram(pool_telegram)`.
- [ ] Vérifier qu'aucun autre fichier `api/src/` n'importe `crate::telegram` (grep) — normalement non (cluster fermé confirmé par l'investigation).

### 3.3 Vérifier qu'aucun `crate::telegram` résiduel dans api
- [ ] `grep -rn "crate::telegram\|use telegram" backend/crates/api/src/` → **vide**.

---

## Task 4: Build + tests + commit

- [ ] `cargo build -p api -p notifications` → `Finished` (les 2 crates compilent).
- [ ] `cargo test --workspace` → **119 verts** (invariant — aucun test déplacé).
- [ ] **Commit** :
```bash
git add -A && git commit -m "refactor: extrait le crate notifications (Telegram) du monolithe api

Phase 1.6a — première extraction du démonolithage. Cluster Telegram (3
fichiers, ~550 lignes) déplacé vers un crate `notifications` dédié. Client
HTTP recréé localement (LazyLock) pour éviter api→notifications. Worker
toujours spawné dans main.rs via notifications::telegram_worker.
Aucun test impacté (119 verts)."
```

---

## Task 5: Validation

- [ ] `cargo build --release` (workspace complet) → OK.
- [ ] Vérifier : `ls backend/crates/notifications/src/` contient bien `lib.rs`, `telegram.rs`, `telegram_formatage.rs`, `telegram_worker.rs`.
- [ ] Vérifier : `grep -rn "crate::telegram" backend/crates/api/src/` → vide.
- [ ] **Vérif runtime (reportée au propriétaire)** : lancer `./scripts/run.sh`, confirmer dans les logs que le `Telegram worker` démarre, et (si possible) qu'un signal de test déclenche bien l'envoi Telegram (le comportement doit être inchangé).
- [ ] Rapport au propriétaire. **Ne pas pousser.**

---

## Self-Review (post-rédaction)

**Spec coverage :** extraction `notifications` = preuve de concept 1.6 ✓.

**Risques résiduels :**
- **Dépendances manquantes dans Cargo.toml** : si un des 3 fichiers utilise une crate non listée (ex: `common`, `strategies`), le build `notifications` échouera → l'implementer lit les `use` et ajoute la dep. Le build est le garde-fou.
- **http_client timeout** : recréé à 10 s (ancien comportement Telegram). Si telegram.rs utilisait un timeout différent via l'ancien `HTTP_CLIENT` (30 s générique), le comportement change légèrement — mais Telegram était historiquement à 10 s, donc OK.
- **`pool_telegram`** : vérifier comment `main.rs` obtient `pool_telegram` (sans doute un clone du pool DB) — inchangé par l'extraction.
- **Aucun test dans le cluster** : l'invariant 119 tests est solide (aucun test à déplacer/casser).

**Critère de réussite de la preuve de concept :** la mécanique crate→déplacement→imports→build→test est validée, ouvrant la voie à 1.6b (llm), 1.6c (news), 1.6d (brokers).
