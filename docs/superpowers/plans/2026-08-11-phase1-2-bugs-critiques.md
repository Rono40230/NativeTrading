# Phase 1.2 — Bugs critiques — Plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Corriger 3 bugs critiques backend identifiés à l'audit — démarrage double des boucles SMC/Straddle, fuite mémoire `String::leak()`, et params SMC ignorés en live — chacun avec un test anti-régression.

**Architecture:** 3 corrections indépendantes (fichiers disjoints), chacune testable unitairement. Bug 1 touche `main.rs` + les 3 fonctions `demarrer_*` ; Bug 2 touche `rockets_niveaux.rs` ; Bug 3 touche `smc_boucle.rs`. Aucune logique métier ajoutée — corrections de bugs existants.

**Tech Stack:** Rust workspace, Actix-Web, Tokio (async), SQLite (SQLx). Tests `cargo test`.

## Global Constraints

(Héritées du document de fondation §0 + plan 1.1.)

- **Zéro panic** : interdiction `unwrap()`/`panic!()`/`console.log()` (Vibe). Les corrections utilisent `?`, `match`, `unwrap_or`.
- **Limite fichier** : ~600 lignes (D0).
- **Test avant validation** : `cargo test --workspace` doit rester vert (131 tests actuels + les nouveaux). Audit Vibe via le hook pre-commit.
- **Commit local par tâche** (décision propriétaire) ; **push contrôlé par le propriétaire** (jamais sans son feu vert).
- **Nouvelle règle** : l'assistant pose ses questions au propriétaire et **attend sa réponse** — il ne se répond jamais lui-même.

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

| Fichier | Action | Bug |
|---------|--------|-----|
| `backend/crates/api/src/main.rs` | Modifier (supprimer les spawns doublons ~lignes 170-185) | 1 |
| `backend/crates/api/src/smc_boucle.rs` | Modifier (garde idempotence sur `demarrer_boucle_smc`) | 1 |
| `backend/crates/api/src/straddle_boucle.rs` | Modifier (garde idempotence sur `demarrer_boucle_straddle`) | 1 |
| `backend/crates/api/src/scheduler.rs` | Modifier (garde idempotence sur `demarrer_surveillance_ml`) | 1 |
| `backend/crates/strategies/src/rockets_niveaux.rs` | Modifier (remplacer `.leak()` ligne 106 + tests) | 2 |
| `backend/crates/api/src/smc_boucle.rs` | Modifier (charger `SmcParams`, extraire `calculer_sl_tp` + test) | 3 |

> Bug 1 et Bug 3 touchent tous deux `smc_boucle.rs` mais des **zones distinctes** (haut du fichier pour la garde, lignes ~203-245 pour les params). Séquencer Task 1 avant Task 3.

---

## Task 0: Préparer la branche + baseline

- [ ] **Step 1: Branche dédiée depuis `main`**

```bash
cd /mnt/IA/native-trading-ai
git checkout main
git checkout -b phase1-2-bugs-critiques
```

- [ ] **Step 2: Baseline tests (référence)**

```bash
cd backend
# (environnement CUDA sourcé — voir Global Constraints)
cargo test --workspace 2>&1 | grep -E "test result:"
```
Expected : **131 passed, 0 failed, 1 ignored** (référence). Noter ce nombre — il devra être ≥ à la fin (nouveaux tests en plus).

---

## Task 1: Bug 1 — Supprimer le double-démarrage des boucles

**Files:**
- Modify: `backend/crates/api/src/main.rs` (supprimer les spawns doublons)
- Modify: `backend/crates/api/src/smc_boucle.rs` (garde idempotence)
- Modify: `backend/crates/api/src/straddle_boucle.rs` (garde idempotence)
- Modify: `backend/crates/api/src/scheduler.rs` (garde idempotence)

**Problème :** `AppState::new()` (`state.rs:106,109-114,137-142`) démarre déjà la surveillance ML + boucles SMC + Straddle. `main.rs:170-185` les redémarre → 2 tâches Tokio par boucle (charge doublée, races).

**Décision :** supprimer les doublons de `main.rs` (qui violent l'encapsulation de `AppState::new`) + ajouter une garde idempotence (anti-régression, pour qu'un futur double-start soit sans effet).

### Étape 1.1 : Supprimer les spawns doublons dans `main.rs`

- [ ] **Lire `main.rs` lignes 165-190** pour confirmer l'emplacement exact (les blocs `smc_boucle::demarrer_boucle_smc(...)`, `straddle_boucle::demarrer_boucle_straddle(...)`, `scheduler::demarrer_surveillance_ml(...)`).

- [ ] **Supprimer ces 3 blocs** et remplacer par un commentaire :

```rust
    // ── Boucles automatiques ─────────────────────────────────────────────────
    // Rappel : SMC + Straddle + surveillance ML sont DÉJÀ démarrés par
    // AppState::new() (voir state.rs). Ne pas les relancer ici (sinon
    // double-spawn → charge doublée + races). Garde idempotence dans chaque
    // demarrer_* au cas où.
```

### Étape 1.2 : Garde idempotence sur `demarrer_boucle_smc` (`smc_boucle.rs`)

- [ ] **Ajouter en haut de `smc_boucle.rs`** (après les `use` existants) :

```rust
use std::sync::atomic::{AtomicBool, Ordering};

/// Garde anti-double-start. La boucle SMC doit n'être spawnée qu'une fois
/// (déjà lancée par AppState::new). Un second appel est un no-op + warning.
static SMC_DEMARREE: AtomicBool = AtomicBool::new(false);

/// Marque la boucle SMC comme démarrée. Retourne `true` si c'est le premier
/// appel (le spawn doit avoir lieu), `false` sinon. Fonction pure → testable.
fn marquer_smc_demarree() -> bool {
    !SMC_DEMARREE.swap(true, Ordering::SeqCst)
}
```

- [ ] **Au début du corps de `demarrer_boucle_smc`** (juste après la signature), ajouter la garde :

```rust
    if !marquer_smc_demarree() {
        tracing::warn!("⚠️  Boucle SMC déjà démarrée — second spawn ignoré");
        return;
    }
```

### Étape 1.3 : Même garde sur `demarrer_boucle_straddle` (`straddle_boucle.rs`)

- [ ] Ajouter en haut (après les `use`) :

```rust
use std::sync::atomic::{AtomicBool, Ordering};
static STRADDLE_DEMARREE: AtomicBool = AtomicBool::new(false);
fn marquer_straddle_demarree() -> bool {
    !STRADDLE_DEMARREE.swap(true, Ordering::SeqCst)
}
```

- [ ] Ajouter au début du corps de `demarrer_boucle_straddle` :

```rust
    if !marquer_straddle_demarree() {
        tracing::warn!("⚠️  Boucle Straddle déjà démarrée — second spawn ignoré");
        return;
    }
```

### Étape 1.4 : Même garde sur `demarrer_surveillance_ml` (`scheduler.rs`)

- [ ] Ajouter en haut (après les `use`) :

```rust
use std::sync::atomic::{AtomicBool, Ordering};
static SURVEILLANCE_ML_DEMARREE: AtomicBool = AtomicBool::new(false);
fn marquer_surveillance_demarree() -> bool {
    !SURVEILLANCE_ML_DEMARREE.swap(true, Ordering::SeqCst)
}
```

- [ ] Ajouter au début du corps de `demarrer_surveillance_ml` :

```rust
    if !marquer_surveillance_demarree() {
        tracing::warn!("⚠️  Surveillance ML déjà démarrée — second appel ignoré");
        return;
    }
```

### Étape 1.5 : Test anti-régression (idempotence)

Les fonctions `marquer_*_demarree` sont pures et testables. L'état global static nécessite de sérialiser les tests (un `#[serial]` serait idéal, mais pour éviter une nouvelle dépendance, on groupe les assertions dans un seul test).

- [ ] **Ajouter un module de test dans `smc_boucle.rs`** :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marquer_smc_demarree_est_idempotente() {
        // RESET (l'état static persiste entre les tests d'un même binaire ;
        // on le remet à false pour ce test isolé).
        SMC_DEMARREE.store(false, Ordering::SeqCst);

        // Premier appel : doit retourner true (autorise le spawn).
        let premier = marquer_smc_demarree();
        // Second appel : doit retourner false (no-op).
        let second = marquer_smc_demarree();

        assert!(premier, "Le premier appel doit autoriser le démarrage");
        assert!(!second, "Le second appel doit être un no-op (anti-double-spawn)");

        // Cleanup pour ne pas polluer les autres tests.
        SMC_DEMARREE.store(false, Ordering::SeqCst);
    }
}
```

- [ ] **Vérifier que le test compile et passe** :

```bash
cd backend && cargo test -p api demarrer_smc -- --nocapture
```
Expected : `test tests::marquer_smc_demarree_est_idempotente ... ok`.

### Étape 1.6 : Build + tests workspace + commit

- [ ] `cargo build -p api --release` → `Finished`.
- [ ] `cargo test --workspace 2>&1 | grep -E "test result:"` → 132 passed (131 + 1 nouveau), 0 failed.
- [ ] **Commit** :

```bash
git add -A && git commit -m "fix(api): supprime double-démarrage boucles SMC/Straddle + garde idempotence

main.rs redémarrait les boucles déjà lancées par AppState::new (state.rs),
créant 2 tâches Tokio par boucle (charge doublée + races). Supprimé les
doublons de main.rs. Ajouté une garde AtomicBool idempotente sur les 3
demarrer_* (smc/straddle/surveillance ML) + test anti-régression. Phase 1.2."
```

---

## Task 2: Bug 2 — Fuite mémoire `String::leak()`

**Files:**
- Modify: `backend/crates/strategies/src/rockets_niveaux.rs` (ligne 106 + tests)

**Problème :** `Verdict::ClotureTotale { label, .. } => Some(label.leak())` (ligne 106) transforme une `String` en `&'static str` sans libération. Le worker Rockets tourne toutes les 3 min → fuite cumulative monotone.

**Décision :** option (a) — les seuls labels atteignant la ligne 106 sont `"tp1"`, `"be"`, `"sl"` (ensemble fini prouvé par lecture de `position_tracking.rs`). On remplace le `.leak()` par un `match label.as_str()` retournant des littéraux `&'static str`. **Signature `Option<&'static str>` inchangée → 0 appelant modifié.**

### Étape 2.1 : Remplacer le bras fautif

- [ ] **Dans `rockets_niveaux.rs`**, remplacer la ligne 106 :

Avant (bras du match, ligne 106 uniquement) :
```rust
    Verdict::ClotureTotale { label, .. }                           => Some(label.leak()),
```

Après :
```rust
    Verdict::ClotureTotale { label, .. } => match label.as_str() {
        "tp1" => Some("tp1"),
        "be"  => Some("be"),
        "sl"  => Some("sl"),
        // Les labels "trailing"/"invalide" sont interceptés plus haut.
        // Garde défensif si position_tracking évolue :
        autre => {
            tracing::warn!(
                "calculer_verdict_rocket: label ClotureTotale inattendu '{}', \
                 traité comme 'sl'", autre
            );
            Some("sl")
        }
    },
```

> Vérifier que `tracing` est déjà importé dans le fichier (il l'est généralement). Si non, ajouter `use tracing::warn;` ou `tracing::warn!`.

### Étape 2.2 : Test anti-régression (pas de leak = littéral interné)

- [ ] **Ajouter/modifier le module de test dans `rockets_niveaux.rs`** :

```rust
#[cfg(test)]
mod tests_leak {
    use super::*;

    /// Un littéral &'static str est interné dans .rodata : deux appels
    /// retournent le MÊME pointeur. Une String leakée serait une nouvelle
    /// allocation tas à chaque appel → pointeurs différents. Ce test échoue
    /// si quelqu'un reintroduit `.leak()` ou `.to_string().leak()`.
    #[test]
    fn verdict_retourne_un_litteral_partage_pas_une_string_leakee() {
        // On appelle calculer_verdict_rocket sur un scénario qui atteint
        // le bras ClotureTotale (label "tp1"/"be"/"sl"). Construire un
        // RocketSignal + (prix, peak, peak_precedent) qui tombe sur ce bras.
        //
        // NOTE : reprendre un setup existant de tests Rockets déjà présents
        // dans strategies/ (cf. tests/rockets_indicateurs.rs) pour bâtir
        // un RocketSignal minimal valide. Si aucun helper n'existe, en créer
        // un petit `fn signal_min(...) -> RocketSignal` dans ce module.
        //
        // Scénario menant à ClotureTotale label "sl" :
        //   position LONG, peak < TP1, prix <= stop_loss.
        let s = signal_min_long(prix_entree: 100.0, stop_loss: 90.0, /* tp1, tp2, atr14... */);
        let v1 = calculer_verdict_rocket(&s, prix_sous_sl, peak_bas, peak_bas);
        let v2 = calculer_verdict_rocket(&s, prix_sous_sl, peak_bas, peak_bas);

        assert!(v1.is_some(), "Le scénario doit produire un verdict");
        let p1 = v1.unwrap().as_ptr();
        let p2 = v2.unwrap().as_ptr();
        assert_eq!(p1, p2,
            "Le label doit être un littéral .rodata partagé (pas une String leakée)");
    }
}
```

> ⚠ Le setup exact du `RocketSignal` doit être adapté aux champs réels du struct (lire `db::rockets::RocketSignal`). L'implementer reprendra les helpers existants dans `backend/crates/strategies/tests/`. **Le verrou anti-fuite est le test de pointeur (`p1 == p2`) — c'est lui qui échouerait si `.leak()` revenait.**

- [ ] Compléter `signal_min_long(...)` avec les champs réels (lire la définition de `RocketSignal`). Si un helper similaire existe déjà dans `tests/rockets_indicateurs.rs`, l'importer/le dupliquer.

### Étape 2.3 : Build + tests + commit

- [ ] `cargo build -p strategies --release` → `Finished`.
- [ ] `cargo test -p strategies 2>&1 | grep -E "test result:"` → tests strategies passent (12 + nouveau).
- [ ] `cargo test --workspace 2>&1 | grep -E "test result:"` → 133 passed cumulés, 0 failed.
- [ ] **Commit** :

```bash
git add -A && git commit -m "fix(strategies): fuite mémoire String::leak dans calculer_verdict_rocket

Le bras ClotureTotale (rockets_niveaux.rs:106) fuyait une String par
appel via .leak() (worker 3-min → fuite cumulative). Remplacé par un
match sur littéraux &'static str (les labels sont un ensemble fini
tp1/be/sl). Signature inchangée, 0 appelant modifié. Test anti-régression
sur l'internage des littéraux. Phase 1.2."
```

---

## Task 3: Bug 3 — Params SMC ignorés en live (SL/TP hardcodés)

**Files:**
- Modify: `backend/crates/api/src/smc_boucle.rs` (charger `SmcParams`, extraire `calculer_sl_tp` + test)

**Problème :** `analyser_asset` (`smc_boucle.rs:209-218`) calcule `sl = prix ± atr14` (×1.0) et `tp1 = prix ± atr14*1.5` (×1.5) en dur, ignorant `SmcParams.atr_sl` (défaut 1.0) et `atr_tp1` (défaut 2.0). Conséquence : TP1 faux (1.5 au lieu de 2.0 par défaut) et le paramétrage utilisateur via `PUT /api/strategies/smc` est silencieusement inopérant sur cette boucle.

**Décision :** reproduire le pattern canonique de `signal_engine_analyse.rs:38-40` (`let smc_params = lire_smc_params(db.pool()).await;`).

### Étape 3.1 : Extraire `calculer_sl_tp` (fonction pure testable)

- [ ] **Ajouter dans `smc_boucle.rs`** (hors de `analyser_asset`) :

```rust
/// Calcule (SL, TP1) à partir du prix, de l'ATR et des params SMC.
/// Extrait de `analyser_asset` pour être testable indépendamment de la DB/ML.
/// Retourne None si la direction est `Both` (pas de SL/TP univoque).
fn calculer_sl_tp(
    direction: common::Direction,
    prix: f64,
    atr14: f64,
    params: &db::strategies_params::SmcParams,
) -> Option<(f64, f64)> {
    match direction {
        common::Direction::Long => Some((
            prix - atr14 * params.atr_sl,
            prix + atr14 * params.atr_tp1,
        )),
        common::Direction::Short => Some((
            prix + atr14 * params.atr_sl,
            prix - atr14 * params.atr_tp1,
        )),
        common::Direction::Both => None,
    }
}
```

### Étape 3.2 : Remplacer le calcul hardcodé dans `analyser_asset`

- [ ] **Dans `analyser_asset`**, remplacer les lignes 209-218 (les blocs `let sl = if score.direction == ... atr14 ...` et `let tp1 = ... atr14 * 1.5`) par :

```rust
    // Paramètres SMC depuis la DB (même pattern que signal_engine_analyse.rs:39).
    // Évite un hardcodage 1.0/1.5 qui ignorait le paramétrage utilisateur.
    let smc_params = db::strategies_params::lire_smc_params(db.pool()).await;
    let (sl, tp1) = match calculer_sl_tp(score.direction, prix, atr14, &smc_params) {
        Some(v) => v,
        None => return Ok(None), // direction Both : pas de signal univoque
    };
```

> Vérifier que `db` est bien en scope dans `analyser_asset` (signature `db: Arc<Database>`) et que `db.pool()` retourne `&SqlitePool`. Vérifier l'import de `db::strategies_params` (ajouter `use db::strategies_params;` en haut si absent). Conserver l'usage ultérieur de `sl`/`tp1` dans la construction du `ParamsSmc` (lignes ~220-245) — inchangé.

### Étape 3.3 : Test

- [ ] **Ajouter au module `tests` de `smc_boucle.rs`** (celui créé en Task 1) :

```rust
    #[test]
    fn calculer_sl_tp_utilise_les_params_pas_hardcodes() {
        // Params custom : atr_sl = 2.0, atr_tp1 = 3.0
        let params = db::strategies_params::SmcParams {
            atr_sl: 2.0,
            atr_tp1: 3.0,
            ..Default::default()
        };
        let (sl, tp1) = calculer_sl_tp(common::Direction::Long, 100.0, 1.0, &params).unwrap();
        // SL = prix - 2.0*atr = 98.0 (et NON 99.0 = prix - 1.0*atr hardcodé)
        assert!((sl - 98.0).abs() < 1e-9, "SL doit utiliser atr_sl=2.0 → 98.0, obtenu {}", sl);
        // TP1 = prix + 3.0*atr = 103.0 (et NON 101.5 = prix + 1.5*atr hardcodé)
        assert!((tp1 - 103.0).abs() < 1e-9, "TP1 doit utiliser atr_tp1=3.0 → 103.0, obtenu {}", tp1);
    }

    #[test]
    fn calculer_sl_tp_valeurs_par_defaut_sont_correctes() {
        // Garde anti-régression : avec les défauts (atr_sl=1.0, atr_tp1=2.0),
        // TP1 = 102.0 (et non 101.5 comme l'ancien code hardcodé).
        let def = db::strategies_params::SmcParams::default();
        let (_, tp1) = calculer_sl_tp(common::Direction::Long, 100.0, 1.0, &def).unwrap();
        assert!((tp1 - 102.0).abs() < 1e-9, "TP1 par défaut doit être 2.0*atr = 102.0 (était 101.5 avant fix)");
    }

    #[test]
    fn calculer_sl_tp_direction_short_est_symetrique() {
        let params = db::strategies_params::SmcParams { atr_sl: 2.0, atr_tp1: 3.0, ..Default::default() };
        let (sl, tp1) = calculer_sl_tp(common::Direction::Short, 100.0, 1.0, &params).unwrap();
        assert!((sl - 102.0).abs() < 1e-9, "SL Short = prix + atr_sl = 102.0");
        assert!((tp1 - 97.0).abs() < 1e-9, "TP1 Short = prix - atr_tp1 = 97.0");
    }
```

- [ ] **Vérifier que `common::Direction::Both` retourne None** est couvert par l'un des tests (ajouter si besoin) :

```rust
    #[test]
    fn calculer_sl_tp_direction_both_retourne_none() {
        let params = db::strategies_params::SmcParams::default();
        assert!(calculer_sl_tp(common::Direction::Both, 100.0, 1.0, &params).is_none());
    }
```

### Étape 3.4 : Build + tests + commit

- [ ] `cargo build -p api --release` → `Finished`.
- [ ] `cargo test -p api 2>&1 | grep -E "test result:"` → tests api passent (26 + 3-4 nouveaux).
- [ ] `cargo test --workspace 2>&1 | grep -E "test result:"` → ~136-137 passed cumulés, 0 failed.
- [ ] **Commit** :

```bash
git add -A && git commit -m "fix(api): boucle SMC utilise SmcParams (atr_sl/atr_tp1) au lieu de hardcodes

smc_boucle.rs hardcodait SL=1.0*ATR et TP1=1.5*ATR, ignorant la table
smc_params (TP1 faux + paramétrage utilisateur inopérant). Désormais
charge SmcParams via lire_smc_params (pattern signal_engine_analyse.rs).
Logique extraite dans calculer_sl_tp() pure + 4 tests anti-régression.
Phase 1.2."
```

---

## Task 4: Validation finale d'intégration

- [ ] **Step 1 : Build + tests workspace complet**

```bash
cd /mnt/IA/native-trading-ai/backend
cargo test --workspace 2>&1 | grep -E "test result:"
```
Expected : tous `ok`, total passed ≥ 131 (référence) + ~5-6 nouveaux tests (idempotence + leak + sl/tp). 0 failed.

- [ ] **Step 2 : Frontend inchangé (sanity check)**

```bash
cd ../frontend && npm run build
```
Expected : build OK (aucun changement frontend, vérification de non-régression globale).

- [ ] **Step 3 : Audit Vibe (hook pre-commit déjà passé à chaque commit)** — rappeler le résultat.

- [ ] **Step 4 : Vérification runtime (reportée au propriétaire)** — lancer `./scripts/run.sh` et vérifier dans `data/logs/backend.log` :
  - une seule ligne « 📐 Boucle SMC Directionnel démarrée » (pas deux),
  - une seule ligne « Boucle Straddle démarrée »,
  - une seule ligne « Surveillance ML démarrée ».
  Si deux lignes apparaissent, la garde idempotence a loggé un warning « déjà démarrée » (attendu si AppState::new ET un autre code lancent — mais main.rs ne le fait plus).

- [ ] **Step 5 : Rapport au propriétaire** — tests avant/après, commits, vérifications runtime. **Ne pas pousser.**

---

## Self-Review (post-rédaction)

**Spec coverage (fondation §5 Phase 1, items bugs critiques) :**
- Double-start boucles → Task 1 ✓
- Fuite `String::leak()` → Task 2 ✓
- Params SMC ignorés → Task 3 ✓

**Placeholders :** le test du Bug 2 demande de bâtir un `RocketSignal` minimal — les champs exacts doivent être lus dans `db::rockete::RocketSignal` par l'implementer (signalé explicitement dans la step, avec renvoi aux helpers existants `tests/rockets_indicateurs.rs`). Ce n'est pas un TODO vague mais une instruction précise avec source.

**Cohérence des types :**
- `marquer_*_demarree() -> bool` défini et utilisé de manière cohérente.
- `calculer_sl_tp(...) -> Option<(f64,f64)>` cohérent entre définition (3.1) et usage (3.2) et tests (3.3).
- `SmcParams` chargé via `lire_smc_params(db.pool())` dont la signature est `async fn lire_smc_params(pool: &SqlitePool) -> SmcParams` (vérifié dans l'investigation).

**Risques résiduels :**
- Bug 2 test setup : nécessite un `RocketSignal` valide ; si les helpers existants ne suffisent pas, l'implementer devra en construire un (effort modéré). Le test de pointeur reste le verrou clé.
- Les gardes idempotence reposent sur un état static global (réinitialisé dans les tests via `.store(false)`). Acceptable en l'absence de parallélisme de tests sur ces modules.
- `main.rs` peut avoir d'autres références aux modules `smc_boucle`/`straddle_boucle`/`scheduler` (déclarations `mod`) — à conserver (ce sont les déclarations de module, pas les appels de démarrage).
