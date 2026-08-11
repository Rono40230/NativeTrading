# Phase 1.6d — Absorption broker (IG + prix Binance) dans `data` — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Absorber l'intégration broker d'`api` (IG session/streaming + fetch prix) dans le crate `data` EXISTANT, résorbant la fuite data↔api. Stratégie : **3 sous-étapes** (ig_session → ig_lightstreamer → prix_utils), chacune isolée par un **shim `pub use data::...`** laissé dans `api` pour ne casser aucun consumer dans la même étape.

**Architecture:** Les types `IgSession`, `IgLightstreamer`, et les fonctions `prix_utils::*` déménagent dans `data`. `api` garde des shims 1-ligne `pub use data::...` (puis éventuellement migre ses imports plus tard). Graphe final : `api → data → db → common` (unidirectionnel, aucun cycle). **`ig_lightstreamer::run()` est déjà un no-op** (désactivé) → risque runtime quasi-nul pour d2.

**Tech Stack:** Rust workspace, reqwest, tokio, SQLx (db).

## Global Constraints
- Zéro panic Vibe ; limite 600 ; **test = 119** (invariant) + `cargo build -p api -p data` à chaque sous-étape.
- Commit local par sous-étape ; push = propriétaire ; l'assistant pose ses questions et attend.
- **Vérif runtime clé (reportée au propriétaire)** à la fin : boot serveur → `/api/ig/statut-local` répond + `/api/prix?assets=XAUUSD,BTC` renvoie des prix.

### Build env CUDA (voir plans précédents).

---

## Task 0: Branche + baseline
- [ ] Branche `phase1-6d-brokers` DÉJÀ CRÉÉE (baseline 119).

---

## Task 1 (1.6d1) : `ig_session` → `data`

**Le plus safe** : `ig_session.rs` est auto-contenu (0 `use crate::`), 1 seul type `IgSession`.

- [ ] **1.1** `data/Cargo.toml` : ajouter `db = { path = "../db" }`.
- [ ] **1.2** Créer `backend/crates/data/src/ig_session.rs` = **copie conforme** de `api/src/ig_session.rs` (237 lignes, contenu inchangé — il n'a aucun `use crate::`).
- [ ] **1.3** `data/src/lib.rs` : ajouter `pub mod ig_session;`.
- [ ] **1.4** Remplacer `api/src/ig_session.rs` par un shim 1-ligne :
```rust
//! Shim de ré-export — IgSession désormais dans le crate `data` (phase 1.6d1).
pub use data::ig_session::IgSession;
```
- [ ] **1.5** Build + tests : `cargo build -p data` OK ; `cargo build -p api` OK (le shim préserve `crate::ig_session::IgSession` partout) ; `cargo test --workspace` = **119**.
- [ ] **Commit** : `refactor: déplace IgSession vers le crate data (shim re-export dans api)`

---

## Task 2 (1.6d2) : `ig_lightstreamer/` → `data`

**Risque runtime quasi-nul** : `run()` est un no-op (désactivé, log "boucle LS run() désactivée").

- [ ] **2.1** `data/Cargo.toml` : ajouter `futures-util = { workspace = true }` (pour `connection.rs` `bytes_stream()`). Vérifier que futures-util est dans les workspace deps (sinon l'ajouter).
- [ ] **2.2** `git mv backend/crates/api/src/ig_lightstreamer backend/crates/data/src/ig_lightstreamer`.
- [ ] **2.3** `data/src/lib.rs` : ajouter `pub mod ig_lightstreamer;`.
- [ ] **2.4** Dans `data/src/ig_lightstreamer/{mod,connection,rest_ig}.rs` : les `use crate::ig_session::IgSession` **restent valides** (désormais intra-`data` car `ig_session` y est aussi). Les `use data::providers::ig as ig_helpers` (déjà présents dans mod.rs/rest_ig) deviennent `use crate::providers::ig as ig_helpers` (intra-data). Vérifier au build.
- [ ] **2.5** Remplacer `api/src/ig_lightstreamer/mod.rs` par un shim — MAIS `api/src/ig_lightstreamer/` est un dossier déplacé. Créer un fichier `api/src/ig_lightstreamer.rs` (fichier simple, pas dossier) :
```rust
//! Shim — IgLightstreamer + LsCandle + SubKey désormais dans `data` (phase 1.6d2).
pub use data::ig_lightstreamer::{IgLightstreamer, LsCandle, SubKey};
```
> ⚠ Comme `api/src/ig_lightstreamer/` était un dossier (module), le déplacer via `git mv` puis créer un fichier `ig_lightstreamer.rs` à la place. Rust accepte un fichier `ig_lightstreamer.rs` comme module même si c'était un dossier avant. Vérifier que `main.rs mod ig_lightstreamer;` compile vers le fichier shim.
- [ ] **2.6** Build + tests : `cargo build -p data` OK ; `cargo build -p api` OK (shim préserve `crate::ig_lightstreamer::IgLightstreamer`) ; `cargo test --workspace` = 119.
- [ ] **Commit** : `refactor: déplace IgLightstreamer vers le crate data (shim re-export)`

---

## Task 3 (1.6d3) : `prix_utils` → `data` (la plus délicate runtime)

5 workers actifs consomment `prix_utils::fetch_*`. Protégé par : signatures inchangées + shim.

- [ ] **3.1** `git mv backend/crates/api/src/prix_utils.rs backend/crates/data/src/prix_utils.rs`.
- [ ] **3.2** `data/src/lib.rs` : ajouter `pub mod prix_utils;`.
- [ ] **3.3** Dans `data/src/prix_utils.rs` : `use crate::ig_session::IgSession` reste valide (intra-data). La fonction `client_http()` (qui faisait `&crate::http_client::HTTP_CLIENT`) — **décision : supprimer `client_http()` du code déplacé** (elle n'a plus accès à `api::http_client`). Les consumers api utilisent `prix_utils::client_http()` à 5 endroits : il faut les migrer.
- [ ] **3.4** Remplacer `api/src/prix_utils.rs` par un shim :
```rust
//! Shim — prix_utils désormais dans `data` (phase 1.6d3).
pub use data::prix_utils::*;
```
- [ ] **3.5** Migrer les 5 consumers de `prix_utils::client_http()` → `crate::http_client::HTTP_CLIENT` (le client partagé api) :
  - `pip_updater.rs:37`, `prix_handlers.rs:18`, `prix_stream.rs:29`, `signaux_handlers.rs:73`, `ws_handlers/ig.rs:108`.
  - Remplacer `prix_utils::client_http()` par `&crate::http_client::HTTP_CLIENT`. Les `fetch_*` prennent déjà `client: &reqwest::Client` en paramètre → pas de changement de signature.
- [ ] **3.6** Build + tests : `cargo build -p data` + `cargo build -p api` OK ; `cargo test --workspace` = 119.
- [ ] **Commit** : `refactor: déplace prix_utils vers le crate data (5 consumers migrés vers http_client partagé)`

---

## Task 4: Validation finale 1.6d

- [ ] `cargo build --release` workspace OK.
- [ ] Grep : `ls backend/crates/api/src/ig_session.rs backend/crates/api/src/ig_lightstreamer.rs backend/crates/api/src/prix_utils.rs` = 3 shims ; `ls backend/crates/data/src/` contient `ig_session.rs`, `ig_lightstreamer/`, `prix_utils.rs`.
- [ ] `cargo test --workspace` = 119.
- [ ] **Vérif runtime (reportée au propriétaire)** — CRUCIALE pour 1.6d :
  - `./scripts/run.sh` → boot sans panic.
  - Logs : "✅ IG Markets: connecté au démarrage" OU "login différé" (state.rs:83-84).
  - `curl 'http://localhost:8080/api/ig/statut-local'` répond `{"connecte":...}`.
  - `curl 'http://localhost:8080/api/prix?assets=XAUUSD,BTC'` renvoie des prix (valide les 5 workers prix).
- [ ] Rapport au propriétaire. Ne pas pousser.

---

## Self-Review
**Risques :**
- **Task 2 piège dossier→fichier** : passer d'un module-dossier `ig_lightstreamer/` à un fichier shim `ig_lightstreamer.rs` dans api. Rust l'accepte (le `mod ig_lightstreamer;` résout vers le fichier). Si souci, alternative : garder un dossier `api/src/ig_lightstreamer/mod.rs` contenant le shim. Le build valide.
- **Task 3 client_http** : 5 consumers à migrer. Si un est oublié → build erreur → corriger. Les fetch_* ont `client` en paramètre → pas de cascade.
- **Signatures inchangées** : `IgSession::new`, `IgLightstreamer::new`, `fetch_prix_asset` etc. ne changent pas → `state.rs` et les workers intacts via shim.
- **Tokens IG en mémoire** (AppState.ig_session) → préservés (l'instance ne bouge pas, seul le chemin du type).
- **`run()` no-op** : confirme que 1.6d2 n'a pas d'effet runtime (le streaming réel est déjà coupé).
- **Invariant 119** solide (aucun test dans les modules broker).
