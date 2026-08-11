# Phase 1.5 — Frontend : couche HTTP unifiée + drift ts-rs — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans.

**Goal:** Unifier la couche HTTP frontend (1 instance axios partagée, 8 bypass routés vers `apiService`, plus de `localhost:8080` hardcodé), corriger le drift `horizon_bougies` (rendre la colonne live), étendre ts-rs à 4 types clés, typer le store `strategyParams`, et corriger 2 bugs bonus (double connexion WS, endpoint inexistant).

**Architecture:** Refactor frontend (+ petit point de backend pour `horizon_bougies`). Comportement préservé sauf : `horizon_bougies` affiche désormais un vrai nombre, fin des notifications de signaux dupliquées, et `useChartAnalyse` fonctionne (ou est retiré si mort).

**Tech Stack:** Vue 3 + TypeScript + axios (frontend), Rust ts-rs (génération types), SQLx (backend).

## Global Constraints

- **Zéro panic Vibe** ; limite 600 lignes (D0) ; **test avant validation** : `cargo test --workspace` (116 backend verts) + `npm run build` (vue-tsc — c'est LE garde-fou principal : toute erreur de type drift sera attrapée ici).
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

> **Note ts-rs** : `cargo test --workspace` ou `cargo test -p <crate> --features export` déclenche l'export ts-rs → régénère `frontend/src/generated/*.ts`. Après avoir ajouté des `#[ts(export)]`, lancer un cargo test pour régénérer, puis committer les fichiers générés.

---

## File Structure

| Fichier | Action | Tâche |
|---------|--------|-------|
| `db/src/strategies_params.rs` | Ajouter `horizon_bougies` à `SmcParams` + `StraddleParams` (struct + Default + SQL SELECT/INSERT/UPDATE) + `#[ts(export)]` sur les 2 structs | 1 |
| `api/src/strategies_params_handlers.rs` | (vérifier) pas de changement si le handler délègue à `lire_*_params` | 1 |
| `common/src/lib.rs` (`Signal`) | `#[ts(export)]` | 2 |
| `ml/src/pipeline.rs` (`PredictionML`) | `#[ts(export)]` | 2 |
| `frontend/src/services/api.types.ts` | Remplacer `Signal`/`PredictionML` manuels par re-export depuis `generated/` | 2 |
| `frontend/src/stores/strategyParams.store.ts` | Typer `smcRaw`/`straddleRaw`/`rocketsRaw` (fini le `Record<string,any>`) | 2 |
| `frontend/src/views/{SmcDefinitionView,VolatiliteDefinitionView}.vue` | Typer `params` (le drift `horizon_bougies` devient compile-time attrapé) | 2 |
| `frontend/src/services/http.client.ts` (**nouveau**) | Instance axios partagée unique | 3 |
| `frontend/src/services/api.*.ts` (7 fichiers) | Remplacer leur `axios.create(...)` local par le shared | 3 |
| `frontend/src/services/api.service.ts` | Ajouter 8 méthodes manquantes (prompts, pre_alertes, marche/klines, save-analysis, ml/feature-importance, smc/analyse-llm, ia/chart) | 3 |
| 8 composants bypass | Remplacer `fetch`/`axios` local par `apiService.*` | 3 |
| `frontend/src/App.vue` + `composables/useSignalEngine.ts` | Dédupliquer la connexion WS signal-engine | 4 |
| `frontend/src/composables/useChartAnalyse.ts` | Réparer ou retirer (endpoint `/api/ia/chart/local` inexistant) | 4 |

---

## Task 0: Branche + baseline

- [ ] `git checkout main && git checkout -b phase1-5-frontend-http-tsrs`
- [ ] Baseline backend : `cargo test --workspace` = **116**. Baseline frontend : `cd ../frontend && npm run build` = OK.

---

## Task 1: Fix drift `horizon_bougies` (backend) + générer ParamsSmc/ParamsStraddle

La colonne existe en DB (migration `0023_strategies_params.sql`, défaut 24 smc / 48 straddle) mais aucun code Rust ne la lit. On la branche.

### 1.1 Structs + Default
- [ ] `backend/crates/db/src/strategies_params.rs` :
  - `SmcParams` (~l.102) : ajouter `pub horizon_bougies: i64,` + `horizon_bougies: 24,` dans `Default`.
  - `StraddleParams` (~l.7) : ajouter `pub horizon_bougies: i64,` + `horizon_bougies: 48,` dans `Default`.

### 1.2 ts-rs export sur les 2 structs
- [ ] Ajouter au-dessus de `SmcParams` :
```rust
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, ts_rs::TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/ParamsSmc.ts")]
```
(idem `StraddleParams` → `ParamsStraddle.ts`). Vérifier les derives déjà présents (Serialize/Deserialize/Clone/Debug) — ajouter `ts_rs::TS` sans dupliquer. Vérifier que `ts-rs` est dans les dépendances de `db/Cargo.toml` (l'ajouter si absent, avec le feature `serde-compat` si besoin — copier sur `smc/src/lib.rs:42` qui exporte déjà `ScoreSmc`).

### 1.3 SQL — lire/écrire `horizon_bougies`
- [ ] `lire_smc_params` (~l.134) : ajouter `horizon_bougies` au `SELECT` + au mapping struct. Idem `sauvegarder_smc_params` (INSERT/UPDATE ~l.163) : ajouter la colonne + bind.
- [ ] `lire_straddle_params` (~l.40) + `sauvegarder_straddle_params` (~l.66) : idem.

### 1.4 Régénérer + build + tests
- [ ] `cargo test -p db` (déclenche ts-rs → régénère `generated/ParamsSmc.ts` + `ParamsStraddle.ts`).
- [ ] Vérifier que les 2 fichiers générés contiennent bien `horizon_bougies: number`.
- [ ] `cargo build -p api` + `cargo test --workspace` = 116 verts (+ éventuels nouveaux tests).
- [ ] Commit : `feat(db): horizon_bougies rendu live (SmcParams/StraddleParams) + ts-rs export`

---

## Task 2: ts-rs `Signal` + `PredictionML` + typer le store

### 2.1 Exporter Signal + PredictionML
- [ ] `common/src/lib.rs` (`Signal` ~l.19) : ajouter `#[ts(export, export_to = "../../../../frontend/src/generated/Signal.ts")]` + derive `ts_rs::TS`. Vérifier champ-par-champ que le struct correspond au type manuel `api.types.ts:51` (noms snake_case, nullabilités). Si différence → **aligner** (le Rust est source de vérité).
- [ ] `ml/src/pipeline.rs` (`PredictionML` ~l.17) : idem → `generated/PredictionML.ts`.
- [ ] `cargo test -p common` + `cargo test -p ml` (régénère les 2 fichiers).

### 2.2 Remplacer les types manuels par re-export (non-cassant)
- [ ] `frontend/src/services/api.types.ts` : remplacer la définition manuelle de `Signal` (~l.51) et `PredictionML` (~l.6) par un re-export : `export type { Signal } from '../generated/Signal';` (et PredictionML). Vérifier que tous les importateurs (`import { Signal } from './api.types'`) continuent de compiler (le re-export preserve le nom). Pattern déjà utilisé pour `ScoreSmc` (api.types.ts:92).

### 2.3 Typer le store `strategyParams.store.ts`
- [ ] Typer les refs : `smcRaw: ref<ParamsSmc | null>(null)`, `straddleRaw: ref<ParamsStraddle | null>(null)`, `rocketsRaw: ref<RocketsConfig | null>(null)` (importer depuis `generated/`).
- [ ] Typer les méthodes `apiService.getSmcParams(): Promise<ParamsSmc>` et `getStraddleParams(): Promise<ParamsStraddle>` (api.service.ts:218, 227) — remplacer `Record<string, number>`.
- [ ] **Conserver** les computed `smcParams`/`straddleParams` (mapping UI) — ils deviennent des transformations typées `ParamsSmc → SmcParams(UI)`. **Pièges à préserver** : renommage `atr_seuil`(API) ↔ `seuil_atr`(UI), conversion `vente_partielle` bool ↔ number 0/1 (l.26, l.39).
- [ ] `SmcDefinitionView.vue:142` / `VolatiliteDefinitionView.vue:150` : typer `params` avec `ParamsSmc`/`ParamsStraddle` (fin du `Record<string, number>`). Le drift `horizon_bougies` aurait été attrapé à la compile.

### 2.4 Build frontend (le garde-fou)
- [ ] `npm run build` → **0 erreur TS**. Si erreurs (un consommateur attendait `Record<string,any>`), corriger. C'est ici que tous les drifts résiduels remontent.
- [ ] Commit : `refactor(frontend): types générés (Signal, PredictionML, Params) + store typé (fin Record<any>)`

---

## Task 3: Mutualiser la couche HTTP (8 bypass + 1 instance axios)

### 3.1 Créer l'instance axios partagée
- [ ] `frontend/src/services/http.client.ts` :
```ts
import axios from 'axios'
export const BASE_URL = 'http://localhost:8080' // TODO 1.7 CPG : dériver d'une variable d'env
export const http = axios.create({ baseURL: BASE_URL, timeout: 15000 })
```
- [ ] Dans `api.service.ts` : remplacer le `BASE_URL`+`axios.create` locaux par `import { http } from './http.client'`. Idem dans les 7 `api.*.ts` (api.rockets, api.straddle, api.smc, api.news, api.engine, api.ml_insights, api.asset_params) — chacun remplace son `axios.create({baseURL:'http://localhost:8080'})` par `import { http }`.

### 3.2 Ajouter les 8 méthodes `apiService` manquantes
- [ ] Dans `api.service.ts` (ou le sous-module pertinent), ajouter :
```ts
// Prompts
getPrompts: () => http.get('/api/prompts').then(r => r.data),
putPrompt: (id, contenu) => http.put(`/api/prompts/${id}`, { contenu }).then(r => r.data),
deletePrompt: (id) => http.delete(`/api/prompts/${id}`).then(r => r.data),
// Pré-alertes
getPreAlertes: (limit = 10) => http.get('/api/pre_alertes', { params: { limit } }).then(r => r.data),
// Marché klines (pour CryptosAlert + VeilleRockets)
getMarcheKlines: (symbol, interval, limit) => http.get('/api/marche/klines', { params: { symbol, interval, limit } }).then(r => r.data),
// IA save-analysis
saveAnalyseIA: (payload) => http.post('/api/ia/save-analysis', payload).then(r => r.data),
// ML feature-importance
getMlFeatureImportance: (strategie) => http.get(`/api/ml/feature-importance/${strategie}`).then(r => r.data),
// SMC analyse-llm (dernière analyse)
getDerniereAnalyseLlmSmc: () => http.get('/api/smc/analyse-llm').then(r => r.data),
```
(Vérifier les signatures/payloads exacts en lisant chaque composant bypass.)

### 3.3 Remplacer les 8 bypass
- [ ] `views/PromptsIAView.vue:13,40,61,75` : remplacer `axios.get/put/delete(BASE_URL...)` par `apiService.getPrompts/putPrompt/deletePrompt`. Retirer `import axios` + `BASE_URL` si inutilisés.
- [ ] `components/common/PreAlertesWidget.vue:65` : `fetch(...)` → `apiService.getPreAlertes(10)`.
- [ ] `components/common/CryptosAlert.vue:213` + `VeilleRockets.vue:119` : `fetch('/api/marche/klines...')` → `apiService.getMarcheKlines(...)`.
- [ ] `components/common/AnalyseIAModal.vue:178` : `fetch(...save-analysis)` → `apiService.saveAnalyseIA(payload)`.
- [ ] `components/common/MlRetrainPanel.vue:222` : `axios.get(BASE_URL/ml/feature-importance)` → `apiService.getMlFeatureImportance(strat.id)`.
- [ ] `composables/useSmcAnalyseNotif.ts:15,27` : `axios.get(BASE_URL/smc/analyse-llm)` → `apiService.getDerniereAnalyseLlmSmc()`.

### 3.4 Vérifier + commit
- [ ] `grep -rn "localhost:8080" frontend/src --include=*.ts --include=*.vue | grep -v http.client.ts` → **vide** (plus qu'un seul endroit, le http.client.ts).
- [ ] `grep -rn "fetch(\|new WebSocket\|axios.create" frontend/src --include=*.ts --include=*.vue | grep -v http.client.ts | grep -v "api\.service\|api\.\|WebSocket("` → ne doit plus montrer de bypass HTTP (les WS restent, traités Task 4).
- [ ] `npm run build` → OK.
- [ ] Commit : `refactor(frontend): couche HTTP unifiée (8 bypass → apiService, 1 instance axios)`

---

## Task 4: Bugs bonus — double connexion WS + useChartAnalyse

### 4.1 Dédupliquer la connexion WS signal-engine
- [ ] **Diagnostiquer** : `App.vue:41` ET `composables/useSignalEngine.ts:46` connectent tous deux `ws://localhost:8080/api/signal-engine/stream`. Vérifier ce que `App.vue` fait de sa connexion (handlers de messages) et ce que `useSignalEngine` fournit.
- [ ] **Consolider** sur `useSignalEngine` (le composable est l'abstraction propre). Retirer la connexion directe dans `App.vue` ; si `App.vue` a des handlers spécifiques, les déplacer vers le composable ou consommer le composable. Objectif : **une seule connexion** au flux signal-engine.
- [ ] (Bonus, si facile) centraliser `WS_URL` dérivé de `BASE_URL` dans `http.client.ts` pour les 4 URLs WS (`App.vue`, `useSignalEngine`, `market.store`, `prix.store`).

### 4.2 useChartAnalyse — endpoint inexistant
- [ ] **Diagnostiquer** : `composables/useChartAnalyse.ts:32` appelle `POST /api/ia/chart/local` qui n'existe pas backend (silencieux, catch vide). Vérifier si ce composable est **réellement utilisé** par un composant (`grep useChartAnalyse`).
  - **Si mort (non importé)** : le supprimer (code mort).
  - **Si utilisé** : déterminer l'intention — `local` suggère un modèle local (Ollama) vs `/api/ia/chart` (Claude cloud). Soit (a) créer `/api/ia/chart/local` backend (Ollama), soit (b) étendre `analyserChart` (api.service.ts:130 → `/api/ia/chart`) pour accepter un flag `local`/`model`. **Reporter la décision au propriétaire** si l'effort backend est non trivial.

### 4.3 Build + commit
- [ ] `npm run build` → OK.
- [ ] Vérifier : `grep -rn "signal-engine/stream" frontend/src` → 1 seule occurrence active (ou via useSignalEngine).
- [ ] Commit : `fix(frontend): déduplique connexion WS signal-engine + useChartAnalyse`

---

## Task 5: Validation finale

- [ ] Backend : `cargo test --workspace` (116+) + `npm run build` frontend OK.
- [ ] Grep résumés : plus de `localhost:8080` hors http.client.ts ; types générés présents (ParamsSmc, ParamsStraddle, Signal, PredictionML) ; 1 seule connexion WS signal-engine.
- [ ] **Vérif runtime (reportée au propriétaire)** : lancer l'app, confirmer (a) `horizon_bougies` affiche un nombre réel dans les vues Définition SMC/Straddle, (b) les notifications de signaux n'arrivent plus en double, (c) les pages Prompts IA / Pré-alertes / Analyse IA fonctionnent toujours (via apiService).
- [ ] Rapport au propriétaire. **Ne pas pousser.**

---

## Self-Review (post-rédaction)

**Spec coverage :** HTTP unifié (Task 3) ✓ ; drift horizon_bougies corrigé (Task 1) ✓ ; ts-rs 4 types (Task 1+2) ✓ ; store typé (Task 2) ✓ ; bugs bonus (Task 4) ✓.

**Risques résiduels :**
- **Task 2 store typing** : le renommage `atr_seuil`↔`seuil_atr` et la conversion bool↔number (`vente_partielle`) sont des pièges — le `npm run build` (vue-tsc) les attrapera si mal gérés. L'implementer doit préserver les computed de mapping existants.
- **Task 2 `Signal` migration** : le struct Rust `common::Signal` peut différer subtilement du type manuel (champs optionnels, casse). Aligner champ-par-champ ; le re-export préserve les importateurs.
- **Task 4 useChartAnalyse** : si le composable est utilisé ET qu'il faut un nouvel endpoint backend Ollama, c'est plus de travail — le plan prévoit de reporter la décision au propriétaire dans ce cas.
- **WS déduplication** : s'assurer qu'`App.vue` ne dépend pas de handlers spécifiques de sa connexion — sinon les déplacer vers useSignalEngine.

**Compteur tests :** backend reste 116 (+ éventuels nouveaux tests db sur horizon_bougies). Frontend : `npm run build` est le garde-fou (pas de suite de tests frontend).
