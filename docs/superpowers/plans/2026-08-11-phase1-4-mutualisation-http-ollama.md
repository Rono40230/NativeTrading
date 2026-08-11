# Phase 1.4 — Mutualisation HTTP + types Ollama — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Mutualiser les clients HTTP (1 client générique + le client Ollama existant), unifier les types « réponse Ollama » dupliqués 7×, et **corriger les 4 contournements du sémaphore Ollama** (le vrai bug de concurrence — l'audit n'en citait qu'un).

**Architecture:** Refactor interne, comportement préservé (sauf amélioration : tous les appels LLM passent par le sémaphore max 2 → fini la saturation VRAM). 2 clients partagés : `OLLAMA_HTTP_CLIENT` (existant, 300 s, LLM) + nouveau `HTTP_CLIENT` (30 s, générique). Timeouts spéciaux gérés par-requête (`.timeout()` sur le `RequestBuilder`).

**Tech Stack:** Rust, Actix-Web, reqwest, tokio (Semaphore + LazyLock).

## Global Constraints

- **Zéro panic Vibe** ; limite 600 lignes (D0) ; **test avant validation** (`cargo test --workspace` = 116 verts, invariant — aucun test supprimé ici) ; commit local par tâche ; **push = propriétaire**.
- **L'assistant pose ses questions et attend la réponse.**

### Build backend — env CUDA (avant toute commande `cargo`)

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

> **Note test** : c'est un refactor. Aucun test existant n'exerce l'appel HTTP Ollama (tests de math/parsing purs) — les 116 tests resteront verts. La validation du comportement LLM réel est une vérification runtime (reportée au propriétaire : lancer avec Ollama, confirmer que l'inférence passe et que la VRAM ne sature plus).

---

## File Structure

| Fichier | Action | Tâche |
|---------|--------|-------|
| `ollama/mod.rs` | Ajouter `pub use types::ReponseOllama;` | 1 |
| `ollama/{rockets,smc,straddle}_{analyse,filtre}.rs` (4 fichiers) | Supprimer le doublon `OllamaResp`/`OllamaMsg`, importer `crate::ollama::ReponseOllama` | 1 |
| `straddle_types.rs` | Supprimer `OllamaResp`/`OllamaMsg` (garder `ReponseLlm`, `RequeteStraddleSignal`) | 1 |
| `straddle_signal_ollama.rs` | Fix bypass n°1 → `OLLAMA_HTTP_CLIENT` + sémaphore | 2 |
| `straddle_signal_handler.rs` | Fix bypass n°2 → idem | 2 |
| `news_traduction.rs` (×2 fonctions) | Fix bypass n°3 et n°4 → idem | 2 |
| `ollama_chat_handler.rs` | `Client::new()` → `OLLAMA_HTTP_CLIENT` | 3 |
| `http_client.rs` (**nouveau**) | Définir `HTTP_CLIENT` générique 30 s | 4 |
| 17 fichiers non-Ollama (anthropic, telegram, news, prix, rockets, sentiment, straddle_moniteur, handlers) | `Client::builder()` → `crate::http_client::HTTP_CLIENT` (+ `.timeout()` par-requête si besoin) | 4 |

**Hors scope 1.4** (laissés tels quels) : `ig_lightstreamer/*` (4 sites, timeout 0 / handshake LS), `state.rs:64` (IG session stockée dans `IgSession`), `bin/backfill_rockets.rs`. La factorisation des pipelines LLM est réservée à 1.5.

---

## Task 0: Branche + baseline

- [ ] `git checkout main && git checkout -b phase1-4-mutualisation-http`
- [ ] Baseline : `cargo test --workspace` = **116**. (Cible finale : 116, invariant.)

---

## Task 1: Unifier les types « réponse Ollama » (6 doublons → 1)

Tous 6 doublons sont **identiques** au canonique `{ message: { content: String } }`. Remplaçables 1:1.

### 1.1 Exposer le type canonique
- [ ] Dans `ollama/mod.rs`, ajouter le re-export : `pub use types::ReponseOllama;` (le module `types` est privé ; le re-export le rend accessible via `crate::ollama::ReponseOllama`).

### 1.2 Supprimer les 6 doublons + ajuster les imports
- [ ] `ollama/straddle_analyse.rs` (~l.29-36) : supprimer `OllamaResp`/`OllamaMsg` ; remplacer l'usage `.json::<OllamaResp>()` par `.json::<super::ReponseOllama>()`.
- [ ] `ollama/rockets_analyse.rs` (~l.260-267) : idem.
- [ ] `ollama/rockets_filtre.rs` (~l.126-133) : idem.
- [ ] `ollama/smc_analyse.rs` (~l.260-267) : idem.
- [ ] `ollama/smc_filtre.rs` (~l.228-235) : idem.
- [ ] `straddle_types.rs` (~l.51-59) : supprimer `OllamaResp`/`OllamaMsg` **uniquement** (conserver `ReponseLlm` l.21-47 et `RequeteStraddleSignal` l.7-17).

### 1.3 Ajuster les importeurs de `straddle_types::OllamaResp`
- [ ] `straddle_signal_ollama.rs:9` et `straddle_signal_handler.rs:7` : remplacer l'import `OllamaResp` (de `straddle_types`) par `crate::ollama::ReponseOllama`. Les usages `.json::<OllamaResp>()` deviennent `.json::<crate::ollama::ReponseOllama>()`. *(Ces 2 fichiers sont réouverts en Task 2 pour le fix bypass — mais l'import type se fait ici.)*

### 1.4 Vérifier + commit
- [ ] `cargo build -p api` → OK. `grep -rn "struct OllamaResp\|struct OllamaMsg" backend/crates/api/src/` → **vide**.
- [ ] `cargo test --workspace` → 116 verts.
- [ ] Commit : `refactor(ollama): unifie le type réponse Ollama (6 doublons → ReponseOllama)`

---

## Task 2: Corriger les 4 contournements du sémaphore (le bug)

Chaque site migre vers le pattern canonique : `OLLAMA_SEMAPHORE.acquire()` + `&*OLLAMA_HTTP_CLIENT`. Le timeout passe de 120/30/15 s → 300 s (amélioration : fini les timeouts prématurés sur Qwen3 lent).

### 2.1 Bypass n°1 — `straddle_signal_ollama.rs` (~l.80-91)
- [ ] Remplacer le bloc de construction du client + l'appel :
```rust
// AVANT (l.80-91) :
let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build()?;
... client.post(&url).json(&corps).send().await?.json::<OllamaResp>().await?.message.content ...

// APRÈS :
let _permit = crate::ollama::OLLAMA_SEMAPHORE.acquire().await.ok();
let client = &*crate::ollama::OLLAMA_HTTP_CLIENT;
let reponse = client.post(&url).json(&corps).send().await
    .map_err(|e| /* TradingError existant */)?;
let texte_brut = reponse.json::<crate::ollama::ReponseOllama>().await?.message.content;
```
> Adapter la gestion d'erreur au `TradingError`/`?` déjà utilisé localement (ne pas introduire de `unwrap`). Retirer l'import `Duration` s'il devient inutilisé.

### 2.2 Bypass n°2 — `straddle_signal_handler.rs` (~l.143-155)
- [ ] Même migration (handler manuel `POST /api/straddle/signal`). Le client est construit l.143, l'appel l.149, deserialize l.155. Remplacer par `OLLAMA_HTTP_CLIENT` + `OLLAMA_SEMAPHORE`.

### 2.3 Bypass n°3 — `news_traduction.rs::traduire` (~l.66, l.74)
- [ ] Même migration. **Le plus important** : cette fonction est appelée par titre d'article (via `news_handlers.rs:149`) → sans sémaphore, peut lancer > 2 inférences concurrentes. Timeout passe 30 s → 300 s.

### 2.4 Bypass n°4 — `news_traduction.rs::analyser_sentiment` (~l.170, l.178)
- [ ] Même migration. Idem risque de saturation (appel massif). Timeout 15 s → 300 s.

### 2.5 Vérifier + commit
- [ ] `cargo build -p api` → OK.
- [ ] Vérification sémantique : `grep -rn "OLLAMA_SEMAPHORE.acquire" backend/crates/api/src/` doit couvrir les 4 sites (straddle_signal_ollama, straddle_signal_handler, news_traduction×2) + les 9 existants.
- [ ] Vérifier qu'aucun `Client::builder()` ne subsiste dans un chemin d'inférence LLM Ollama : `grep -rn "Client::builder\|Client::new" backend/crates/api/src/ | grep -v ig_lightstreamer | grep -v state.rs` → les seuls restants doivent être non-Ollama (Anthropic, Telegram, news, prix… — traités Task 4) + `ollama/mod.rs` (définition du client partagé).
- [ ] `cargo test --workspace` → 116 verts.
- [ ] Commit : `fix(ollama): 4 contournements du sémaphore corrigés (straddle×2, news×2)`

---

## Task 3: `ollama_chat_handler.rs` → client partagé

- [ ] `ollama_chat_handler.rs:184` : `reqwest::Client::new()` (GET `/api/tags` de disponibilité) → `&*crate::ollama::OLLAMA_HTTP_CLIENT`. Pas de sémaphore (GET de dispo, pas une inférence).
- [ ] `cargo build -p api` + tests 116. Commit : `refactor(ollama): ollama_chat_handler réutilise OLLAMA_HTTP_CLIENT`

---

## Task 4: Client HTTP générique `HTTP_CLIENT` + migration des 17 sites non-Ollama

### 4.1 Créer `backend/crates/api/src/http_client.rs`
```rust
//! Client HTTP partagé pour les services externes non-LLM (Anthropic, Telegram,
//! news, prix, IG REST…). Pour Ollama, utiliser `crate::ollama::OLLAMA_HTTP_CLIENT`
//! (timeout 300 s + sémaphore). Les timeouts spéciaux se gèrent par-requête via
//! `RequestBuilder::timeout(...)`.
use std::sync::LazyLock;
use std::time::Duration;

pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
```
- [ ] Déclarer `mod http_client;` dans `main.rs` et `pub use`/`use crate::http_client::HTTP_CLIENT;` selon le style.

### 4.2 Migrer les 17 sites (catégorie b)
Pour chaque site : remplacer `reqwest::Client::builder()...build()` (ou `Client::new()`) par `&*crate::http_client::HTTP_CLIENT`. Ajouter `.timeout(...)` **par-requête** si le timeout actuel diffère de 30 s.

| Fichier:~ligne | Service | Timeout à conserver par-requête |
|---|---|---|
| `anthropic.rs:113` | Anthropic chart | `.timeout(Duration::from_secs(180))` |
| `anthropic.rs:208` | Anthropic chat | `.timeout(Duration::from_secs(60))` |
| `telegram.rs:11`, `:107` | Telegram | (10 s < 30 s du client → OK, laisser client gérer, retirer builder) |
| `calendar_handlers.rs:25` | Calendrier éco | (8 s, user_agent — **piège** : le user_agent est sur le builder. Voir note) |
| `news_fear_greed.rs:20` | fear-greed | (5 s, user_agent — **piège**) |
| `news_handlers.rs:63`, `:189` | RSS / scraper | (10/15 s, user_agent Mozilla — **piège**) |
| `prix_handlers.rs:115` | Binance prix | (10 s) |
| `prix_utils.rs:210` | Prix IG | `.timeout(Duration::from_millis(1500))` |
| `rockets_scan.rs:39` | Binance klines | (15 s) |
| `rockets_suivi.rs:16`, `:231` | Suivi Rockets | (10 s) |
| `sentiment_handlers.rs:125` | Scraper sentiment | (user_agent — **piège**) |
| `straddle_moniteur_position.rs:55` | Moniteur Straddle | (8 s) |
| `handlers.rs:195` | Binance ticker | (5 s) |

> **Piège user_agent** : 5 sites posent un `.user_agent("Mozilla/5.0...")` ou similaire sur le builder. Le `HTTP_CLIENT` partagé n'en a pas. Deux options : (a) poser le `user_agent` par-requête via `RequestBuilder::header(USER_AGENT, ...)` ; (b) laisser sans UA (peut casser les scrapers RSS/sentiment qui exigent un UA navigateur). **Recommandation : poser le UA par-requête** pour ces 5 sites (conserver le comportement). Si l'implementer préfère, il peut garder un client local pour ces 5 sites — mais l'objectif est la mutualisation, donc (a).

### 4.3 Vérifier + commit
- [ ] `cargo build -p api` → OK.
- [ ] `grep -rn "Client::builder\|Client::new" backend/crates/api/src/ | grep -v "ollama/mod.rs\|http_client.rs\|ig_lightstreamer\|state.rs"` → **vide** (tous migrés).
- [ ] `cargo test --workspace` → 116 verts.
- [ ] Commit : `refactor(http): client HTTP générique partagé (17 sites migrés)`

---

## Task 5: Validation finale

- [ ] `cargo build --release` + `cargo test --workspace` (116 verts).
- [ ] `npm run build` (sanity, frontend non touché).
- [ ] Vérifications grep résumées :
  - `struct OllamaResp|struct OllamaMsg` → vide (types unifiés).
  - `Client::builder|Client::new` hors `{ollama/mod.rs, http_client.rs, ig_lightstreamer/*, state.rs, bin/}` → vide.
  - `OLLAMA_SEMAPHORE.acquire` couvre les 4 ex-bypass + les 9 existants.
- [ ] **Vérification runtime (reportée au propriétaire)** : lancer `./scripts/run.sh` avec Ollama en route, déclencher une analyse Straddle + une revue de presse (news_traduction), et confirmer : (a) l'inférence passe, (b) `nvidia-smi` ne montre pas > 2 inférences concurrentes (sémaphore respecté).
- [ ] Rapport au propriétaire. **Ne pas pousser.**

---

## Self-Review (post-rédaction)

**Spec coverage (fondation §5 Phase 1.4) :**
- 1 client HTTP partagé → Task 4 ✓ (HTTP_CLIENT + OLLAMA_HTTP_CLIENT existant)
- Types Ollama uniques → Task 1 ✓
- Sémaphore respecté partout → Task 2 (4 bypass) + Task 3 ✓

**Placeholders :** le code de migration des bypass (§2.1) est donné en exemple pour le n°1 ; les 3 autres suivent le même pattern (l'implementer relit chaque site pour le `TradingError` local). Les 5 sites user_agent (§4.2) ont un piège explicitement traité.

**Risques résiduels :**
- **Comportement LLM runtime** : aucun test ne couvre l'appel HTTP réel. La validation du sémaphore est sémantique (grep) + runtime (propriétaire). C'est la limite naturelle d'un refactor réseau sans tests d'intégration.
- **Timeouts allongés** (15/30/120 s → 300 s sur les bypass) : amélioration (Qwen3 lent), mais un appel news bloqué occupe un slot sémaphore plus longtemps. Acceptable (max 2 concurrents → backpressure naturelle).
- **user_agent scrapers** : si l'implementer oublie le UA par-requête sur les 5 sites RSS/scraper, les sites peuvent 403. §4.2 le signale explicitement.
- **Task 4 volumineuse** (17 sites) : purement mécanique mais touche beaucoup de fichiers. Risque d'oubli → le grep §4.3 le rattrape.
