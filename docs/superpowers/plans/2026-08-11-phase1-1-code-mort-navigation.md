# Phase 1.1 — Code mort & navigation cassée — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Supprimer le code mort frontend (composant + fonctions dev jamais appelées), réparer la navigation cassée (2 liens vers routes inexistantes) et retirer 2 routes backend de debug jamais consommées — sans aucun changement de comportement pour les chemins vivants.

**Architecture:** Modifications purement soustractives (deletions + retraits de lignes), isolées, n' touchant aucune logique métier. Chaque tâche est indépendamment compilable et testable.

**Tech Stack:** Vue 3 + TypeScript + Vite (frontend) · Rust workspace + Actix-Web (backend) · Vibe Framework (validation).

## Global Constraints

(Copnées verbatim depuis le document de fondation §0 — s'appliquent à toute tâche.)

- **Zéro panic** : interdiction `unwrap()`/`panic!()`/`console.log()` (règle Vibe). Aucun ajout ici, mais à respecter.
- **Limite fichier** : ~600 lignes max sur la logique métier cohérente (D0).
- **Test avant validation** : toute tâche validée par `cargo test --workspace` + `npm run build` + `npm run test`. Audit Vibe : `.vibe/bin/audit.sh` si disponible.
- **Commit contrôlé par le propriétaire** : l'assistant **ne committe pas** sans le « Valide » du propriétaire. Les commandes `git commit` ci-dessous sont exécutées uniquement après validation.
- **Vérification de non-régression** : l'assistant exécute tous les tests après chaque changement et rapporte le résultat réel (jamais d'approximation).
- **Branche** : tout le travail se fait sur une branche dédiée, jamais sur `main`.

### Build backend — variables d'environnement requises

Le backend ne compile pas sans l'environnement CUDA/LibTorch. Avant toute commande `cargo`, sourcer ces variables (extraites de `scripts/run.sh`) :

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
# archive vide si manquante (workaround GCC 15)
[ -f ../.cargo-fake-libs/libstdc++fs.a ] || ar rcs ../.cargo-fake-libs/libstdc++fs.a
```

> Astuce : pour la vérification d'intégration finale, `./scripts/run.sh` configure tout et lance l'app.

---

## File Structure

| Fichier | Action | Responsabilité |
|---------|--------|----------------|
| `frontend/src/components/common/NavBar.vue` (+ `.vue.js`) | **Supprimer** | Composant de navigation mort (jamais importé par `App.vue`) |
| `frontend/src/components/common/SideBar.vue` | Modifier (retirer 1 ligne) | Retirer l'entrée cassée `/smc/analyser` |
| `frontend/src/views/StraddleView.vue` | Modifier (retirer 1 ligne) | Retirer le `RouterLink` cassé `/pnl` |
| `frontend/src/views/HeatmapView.vue` | Modifier (retirer ~5 blocs) | Retirer 3 fonctions dev mortes + 2 déclarations devenues inutilisées |
| `backend/crates/api/src/routes.rs` | Modifier (retirer 1 bloc) | Retirer la route debug `/api/smc/score-debug` |
| `backend/crates/api/src/routes_rockets.rs` | Modifier (retirer 1 bloc) | Retirer la route debug `/api/rockets/scan/debug` |

**Hors périmètre (explicitement) :** les fonctions handler backend `score_debug` et `scan_momentum_debug` restent en place (devenues code mort inoffensif, `pub fn` en binaire = aucun warning ; leur nettoyage est reporté à un balayage ultérieur). Les endpoints dev `/api/straddle/dev/*` restent (outils dev, hors scope).

---

## Task 0: Préparer la branche de travail

**Files:** aucune modification de code.

- [ ] **Step 1: Créer une branche dédiée depuis `main`**

```bash
cd /mnt/IA/native-trading-ai
git checkout main
git pull --ff-only 2>/dev/null || true
git checkout -b phase1-1-code-mort-navigation
```

- [ ] **Step 2: Vérifier l'état de départ (build + tests verts)**

```bash
cd frontend && npm run build && npm run test
```
Expected: build成功, tests passent (ou suite existante inchangée).

Puis backend (avec l'environnement CUDA sourcé, voir Global Constraints) :
```bash
cd ../backend && cargo test --workspace
```
Expected: `cargo test` compile et passe (ou reste dans l'état de référence). **Noter le nombre de tests au départ** — il devra être identique (ou supérieur) à la fin.

> Si le build de départ échoue pour une raison indépendante de ce plan, le signaler au propriétaire avant de continuer.

---

## Task 1: Supprimer le composant mort `NavBar.vue`

**Files:**
- Delete: `frontend/src/components/common/NavBar.vue`
- Delete: `frontend/src/components/common/NavBar.vue.js`

**Vérification préalable :** `NavBar` n'est importé nulle part (confirmé : `grep -rn "NavBar" frontend/src` ne renvoie que le fichier lui-même).

- [ ] **Step 1: Confirmer l'absence d'imports**

```bash
cd /mnt/IA/native-trading-ai/frontend
grep -rn "NavBar" src --include=*.vue --include=*.ts | grep -v "components/common/NavBar.vue"
```
Expected: **vide** (aucun import). Si non vide → NE PAS supprimer, signaler.

- [ ] **Step 2: Supprimer les fichiers**

```bash
rm src/components/common/NavBar.vue src/components/common/NavBar.vue.js
```

- [ ] **Step 3: Vérifier que le frontend compile**

```bash
npm run build
```
Expected: build réussi sans erreur d'import manquant.

- [ ] **Step 4: Validation (Vibe) + commit**

```bash
npm run test
# puis .vibe/bin/audit.sh si disponible
git add -A && git commit -m "chore(frontend): supprime composant mort NavBar

NavBar.vue n'était jamais importé par App.vue. Nettoyage phase 1.1."
```
> Commit exécuté uniquement après « Valide » du propriétaire.

---

## Task 2: Retirer l'entrée sidebar cassée `/smc/analyser`

**Files:**
- Modify: `frontend/src/components/common/SideBar.vue` (retirer la ligne 74)

La route `/smc/analyser` n'existe pas dans le router → lien 404. Décision propriétaire : retirer l'entrée (l'analyse SMC reste accessible via la modale existante).

- [ ] **Step 1: Retirer la ligne de navigation**

Dans `frontend/src/components/common/SideBar.vue`, supprimer exactement :

```
      { to: '/smc/analyser',  icone: '📊', label: 'Analyser' },
```

(le bloc `groupe: 'SMC'` conserve ses 2 autres liens : Signaux actifs + Lexique SMC.)

- [ ] **Step 2: Vérifier la compilation**

```bash
npm run build
```
Expected: build OK.

- [ ] **Step 3: Vérification visuelle rapide**

```bash
npm run dev
```
Ouvrir `http://localhost:1420`, section SMC de la sidebar → l'entrée « Analyser » a disparu, les 2 autres restent. Arrêter le dev server (`Ctrl+C`).

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "fix(frontend): retire entrée sidebar cassée /smc/analyser

La route /smc/analyser n'existe pas (404). L'analyse SMC reste
accessible via la modale d'analyse. Phase 1.1."
```

---

## Task 3: Retirer le lien cassé `/pnl` dans StraddleView

**Files:**
- Modify: `frontend/src/views/StraddleView.vue` (retirer la ligne du `RouterLink`)

La route `/pnl` n'existe pas. Le lien pointait vers « Backtest heure précise » — de toute façon ciblé pour suppression avec le backtest (D1).

- [ ] **Step 1: Retirer le `RouterLink`**

Dans `frontend/src/views/StraddleView.vue`, supprimer la ligne :

```
          <RouterLink to="/pnl" class="text-yellow-400 hover:underline ml-1">→ Backtest heure précise</RouterLink>
```

Le paragraphe conserve sa phrase descriptive : « Le LLM analyse l'historique OHLCV et identifie les créneaux récurrents de forte volatilité bidirectionnelle. »

- [ ] **Step 2: Vérifier la compilation**

```bash
npm run build
```
Expected: build OK, plus aucune référence à `/pnl`.

Vérifier : `grep -rn "/pnl" src` → **vide**.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "fix(frontend): retire lien cassé /pnl dans StraddleView

La route /pnl n'existe pas (404). Lien enlevé (backtest en cours de
suppression, D1). Phase 1.1."
```

---

## Task 4: Supprimer les fonctions dev mortes dans HeatmapView

**Files:**
- Modify: `frontend/src/views/HeatmapView.vue`

Les fonctions `seedDev`, `signalTestDev`, `cloturerTestDev` sont définies mais **jamais appelées** par le template (vérifié). Une fois retirées, `estDev` et `dernierSignalTestId` deviennent inutilisées → on les retire aussi.

- [ ] **Step 1: Retirer la déclaration `estDev` (devenue inutilisée)**

Supprimer la ligne :
```
const estDev = import.meta.env.DEV
```

- [ ] **Step 2: Retirer la déclaration `dernierSignalTestId` (devenue inutilisée)**

Supprimer la ligne :
```
const dernierSignalTestId = ref<string | null>(null)
```

- [ ] **Step 3: Retirer la fonction `seedDev`**

Supprimer tout le bloc :
```ts
async function seedDev() {
  const assetCible = classementVol.value[0]?.asset ?? assets.value[0] ?? 'BTC'
  try {
    const res = await apiService.seedStraddleCreneauxDev(assetCible)
    alerteStore.afficherSucces(`Seed dev ${res.asset}: ${res.inserted} créneaux`) 
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Seed dev: ${(e as Error).message}`)
  }
}
```

- [ ] **Step 4: Retirer la fonction `signalTestDev`**

Supprimer tout le bloc :
```ts
async function signalTestDev() {
  const assetCible = classementVol.value[0]?.asset ?? assets.value[0] ?? 'BTC'
  try {
    const res = await apiService.creerSignalStraddleTestDev(assetCible, 'M15')
    dernierSignalTestId.value = res.signal_id
    alerteStore.afficherSucces(`Signal test créé: ${res.signal_id.slice(0, 8)}…`) 
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Signal test: ${(e as Error).message}`)
  }
}
```

- [ ] **Step 5: Retirer la fonction `cloturerTestDev`**

Supprimer tout le bloc :
```ts
async function cloturerTestDev() {
  if (!dernierSignalTestId.value) return
  try {
    await apiService.cloturerFeedbackStraddleTest(dernierSignalTestId.value, 'tp1', 101)
    alerteStore.afficherSucces('Feedback test clôturé (tp1)')
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Clôture test: ${(e as Error).message}`)
  }
}
```

- [ ] **Step 6: Vérifier qu'il ne reste aucune référence**

```bash
cd /mnt/IA/native-trading-ai/frontend
grep -n "seedDev\|signalTestDev\|cloturerTestDev\|estDev\|dernierSignalTestId" src/views/HeatmapView.vue
```
Expected: **vide**.

- [ ] **Step 7: Vérifier la compilation + tests**

```bash
npm run build && npm run test
```
Expected: build OK, tests inchangés.

> Note : les méthodes `apiService.seedStraddleCreneauxDev` / `creerSignalStraddleTestDev` / `cloturerFeedbackStraddleTest` peuvent devenir inutilisées dans `apiService` ; hors scope (elles servent peut-être ailleurs). Ne pas les retirer ici.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "chore(frontend): supprime fonctions dev mortes HeatmapView

seedDev/signalTestDev/cloturerTestDev n'étaient jamais appelées par le
template. Retire aussi estDev et dernierSignalTestId devenus inutilisés.
Phase 1.1."
```

---

## Task 5: Retirer les routes backend debug mortes

**Files:**
- Modify: `backend/crates/api/src/routes.rs` (retirer le bloc `/api/smc/score-debug`)
- Modify: `backend/crates/api/src/routes_rockets.rs` (retirer le bloc `/api/rockets/scan/debug`)

Ces 2 endpoints de debug ne sont jamais appelés par le frontend. On retire l'enregistrement de la route uniquement (les handlers restent, code mort inoffensif).

- [ ] **Step 1: Retirer la route `/api/smc/score-debug` dans `routes.rs`**

Supprimer le bloc :
```rust
        .service(
            web::resource("/api/smc/score-debug")
                .route(web::get().to(crate::smc_handlers::score_debug)),
        )
```

- [ ] **Step 2: Retirer la route `/api/rockets/scan/debug` dans `routes_rockets.rs`**

Supprimer le bloc :
```rust
    .route(
        "/api/rockets/scan/debug",
        web::get().to(crate::rockets_handlers::scan_momentum_debug),
    )
```

- [ ] **Step 3: Vérifier la compilation backend**

```bash
cd /mnt/IA/native-trading-ai/backend
# (environnement CUDA sourcé — voir Global Constraints)
cargo build -p api --release
```
Expected: `Compiling api ... Finished` sans erreur. La chaîne d'appels `.service(...)/.route(...)` reste valide.

- [ ] **Step 4: Vérifier les tests backend**

```bash
cargo test --workspace
```
Expected: nombre de tests ≥ nombre de référence (Task 0). Aucune régression.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "chore(api): retire routes debug mortes score-debug et scan/debug

Endpoints jamais consommés par le frontend. Handlers laissés en place
(code mort inoffensif). Phase 1.1."
```

---

## Task 6: Validation finale d'intégration

**Files:** aucune.

- [ ] **Step 1: Build complet + tests complets**

```bash
cd /mnt/IA/native-trading-ai
cd frontend && npm run build && npm run test && cd ..
cd backend && cargo test --workspace && cd ..
```
Expected: tout vert.

- [ ] **Step 2: Audit Vibe (si disponible)**

```bash
.vibe/bin/audit.sh
```
Expected: 🟢 VERT. (Si le script n'existe pas/failit pour une raison indépendante, le signaler.)

- [ ] **Step 3: Lancer l'app et vérifier les chemins vivants**

```bash
./scripts/run.sh
```
Vérifier dans la fenêtre native :
- Dashboard s'affiche.
- Sidebar : section SMC n'a plus « Analyser » ; les autres entrées fonctionnent.
- Vue Straddle (`/straddle`) : le lien `/pnl` a disparu, le tableau des créneaux s'affiche.
- Vue Heatmap (`/heatmap`) : s'affiche, boutons « Actualiser » et onglets ATR/Horaire fonctionnels (les boutons dev, s'ils existaient visuellement, doivent être absents — sinon, c'est qu'ils étaient câblés et il faut ré-examiner).

Arrêter avec `Ctrl+C`.

- [ ] **Step 4: Rapport au propriétaire**

Présenter : nombre de tests avant/après, résultat build, résultat audit, et la liste des vérifications visuelles. **Ne pas pousser.**

- [ ] **Step 5: (Optionnel) Fusionner la branche — décision propriétaire**

Le propriétaire décide de fusionner `phase1-1-code-mort-navigation` vers `main` et/ou de pousser sur GitHub.

---

## Self-Review (post-rédaction)

**Spec coverage (fondation §5 Phase 1, items de code mort + nav) :**
- Code mort frontend (NavBar) → Task 1 ✓
- Code mort backend (routes dev) → Task 5 ✓
- Fonctions dev mortes (HeatmapView) → Task 4 ✓
- Navigation cassée (`/smc/analyser`, `/pnl`) → Task 2, 3 ✓

**Placeholders :** aucun. Tous les blocs de code à supprimer sont cités intégralement.

**Cohérence des types :** aucune signature modifiée (pur retrait). `cargo test --workspace` garantit la non-régression backend ; `npm run build` garantit le frontend.

**Risques résiduels :**
- Si un bouton du template HeatmapView était câblé à une fonction dev (non détecté), le build TS échouerait → Step 6/7 le rattrape.
- Les handlers backend laissés (`score_debug`, `scan_momentum_debug`) sont inoffensifs (code mort balayable plus tard).
