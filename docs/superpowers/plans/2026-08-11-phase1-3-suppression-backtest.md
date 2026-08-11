# Phase 1.3 — Suppression du backtest [D1] — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Supprimer toute trace du backtest de l'app (crate backend, handler, route, UI, types générés) — décision D1. Le backtest d'une stratégie se fait désormais en externe (TradingView/MT5/Python) avant intégration.

**Architecture:** Suppression pure (aucune relocalisation de type — l'investigation a prouvé que le couplage supposé `Recommandation`→Rockets n'existe pas : 3 structs homonymes indépendants). Préserver la route `/api/pre_alertes` (feature live) qui cohabitait dans `routes_backtest.rs`.

**Tech Stack:** Rust workspace (suppression d'un crate entier), Actix-Web, Vue 3/TS (suppression de vues/composants/types).

## Global Constraints

(Héritées du document de fondation §0 + plans 1.1/1.2.)

- **Zéro panic Vibe** : interdiction `unwrap()`/`panic!()`/`console.log()`. Ici : suppressions, mais respecter la règle.
- **Limite fichier** : ~600 lignes (D0 appliqué).
- **Test avant validation** : `cargo test --workspace` + `npm run build`. **Attendu : le compte de tests BAISSE** (le crate backtest embarque ~22 tests qui disparaissent avec lui). C'est normal et voulu.
- **Commit local par tâche** ; **push contrôlé par le propriétaire**.
- **L'assistant pose ses questions et attend la réponse** — il ne se répond jamais lui-même.

### Build backend — variables d'environnement requises (avant toute commande `cargo`)

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

### Suppressions backend
| Chemin | Quoi |
|--------|------|
| `backend/crates/backtest/` | **Crate entier** (9 fichiers .rs + Cargo.toml) |
| `backend/crates/api/src/backtest_handler.rs` | Handler `/api/backtest/lancer` |

### Éditions backend
| Chemin | Action |
|--------|--------|
| `backend/Cargo.toml:5` | Retirer `"crates/backtest",` du workspace |
| `backend/crates/api/Cargo.toml:15` | Retirer `backtest = { path = "../backtest" }` |
| `backend/crates/api/src/main.rs:8` | Retirer `mod backtest_handler;` |
| `backend/crates/api/src/main.rs:109` | Renommer `mod routes_backtest;` → `mod routes_prealerte;` |
| `backend/crates/api/src/routes_backtest.rs` | **Renommer** en `routes_prealerte.rs` + retirer la route backtest (**garder** `/api/pre_alertes`) |
| `backend/crates/api/src/routes.rs:295` | `routes_backtest::configurer` → `routes_prealerte::configurer` |

### Suppressions frontend (8 sources + 12 types générés)
- Sources : `views/BacktestView.vue`, `stores/backtest.store.ts`, `services/api.backtest.ts`, `composables/useBacktestColonnes.ts`, `components/common/{ColonneBacktest,BacktestEquityCurve,BacktestDistribRR,BacktestFenetresPropices}.vue`
- Generated (12) : `generated/{Recommandation,BacktestResult,BacktestConfig,StrategieParams,ParamsStraddle,ParamsSmc,StrategieType,ResultatTrade,TradeBacktest,StatHeure,StatJour,FenetrePropice}.ts`

### Éditions frontend (croisements)
`router/index.ts`, `SideBar.vue`, `StraddleCreneauxTable.vue`, `StraddleCreneauxBloc.vue`, `StraddleAnalyseModal.vue`, `services/api.straddle.ts`, `services/api.types.rockets.ts`, `AssetParamsPanel.vue`.

---

## ⚠ Décision ouverte (à trancher avant exécution)

Les champs `backtest_winrate` / `backtest_profit_factor` des créneaux Straddle existent côté backend **et DB** mais sont une **coïncidence de nommage** : ils sont alimentés par la boucle Straddle / PATCH utilisateur, **pas par le moteur backtest**. Supprimer le crate backtest **ne les casse pas**.

- **Backend + DB** : on **ne touche pas** (recommandé — ce sont juste des noms, les données restent valides).
- **UI** : la colonne « Backtest » + l'affichage WR/PF dans StraddleCreneauxTable/Bloc — **recommandation : retirer l'affichage** (le concept backtest disparaît de l'app). Les champs DB restent mais ne sont plus montrés. *Si tu veux garder l'affichage WR/PF sous un autre label, dis-le.*

---

## Task 0: Branche + baseline

- [ ] **Step 1 : Branche dédiée**

```bash
cd /mnt/IA/native-trading-ai
git checkout main
git checkout -b phase1-3-suppression-backtest
```

- [ ] **Step 2 : Baseline tests (référence = 138)**

```bash
cd backend
# (env CUDA sourcé)
cargo test --workspace 2>&1 | grep -E "test result: ok\." | awk '{s+=$4} END {print "Baseline: " s}'
```
Expected : **138**. Cible finale : **~116** (138 − ~22 tests du crate backtest supprimé).

---

## Task 1: Backend — casser la dépendance + supprimer handler/route/crate

### Étape 1.1 : Préserver pre_alertes (renommer routes_backtest.rs)

- [ ] **Renommer** `backend/crates/api/src/routes_backtest.rs` → `routes_prealerte.rs`, puis **éditer** pour ne garder que la route pre_alertes :

```rust
//! Routes dédiées aux pré-alertes (feature live).
//! (Le backtest a été supprimé — décision D1 ; la route /api/backtest/lancer est retirée.)
use actix_web::web;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/pre_alertes",
        web::get().to(crate::prealerte_handlers::get_pre_alertes),
    );
}
```

### Étape 1.2 : Supprimer le handler backtest

- [ ] `rm backend/crates/api/src/backtest_handler.rs`

### Étape 1.3 : Mettre à jour main.rs

- [ ] Retirer la ligne `mod backtest_handler;`
- [ ] Remplacer `mod routes_backtest;` par `mod routes_prealerte;`

### Étape 1.4 : Mettre à jour routes.rs

- [ ] Remplacer `crate::routes_backtest::configurer(cfg);` par `crate::routes_prealerte::configurer(cfg);`

### Étape 1.5 : Retirer la dépendance Cargo de api

- [ ] Dans `backend/crates/api/Cargo.toml`, retirer la ligne `backtest = { path = "../backtest" }`

### Étape 1.6 : Retirer le crate du workspace + supprimer le dossier

- [ ] Dans `backend/Cargo.toml`, retirer la ligne `    "crates/backtest",` du `[workspace] members`.
- [ ] `rm -rf backend/crates/backtest`

### Étape 1.7 : Build backend + tests

- [ ] `cargo build -p api --release` → `Finished` (aucune référence à `backtest::` ne doit rester).
- [ ] Vérifier : `grep -rn "backtest::" backend/crates/api/src/` → **vide**.
- [ ] `cargo test --workspace 2>&1 | grep -E "test result: ok\." | awk '{s+=$4} END {print s}'` → **~116** (138 − 22).
- [ ] **Commit** :

```bash
git add -A && git commit -m "feat(api)!: supprime le crate backtest + handler + route (D1)

Le backtest est retiré de l'app (décision D1 : modèle backtest externe).
- Supprimé backend/crates/backtest/ (9 fichiers) + backtest_handler.rs
- routes_backtest.rs → routes_prealerte.rs (préserve /api/pre_alertes)
- Retiré dépendance api/Cargo.toml + membre workspace
Aucune relocalisation de type nécessaire (Recommandation backtest ≠ Rockets).
Tests backend : ~116 (−22 tests du crate supprimé). Phase 1.3."
```

---

## Task 2: Frontend — suppression des fichiers backtest-only

- [ ] **Step 1 : Supprimer les 8 sources**

```bash
cd /mnt/IA/native-trading-ai/frontend
rm src/views/BacktestView.vue \
   src/stores/backtest.store.ts \
   src/services/api.backtest.ts \
   src/composables/useBacktestColonnes.ts \
   src/components/common/ColonneBacktest.vue \
   src/components/common/BacktestEquityCurve.vue \
   src/components/common/BacktestDistribRR.vue \
   src/components/common/BacktestFenetresPropices.vue
```

- [ ] **Step 2 : Supprimer les 12 types générés**

```bash
rm src/generated/Recommandation.ts src/generated/BacktestResult.ts \
   src/generated/BacktestConfig.ts src/generated/StrategieParams.ts \
   src/generated/ParamsStraddle.ts src/generated/ParamsSmc.ts \
   src/generated/StrategieType.ts src/generated/ResultatTrade.ts \
   src/generated/TradeBacktest.ts src/generated/StatHeure.ts \
   src/generated/StatJour.ts src/generated/FenetrePropice.ts
```

- [ ] **Step 3 : Supprimer les `.js`/`.vue.js` miroirs** (build artifacts gitignored, mais nettoyer le disque) — `find src -name "*.backtest.*" -o -name "BacktestView.vue.js" -o -name "backtest.store.js" -o -name "ColonneBacktest.vue.js"` à supprimer. (Optionnel : le build les régénérera/épurera.)

- [ ] **Step 4 : Vérifier qu'aucun import cassé** — `npm run build` va échouer si un fichier supprimé est encore importé. On corrige dans Task 3 (croisements). Ne PAS commettre encore.

---

## Task 3: Frontend — éditions chirurgicales (croisements)

### 3.1 Retirer la route + l'entrée sidebar

- [ ] `frontend/src/router/index.ts` : retirer la ligne `{ path: '/straddle/backtest', component: () => import('../views/BacktestView.vue') },`
- [ ] `frontend/src/components/common/SideBar.vue` : retirer la ligne `{ to: '/straddle/backtest', icone: '🧪', label: 'Backtest' },`

### 3.2 StraddleCreneauxTable.vue — retirer la colonne « Backtest »

- [ ] Retirer l'en-tête `<th class="text-center px-4 py-3">Backtest</th>` (ligne ~39).
- [ ] Retirer le `<td>` WR/PF correspondant (lignes ~83-90, le bloc `<template v-if="c.backtest_winrate != null">...</template>` + `<span v-else>–</span>`).

### 3.3 StraddleCreneauxBloc.vue — retirer l'affichage WR/PF + le tri

- [ ] Retirer les deux `<span>` WR/PF (lignes ~43-46).
- [ ] Retirer le terme `(b.backtest_winrate ?? 0) - (a.backtest_winrate ?? 0) ||` du comparateur de tri (ligne ~78).

### 3.4 StraddleAnalyseModal.vue — reformuler le texte

- [ ] Ligne ~98 : `→ Voir les créneaux &amp; backtest horaire complet` → `→ Voir les créneaux &amp; l'analyse horaire complète`

### 3.5 services/api.straddle.ts — retirer les champs backtest

- [ ] Ligne ~42 : `data: { statut?: string; backtest_winrate?: number; backtest_profit_factor?: number },` → `data: { statut?: string },`

### 3.6 services/api.types.rockets.ts — retirer les champs du type StraddleCreneau

- [ ] Retirer les 2 lignes `backtest_winrate: number | null` et `backtest_profit_factor: number | null` (~lignes 100-101).

### 3.7 AssetParamsPanel.vue — reformuler un commentaire

- [ ] Ligne ~179 : `// Persiste le capital dans le store global (backtesting + dimensionnement)` → `// Persiste le capital dans le store global (dimensionnement)`

### 3.8 Build frontend + commit

- [ ] `npm run build` → build OK (0 erreur TS, aucun import cassé).
- [ ] Vérifier : `grep -rni "backtest" frontend/src --include=*.ts --include=*.vue | grep -v node_modules` → ne doit plus retourner que d'éventuels commentaires légitimes (prompts) ou rien.
- [ ] **Commit** :

```bash
git add -A && git commit -m "feat(frontend)!: supprime toute la UI backtest (D1)

- Supprimé BacktestView, store, service, composable, 4 composants, 12 types générés
- Retiré route /straddle/backtest + entrée sidebar
- Édité StraddleCreneauxTable/Bloc (colonne+tri WR/PF), StraddleAnalyseModal
  (texte), api.straddle.ts + api.types.rockets.ts (champs backtest_*).
Champs DB backtest_winrate/PF conservés (coïncidence de nom, alimentés par
la boucle straddle). Phase 1.3."
```

---

## Task 4: Validation finale

- [ ] **Step 1 : Backend complet**

```bash
cd /mnt/IA/native-trading-ai/backend
cargo build --release 2>&1 | grep -E "Compiling api|Finished|error"
cargo test --workspace 2>&1 | grep -E "test result: ok\." | awk '{s+=$4} END {print "Tests workspace: " s}'
```
Expected : build OK ; tests **~116** (138 − 22 du crate backtest). 0 failed.

- [ ] **Step 2 : Frontend complet**

```bash
cd ../frontend && npm run build && npm run test
```
Expected : build OK.

- [ ] **Step 3 : Audit Vibe** (hook pre-commit passé à chaque commit).

- [ ] **Step 4 : Vérification runtime (reportée au propriétaire)** — lancer `./scripts/run.sh`, vérifier que l'app démarre, que la sidebar n'a plus « Backtest », que `/straddle` (créneaux) s'affiche sans la colonne Backtest, que `/api/pre_alertes` répond toujours (feature live préservée).

- [ ] **Step 5 : Rapport au propriétaire** — tests avant/après (138 → ~116), commits, vérifications runtime. **Ne pas pousser.**

---

## Self-Review (post-rédaction)

**Spec coverage (fondation §5 Phase 1.3 / D1) :**
- Suppression crate backtest → Task 1 ✓
- Suppression UI backtest → Task 2 + 3 ✓
- Préservation pre_alertes → Task 1.1 ✓
- Recommandation relocalisation → **N/A** (couplage inexistant prouvé)

**Placeholders :** aucun. Tous les chemin/éditions cités avec lignes ~ (les numéros exacts peuvent avoir légèrement bougé ; l'implementer relit les fichiers).

**Risques résiduels :**
- **Pre_alertes** : le piège majeur. Si l'implementer supprime `routes_backtest.rs` sans préserver pre_alertes → la feature live pré-alertes perd son endpoint. Task 1.1 le gère explicitement (renommer + éditer).
- **Compteur de tests** : il BAISSE (~116 vs 138). Ce n'est PAS une régression — ce sont les tests du crate supprimé. À expliquer clairement dans le rapport.
- **ts-rs** : après suppression du crate, les `generated/*.ts` ne sont plus régénérés. Task 2 Step 2 les supprime manuellement.
- **Straddle DB fields** (`backtest_winrate`/`PF`) : conservés (naming). Si l'implementer voit une erreur de compilation frontend liée à ces champs après édition des types, c'est qu'il en a manqué un usage — corriger.
- **`.js` miroirs** : gitignored, régénérés au build. Nettoyage disque optionnel.
