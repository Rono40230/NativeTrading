# 🗺️ ROADMAP — Native Trading AI
> Dernière mise à jour : 15 avril 2026

## � RÈGLES ABSOLUES (non négociables)

### Règle 1 — Zéro régression
Avant toute implémentation, analyser les conséquences sur le code existant :
- Lister les fichiers impactés (directs et indirects)
- Identifier les appels entrants vers les fonctions modifiées
- Vérifier que les signatures d'API REST ne changent pas sans migration
- Lancer `cargo build --workspace` + `cargo test --workspace` après chaque modification
- Si un test existant échoue → corriger AVANT de continuer, jamais ignorer

### Règle 2 — Explication avant action
Pour chaque tâche non triviale, fournir AVANT de toucher au code :
1. **Ce qui va être modifié** : liste des fichiers + fonctions concernées
2. **Pourquoi** : justification métier ou technique
3. **Risques identifiés** : ce qui pourrait casser
4. **Plan de rollback** : comment revenir si ça échoue
→ L'utilisateur valide (explicitement ou par silence >30s) avant l'implémentation.

### Règle 3 - WORKFLOW (rappel)

Chaque item coché = audit obligatoire avant commit :
```bash
./.vibe/bin/audit.sh        # Clippy + tests + taille fichiers + zero-unwrap
cargo test --workspace      # Tous les tests backend
cd frontend && npm run test # Tests Vue
```

Signal d'alerte :
- Fichier ≥ 250 lignes → split immédiat
- `unwrap()` / `console.log()` → bloquant
- Calcul métier côté Vue → interdit (tout passe par le backend)

---

## 🚀 PLAN — Money Management (Risk Engine)

> Implémentation par ordre de priorité logique. Chaque étape est un prérequis de la suivante.

---

### Priorité 1 — Table `positions_ouvertes` (fondation DB)

## 🚀 PLAN — Suivi de performance & Entraînement LLM + ML

> **Objectif unique de l'app** : que le système apprenne de chaque signal émis pour produire des signaux de plus en plus pertinents.
>
> **Règle absolue** : les 3 stratégies (Rockets, Straddle, SMC) ont des apprentissages **strictement indépendants**. Aucune donnée, aucun modèle, aucun prompt ne se croise entre stratégies.
>
> **3 boucles d'apprentissage parallèles (une par stratégie) :**
> - **Boucle ML** : XGBoost/LSTM ré-entraîné sur trades clôturés réels → meilleur filtre quantitatif
> - **Boucle LLM** : contexte enrichi avec feedbacks réels → meilleure analyse qualitative
> - **Boucle calibration** : paramètres de détection ajustés depuis les corrélations → meilleur générateur

---

### ⚠️ PROBLÈME ARCHITECTURAL IDENTIFIÉ (13 avril 2026) — ✅ RÉSOLU

Le pipeline ML entraînait XGBoost/LSTM uniquement sur des **bougies brutes OHLCV** (prédiction directionnelle générique). Il n'analysait **jamais** les résultats réels des trades clôturés. La boucle d'apprentissage stratégie-spécifique était donc absente.

**Architecture cible (deux niveaux, par stratégie) :**
```
Niveau 1 — Pré-entraînement général (conservé)
  Bougies OHLCV historiques → labelliser() → XGBoost/LSTM
  → Donne la connaissance directionnelle du marché

Niveau 2 — Fine-tuning stratégie (à implémenter, indépendant par stratégie)
  Signal émis → snapshot features → trade clôturé → label réel (TP/SL)
  → {strategie}_features_snapshot JOIN {strategie}_signaux
  → entrainer_sur_trades_clostures() → XGBoost fine-tuné par stratégie
```

---

### Ordre d'implémentation logique

```
Bloc A — ML robuste (sans dépendances externes)
  P1  Walk-forward comme critère de rollback      ← aucune dépendance
  P2  Snapshot features à l'émission (Rockets)    ← fondation ML stratégie
  P3  Ré-entraînement sur trades clôturés          ← dépend P2
  P4  Feature importance                           ← dépend P3
  P5  Seuil de confiance ML dynamique              ← dépend P3

Bloc B — Boucle LLM / feedback trader
  P6  Saisie résultats trader (feedback loop)     ← maillon critique LLM
  P7  Capital simulé + position sizing             ← indépendant, peut être en //
  P8  Tableau de bord performances LLM             ← dépend P6
  P9  Enrichissement automatique feedback          ← dépend P6
  P10 Détection patterns d'échec                  ← dépend P6 + P8
  P11 UI tableau suivi signaux                     ← dépend P6

Bloc C — Calibration + extension stratégies
  P12 Calibration automatique paramètres           ← dépend P3 + P6
  P13 Extension Straddle et SMC                   ← dépend P12 validé sur Rockets
```

---

## 📊 BILAN D'AVANCEMENT — 15 avril 2026

| Priorité | Intitulé | Statut | Ce qui manque |
|---|---|---|---|
| **P1** | Walk-forward rollback ML | ✅ FAIT | — |
| **P2** | Snapshot features Rockets | ✅ FAIT | — |
| **P3** | Ré-entraînement sur trades clôturés | ✅ FAIT | — |
| **P4** | Feature importance | ✅ FAIT | — |
| **P5** | Seuil de confiance ML dynamique | ✅ FAIT | — |
| **P6** | Feedback loop trader (UI) | ✅ FAIT | — boucle automatique : worker 3min → `reconcilier_feedback` → few-shot LLM. Endpoint saisie manuelle optionnel. |
| **P7** | Capital simulé + position sizing | ✅ FAIT | — `lotPourSignal()` dans `useSignauxTableau.ts`, affiché en jaune dans `SignauxTableau.vue` pour Rockets et SMC |
| **P8** | Dashboard performances LLM | ✅ FAIT | — |
| **P9** | Enrichissement automatique feedback | ✅ FAIT | — |
| **P10** | Détection patterns d'échec | ✅ FAIT | — |
| **P11** | UI tableau suivi signaux pris | ⚠️ PARTIEL | Colonnes PnL R et comparaison Conviction LLM vs résultat absentes |
| **P12** | Calibration automatique paramètres scan | ⚠️ PARTIEL | Grid search backend OK ; panneau validation humaine dans `SettingsView` absent |
| **P13** | Extension Straddle + SMC | ❌ À FAIRE | Aucun snapshot features, aucun fine-tuning, aucun modèle séparé |

**Récap** : 5 items ✅ terminés · 7 items ⚠️ partiels · 1 item ❌ à faire

**Prochains chantiers prioritaires :**
1. **P6** — Boutons « Pris / Ignoré » dans `SignauxTableau.vue` ← impact immédiat sur la boucle LLM
2. **P7** — Injecter position sizing dans signal + UI Money Management
3. **P11** — Colonnes PnL R + conviction vs résultat dans `SignauxTableau`
4. **P12** — Panneau UI validation calibration dans `SettingsView`

---

## BLOC A — ML robuste

---

### P1 — Walk-forward comme critère de rollback ML

> **État : ✅ IMPLÉMENTÉ** — Le rollback utilise `accuracy_val_recente()` qui lit la colonne `accuracy_val` en DB, alimentée avec `wf.accuracy_finale` (score OOS 25% test set). Détection overfitting active si `gap_train_val > 15%`. Badge « Overfit / OK » + valeur du gap affichés dans `MlRetrainPanel.vue`. L'audit du 15 avril avait tort : ce point est complet.

> **Aucune dépendance — implémentable immédiatement.**

**Problème actuel :** le rollback dans `ml_retrain_handler.rs` compare l'accuracy sur le jeu d'**entraînement** — validation optimiste (le modèle a vu ces données). `walk_forward.rs` est implémenté et appelé mais sa métrique est ignorée — code orphelin.

**Correction :**
```rust
// AVANT (trop optimiste)
if accuracy_apres < accuracy_avant - 0.02 { rollback() }

// APRÈS (robuste)
if wf_score_apres < wf_score_avant - 0.02 { rollback() }
// + si gap train/wf > 15% → overfitting détecté → rollback aussi
```

**Métriques à logger à chaque entraînement :**
- Accuracy train (existant)
- Score walk-forward out-of-sample (existant mais ignoré)
- Gap train/wf (nouveau) → alerte si > 15%

**Fichiers :**
- `backend/crates/api/src/ml_retrain_handler.rs` : utiliser wf_score pour le rollback
- `backend/crates/api/src/scheduler.rs` : retourner wf_score depuis `entrainer_walk_forward()`
- `backend/crates/db/src/ml_insights.rs` : logger gap train/wf
- `frontend/src/views/MlInsightsView.vue` : afficher le gap comme indicateur de santé

---

### P2 — Snapshot features ML à l'émission du signal (Rockets)

> **État : ✅ IMPLÉMENTÉ** — Table `rockets_features_snapshot` (migration 0047), fonctions `inserer_snapshot` + `lire_snapshots_avec_labels` dans `rockets_features.rs`, appelé depuis `rockets_handlers.rs`.

**C'est le chaînon manquant.** Sans ce snapshot, on ne peut jamais reconstruire fidèlement les features qui existaient au moment du signal.

**Table à créer :**
```sql
CREATE TABLE rockets_features_snapshot (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL UNIQUE REFERENCES rockets_signaux(id),
    ticker        TEXT NOT NULL,
    cree_le       TEXT NOT NULL DEFAULT (datetime('now')),
    features_json TEXT NOT NULL   -- Vec<f64> sérialisé en JSON compact
);
```

**Déclencheur :** dans `rockets_sauvegarder.rs`, juste après `INSERT INTO rockets_signaux`, appeler `extraire_features()` sur les bougies courantes et insérer dans `rockets_features_snapshot`.

**Fichiers :**
- `backend/crates/db/migrations/` : nouvelle migration
- `backend/crates/db/src/rockets_features.rs` : `inserer_snapshot(pool, signal_id, features)`
- `backend/crates/api/src/rockets_sauvegarder.rs` : appel après sauvegarde signal

---

### P3 — Ré-entraînement sur trades clôturés (Rockets)

> **État : ✅ IMPLÉMENTÉ** — `entrainer_sur_trades_clotures` dans `rockets_trainer.rs`, job `executer_fine_tuning_rockets` dans `ml_retrain_job.rs`, guard min 50 samples, appelé par le scheduler quotidien.

**Dépend de P2.** Les 57 trades clôturés actuels suffisent pour amorcer (minimum 50 requis).

**Logique :**
```sql
SELECT s.features_json,
       CASE WHEN rs.verdict IN ('TP1','TP2','TP3') THEN 1.0 ELSE 0.0 END AS label
FROM rockets_features_snapshot s
JOIN rockets_signaux rs ON rs.id = s.signal_id
WHERE rs.statut = 'ferme' AND rs.verdict IS NOT NULL
```

**Nouvelle fonction dans `crates/ml/src/lib.rs` :**
```rust
pub fn entrainer_sur_trades_clostures(
    features: &[Vec<f64>],
    labels: &[f64],    // 1.0=TP, 0.0=SL/invalide
) -> Result<f64>       // accuracy walk-forward
```

**Score de fusion mis à jour :**
```
Score_Final = 0.4 × LSTM_bougies + 0.3 × XGB_bougies + 0.3 × XGB_trades
```
Le poids XGB_trades monte progressivement avec le nombre d'échantillons disponibles.

**Fichiers :**
- `backend/crates/ml/src/lib.rs` : `entrainer_sur_trades_clostures()`
- `backend/crates/api/src/scheduler.rs` : appel après `executer_entrainements_tous()`
- `backend/crates/api/src/ml_retrain_handler.rs` : inclure dans le job de ré-entraînement

---

### P4 — Feature importance : surfacer ce que le modèle a appris

> **État : ✅ IMPLÉMENTÉ** — Graphique barres horizontales « Top features prédictives (Rockets) » dans `MlRetrainPanel.vue`, alimenté par `chargerTopFeatures()` via `GET /api/ml/feature-importance/rockets`. Endpoint dans `ml_retrain_handler.rs`, persistance dans `ml_feature_importance.rs`. L'audit du 15 avril était erroné.

**Dépend de P3.** XGBoost calcule nativement les importances après entraînement — elles sont actuellement ignorées.

**Utilités :**
1. Savoir quelles features prédisent réellement les TP → orienter la calibration (P12)
2. Identifier les features inutiles → les supprimer → modèle plus précis
3. Afficher dans ML Insights "top 5 features les plus prédictives" → compréhension trader

```sql
CREATE TABLE ml_feature_importance (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    calcule_le  TEXT NOT NULL DEFAULT (datetime('now')),
    strategie   TEXT NOT NULL,
    feature_idx INTEGER NOT NULL,
    feature_nom TEXT NOT NULL,
    importance  REAL NOT NULL
);
```

**Fichiers :**
- `backend/crates/ml/src/xgboost.rs` : extraire et retourner les importances après `entrainer()`
- `backend/crates/ml/src/features.rs` : ajouter `const FEATURE_NOMS: [&str; NB_FEATURES]`
- `backend/crates/db/src/ml_insights.rs` : sauvegarder les importances
- `frontend/src/views/MlInsightsView.vue` : graphique barres horizontales

---

### P5 — Seuil de confiance ML dynamique par stratégie

> **État : ✅ IMPLÉMENTÉ** — Table `configuration` avec clés `seuil_confiance_rockets/straddle/smc` (migration 0049), seuils calibrés par phase/session dans `rockets_calibration.rs`.

**Dépend de P3.** Actuellement un score de 0.51 et 0.95 produisent tous les deux un signal publié.

**Principe :** seuil configurable par stratégie en DB. Tout signal sous le seuil est supprimé avant publication.

```
Confiance ML = 0.73  | Seuil = 0.65  → Signal publié ✅
Confiance ML = 0.58  | Seuil = 0.65  → Signal supprimé ❌
```

**Configuration (table `configuration` existante) :**
```
seuil_confiance_rockets  = 0.60
seuil_confiance_straddle = 0.60
seuil_confiance_smc      = 0.65
```

**Fichiers :**
- `backend/crates/api/src/rockets_sauvegarder.rs` : vérification seuil avant `INSERT INTO rockets_signaux`
- `backend/crates/api/src/smc_signal_handler.rs` : idem
- `backend/crates/api/src/straddle_signal_handler.rs` : idem
- `frontend/src/views/SettingsView.vue` : sliders de seuil par stratégie

---

## BLOC B — Boucle LLM / feedback trader

---

### P6 — Saisie du résultat par le trader (feedback loop)

> **État : ✅ IMPLÉMENTÉ** — La boucle est entièrement automatique : `demarrer_worker_suivi()` s'exécute toutes les 3 minutes, fetch le prix Binance, calcule le verdict (`calculer_verdict_rocket`), met à jour `rockets_signaux`, puis appelle `reconcilier_feedback` qui alimente `rockets_feedback` (pnl_r, gagnant, session). `construire_few_shot()` injecte ces feedbacks dans le prochain prompt LLM. L'endpoint `POST /api/rockets/feedback/trader` est un ajout optionnel pour saisie manuelle d'une sortie sur un prix différent du détecté automatiquement.

**C'est le maillon le plus critique de la boucle LLM.** Sans résultats réels saisis, le LLM tourne à vide.

**Flux UX :**
```
Signal Rockets affiché → bouton "J'ai pris ce trade"
   → formulaire minimal : prix d'entrée réel | prix de sortie | résultat (SL / TP1 / TP2 / TP3)
   → backend calcule pnl_r = (sortie - entrée) / atr14
   → insère dans rockets_feedback avec verdict + pnl_r + gagnant
```

**Alternative** : bouton "Signal ignoré" → verdict="ignoré" (utile pour analyser les signaux non pris).

**Fichiers :**
- `SignauxTableau.vue` : boutons "Pris" / "Ignoré" sur chaque ligne
- `backend/crates/api/src/rockets_feedback_handler.rs` : `POST /api/rockets/feedback`
- `backend/crates/db/src/rockets_feedback.rs` : `inserer_feedback` existe déjà

---

### P7 — Capital simulé + position sizing indicatif

> **État : ⚠️ PARTIEL** — `calculer_taille_position` implémenté dans `risk/src/lib.rs` (risque 1% capital). Non injecté dans le signal publié vers le frontend. Bloc Money Management absent dans `SettingsView.vue`.

**Indépendant — peut être implémenté en parallèle de P6.**

**Configuration (SettingsView) :**
```
Capital de référence    : [10 000 €]
Risk par trade Rockets  : [1.5 %]
Risk par trade Straddle : [1.0 %]
Risk par trade SMC      : [1.5 %]
```

**Calcul backend (crate `risk`) :**
```rust
pub fn position_sizing_indicatif(capital: f64, risk_pct: f64, distance_sl_pct: f64) -> PositionSize {
    let montant_risque = capital * risk_pct;
    let nb_unites = montant_risque / distance_sl_pct;
    PositionSize { montant_risque, nb_unites }
}
```

**Fichiers :**
- `backend/crates/risk/src/lib.rs` : `position_sizing_indicatif`
- `backend/crates/api/src/rockets_sauvegarder.rs` : injecter dans le signal publié
- `frontend/src/views/SettingsView.vue` : bloc "Money Management"

---

### P8 — Tableau de bord performances LLM

> **État : ✅ IMPLÉMENTÉ** — Endpoint `GET /api/rockets/monitoring-ml` + handler, stats globales + par phase, courbe equity (`GET /api/rockets/equity`), `rockets_feedback_stats.rs`, section Rockets dans `MlInsightsView.vue`.

**Dépend de P6.**

**Métriques :**
- Win rate global | par phase | par session | par ticker
- PnL moyen en R | meilleur trade | pire trade
- Conviction LLM vs résultat réel
- Évolution 7j / 30j / tout

**Fichiers :**
- `backend/crates/api/src/rockets_stats_handler.rs` : `GET /api/rockets/stats`
- `backend/crates/db/src/rockets_feedback.rs` : `stats_par_phase`, `stats_par_ticker`, `stats_par_session`
- `frontend/src/views/HistoryView.vue` : section stats

---

### P9 — Enrichissement automatique du feedback

> **État : ✅ IMPLÉMENTÉ** — Migration 0050 : colonnes `prix_entree_reel`, `prix_sortie_reel`, `session_sortie`, `notes_trader` ajoutées sur les 3 tables feedback (rockets/straddle/smc). Mise à jour dans `maj_feedback_verdict` à chaque clôture.

**Dépend de P6.**

**Données à ajouter dans `rockets_feedback` :**
```sql
ALTER TABLE rockets_feedback ADD COLUMN prix_entree_reel REAL;
ALTER TABLE rockets_feedback ADD COLUMN prix_sortie_reel  REAL;
ALTER TABLE rockets_feedback ADD COLUMN duree_trade_min   INTEGER;
ALTER TABLE rockets_feedback ADD COLUMN session_sortie    TEXT;
ALTER TABLE rockets_feedback ADD COLUMN notes_trader      TEXT;
```

Impact LLM : `construire_few_shot` peut afficher durée, session de sortie, notes libres → contexte plus riche.

---

### P10 — Détection de patterns d'échec récurrents

> **État : ✅ IMPLÉMENTÉ** — Table `regles_rejet_apprises` (migration 0051), job `patterns_echec_job.rs`, fonctions `upsert_regle` et `lister_actives` dans `regles_rejet.rs`.

**Dépend de P6 + P8. Job toutes les 6h, min 10 trades par combinaison.**

```
Pour chaque (phase, session, atr_ratio_bucket, rsi_bucket) :
    si nb_trades >= 10 ET win_rate < 35% :
        → créer règle de rejet en DB
        → injecter dans le prompt LLM (section "Leçons systémiques")
```

```sql
CREATE TABLE regles_rejet_apprises (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    strategie   TEXT NOT NULL,
    phase       TEXT,
    session     TEXT,
    condition   TEXT NOT NULL,
    win_rate    REAL NOT NULL,
    nb_trades   INTEGER NOT NULL,
    active      INTEGER NOT NULL DEFAULT 1,
    apprise_le  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

---

### P11 — UI : tableau de suivi des signaux pris

**Dépend de P6.**

**Vue `en_cours` :** prix d'entrée réel | PnL simulé live | bouton "Clôturer"
**Vue `clôturées` :** PnL R coloré | Conviction LLM vs résultat | filtres phase/session/ticker
**Widget dashboard :** WR 7j | WR 30j | PnL total R | qualité LLM

---

## BLOC C — Calibration + extension stratégies

---

### P12 — Calibration automatique des paramètres de détection

> **État : ⚠️ PARTIEL** — `rockets_calibration.rs` implémente un grid search sur `(score_scan, conviction_llm)` toutes les 6h. Table `rockets_calibration` (migration 0029) stocke les seuils optimaux par phase+session. `straddle_calibration.rs` suit le même pattern. **Manque** : panneau UI « Calibration — propositions en attente » dans `SettingsView.vue` (validation humaine obligatoire avant application).

**Dépend de P3 (labels fiables) + P6 (résultats saisis). Job toutes les 24h, min 30 trades.**

**Paramètres candidats :** `score_min`, `atr_ratio_max`, `rsi_min/max`, `nb_bougies_compression_min`, `volume_seche_max`.

```
Pour chaque paramètre P :
    corrélation(P au moment du signal, verdict=TP)
    si corrélation > seuil ET nb_trades >= 30 :
        proposer nouveau seuil → enregistrer dans calibration_parametres
        validation humaine obligatoire avant application
```

```sql
CREATE TABLE calibration_parametres (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    strategie       TEXT NOT NULL,
    parametre       TEXT NOT NULL,
    valeur_actuelle REAL NOT NULL,
    valeur_proposee REAL NOT NULL,
    win_rate_avant  REAL NOT NULL,
    win_rate_predit REAL NOT NULL,
    nb_trades_base  INTEGER NOT NULL,
    calcule_le      TEXT NOT NULL DEFAULT (datetime('now')),
    applique        INTEGER NOT NULL DEFAULT 0,
    applique_le     TEXT
);
```

**Validation humaine obligatoire :** panneau `SettingsView` → "Calibration — propositions en attente". Affichage : "Score min : 65 → 72 (43 trades, WR prédit +8%)".

**Fichiers :**
- `backend/crates/api/src/rockets_calibration.rs` : enrichir avec cette logique (existe)
- `backend/crates/db/src/calibration.rs` : lecture/écriture propositions
- `frontend/src/views/SettingsView.vue` : panneau validation
- `frontend/src/components/common/RocketsReglages.vue` : paramètres actifs vs proposés

---

### P13 — Extension à Straddle et SMC (apprentissages indépendants)

> **État : ❌ À FAIRE** — Aucun snapshot features pour Straddle/SMC, aucun fine-tuning, aucun modèle séparé. Seul `modele_xgboost.json` généraliste existe. Les stats Straddle/SMC sont affichées dans ML Insights mais sans boucle d'apprentissage stratégie-spécifique.

**Dépend de P12 validé et stable sur Rockets.**

**Règle absolue :** chaque stratégie a ses propres tables, son propre modèle, son propre prompt LLM. Aucune donnée partagée entre stratégies.

| Composant | Rockets | Straddle | SMC |
|---|---|---|---|
| Snapshot features | `rockets_features_snapshot` | `straddle_features_snapshot` | `smc_features_snapshot` |
| Feedback | `rockets_feedback` | `straddle_feedback` | `smc_feedback` |
| Modèle XGB | `modele_xgboost_rockets.json` | `modele_xgboost_straddle.json` | `modele_xgboost_smc.json` |
| Features | `extraire_features()` | idem (OHLCV) | `extraire_features_smc()` (+ 5 indicateurs SMC) |

**Note SMC :** les features SMC incluent Order Blocks, Imbalance, IFVG, Fibonacci, Tendance — absents pour Rockets et Straddle. `extraire_features_smc()` à créer séparément dans `crates/smc/`.

**Fichiers :**
- `backend/crates/api/src/smc_signal_handler.rs` : ajout snapshot à l'émission
- `backend/crates/api/src/straddle_signal_handler.rs` : idem
- `backend/crates/db/src/straddle_features.rs` : table + fonctions
- `backend/crates/db/src/smc_features.rs` : table + fonctions
- `backend/crates/smc/src/features.rs` : `extraire_features_smc()`
- `backend/crates/api/src/smc_calibration_job.rs` (existe) : enrichir
- `backend/crates/api/src/straddle_calibration.rs` (existe) : enrichir

---

### Ce qui est hors scope (rappel)

| Élément | Pourquoi exclu |
|---|---|
| Exécution automatique des trades | L'app est signal-only |
| Connexion API Binance/MT5 pour le solde | Pas de trading réel |
| Trailing stop automatique | Pas d'exécution |
| Blocage automatique de nouveaux signaux | Le trader décide toujours |
| Fine-tuning du modèle LLM | Trop lourd, hors scope actuel |
| Apprentissage croisé entre stratégies | Contamination des données — interdit |


---

> ⚠️ **SECTION ARCHIVÉE** — Le contenu ci-dessous correspond à l'ancienne organisation de la roadmap (avant le 13 avril 2026). La référence canonique est le Bloc A/B/C ci-dessus avec les statuts à jour. Cette section est conservée pour traçabilité.

---

### ⚠️ PROBLÈME ARCHITECTURAL IDENTIFIÉ (13 avril 2026) — ✅ RÉSOLU

Le pipeline ML entraînait XGBoost/LSTM uniquement sur des **bougies brutes OHLCV** (prédiction directionnelle générique). Il n'analysait **jamais** les résultats réels des trades Rockets clôturés. La boucle d'apprentissage stratégie-spécifique était donc absente.

**Architecture cible (deux niveaux) :**
```
Niveau 1 — Pré-entraînement général (conservé)
  Bougies OHLCV historiques → labelliser() → XGBoost/LSTM
  → Donne la connaissance directionnelle du marché

Niveau 2 — Fine-tuning stratégie (à implémenter)
  Signal Rockets émis → snapshot features → trade clôturé → label réel (TP/SL)
  → rockets_features_snapshot JOIN rockets_signaux
  → entraîner_sur_trades_clostures() → XGBoost fine-tuné
  → Le modèle apprend POURQUOI un signal Rocket réussit ou échoue
```

**Pré-requis :** stocker un snapshot des features ML au moment de l'émission de chaque signal dans une table dédiée `rockets_features_snapshot` (pas dans `rockets_signaux` — trop de colonnes, schéma différent).

---

### Priorité 0 — Snapshot features ML à l'émission du signal ⭐ NOUVEAU

**C'est le chaînon manquant.** Sans ce snapshot, on ne peut jamais reconstruire fidèlement les features qui existaient au moment du signal.

**Table à créer :**
```sql
CREATE TABLE rockets_features_snapshot (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id   INTEGER NOT NULL UNIQUE REFERENCES rockets_signaux(id),
    ticker      TEXT NOT NULL,
    cree_le     TEXT NOT NULL DEFAULT (datetime('now')),
    -- Les NB_FEATURES valeurs calculées par extraire_features() au moment du signal
    features_json TEXT NOT NULL   -- Vec<f64> sérialisé en JSON compact
);
```

**Déclencheur :** à l'émission du signal dans `rockets_sauvegarder.rs`, juste après `INSERT INTO rockets_signaux`, appeler `extraire_features()` sur les bougies courantes et insérer dans `rockets_features_snapshot`.

**Fichiers :**
- `backend/crates/db/migrations/` : nouvelle migration
- `backend/crates/db/src/rockets_features.rs` : `inserer_snapshot(pool, signal_id, features)`
- `backend/crates/api/src/rockets_sauvegarder.rs` : appel après sauvegarde signal

---

### Priorité 0b — Ré-entraînement sur trades clôturés ⭐ NOUVEAU

**Dépend de P0.** Peut commencer dès que 50 snapshots existent (57 trades clôturés actuels suffisent si on rétro-génère les features depuis les bougies historiques pour amorcer).

**Logique :**
```
SELECT s.features_json, CASE WHEN rs.verdict IN ('TP1','TP2','TP3') THEN 1.0 ELSE 0.0 END
FROM rockets_features_snapshot s
JOIN rockets_signaux rs ON rs.id = s.signal_id
WHERE rs.statut = 'ferme' AND rs.verdict IS NOT NULL
```

**Nouvelle fonction dans `crates/ml/src/lib.rs` :**
```rust
pub fn entrainer_sur_trades_clostures(
    features: &[Vec<f64>],
    labels: &[f64],       // 1.0=TP, 0.0=SL/invalide
) -> Result<f64>          // accuracy
```

**Score de fusion mis à jour :**
```
Score_Final = 0.4 × LSTM_bougies + 0.3 × XGB_bougies + 0.3 × XGB_trades
```
(Le XGB trades est pondéré progressivement selon le nombre d'échantillons disponibles)

**Fichiers :**
- `backend/crates/ml/src/lib.rs` : `entrainer_sur_trades_clostures()`
- `backend/crates/api/src/scheduler.rs` : appel après `executer_entrainements_tous()`
- `backend/crates/api/src/ml_retrain_handler.rs` : inclure dans le job de ré-entraînement

---

### Comment le LLM s'améliore (principe)

```
Signal émis → Trader exécute (ou non) → Résultat saisi → Feedback en DB
                                                              ↓
                               Prochain signal similaire → LLM voit le feedback
                                                              ↓
                                                    Meilleure décision
```

Le LLM n'apprend pas par fine-tuning (trop lourd). Il apprend par **contexte enrichi** :
à chaque nouvelle analyse, il reçoit les résultats de signaux passés similaires.
Plus la base de feedbacks est dense et précise, meilleur est le filtre.

---

### Priorité 1 — Saisie du résultat par le trader (feedback loop)

**C'est le maillon le plus critique.** Sans résultats réels saisis, le LLM tourne à vide.

**Ce qui existe déjà** : table `rockets_feedback` avec `verdict`, `pnl_r`, `gagnant`.  
**Ce qui manque** : une UX fluide pour saisir le résultat après avoir pris (ou pas) un signal.

**Flux UX** :
```
Signal Rockets affiché → bouton "J'ai pris ce trade"
   → formulaire minimal : prix d'entrée réel | prix de sortie | résultat (SL / TP1 / TP2 / TP3)
   → backend calcule pnl_r = (sortie - entrée) / atr14
   → insère dans rockets_feedback avec verdict + pnl_r + gagnant
```

**Alternative** : bouton "Signal ignoré" → sauvegarde verdict="ignoré" (utile pour savoir quels signaux le trader ne prend pas et pourquoi).

**Fichiers** :
- `SignauxTableau.vue` ou `RocketsTableau.vue` : boutons "Pris" / "Ignoré" sur chaque ligne
- `backend/crates/api/src/rockets_feedback_handler.rs` : endpoint `POST /api/rockets/feedback`
- `backend/crates/db/src/rockets_feedback.rs` : `inserer_feedback` existe déjà

---

### Priorité 1b — Bloc performance + courbe equity (Dashboard)

**Dépend de la priorité 1** : la courbe n'a de sens que si des feedbacks sont saisis.  
À implémenter immédiatement après la feedback loop.

**Emplacement** : `DashboardHome.vue`, sous la barre Rockets existante.

**Maquette** :
```
┌─ Performance Rockets ────────────────────────────────────────┐
│  Capital simulé : 10 850€  (+8.5%)  │  WR 30j : 61%         │
│  [courbe equity — ligne verte sur fond sombre, ~30 points]   │
│  ▲ Meilleur : +3.2R   ▼ Pire : -1.0R   Σ 23 trades clôturés │
│  ⚠️ Taux de saisie : 74% — résultats partiels                 │
└──────────────────────────────────────────────────────────────┘
```

**Données** :
- Courbe = equity simulée cumulée point par point, un point par feedback saisi (pas par bougie)
- `equity[n] = capital_initial + Σ pnl_r[0..n] × montant_risque`
- Couleur courbe : verte si equity[last] > equity[last-7], rouge sinon
- Badge ⚠️ si `nb_feedbacks_saisis / nb_signaux_emis < 0.80` (biais de confirmation)

**Technologie** : SVG inline ou `<canvas>` simple — PAS TradingView (série de 20–50 points max, pas de bougie).

**Endpoint backend** : `GET /api/rockets/equity` → retourne `Vec<{ date, equity_cumulee, pnl_r, ticker }>`

**Fichiers** :
- `backend/crates/db/src/rockets_feedback.rs` : fonction `courbe_equity(pool, capital_initial) -> Vec<EquityPoint>`
- `backend/crates/api/src/rockets_stats_handler.rs` : endpoint `GET /api/rockets/equity`
- `frontend/src/composables/useRocketsPerf.ts` : fetch + calcul de la série SVG
- `frontend/src/components/common/RocketsPerfBloc.vue` : composant dédié (<250 lignes)
- `frontend/src/views/DashboardHome.vue` : intégration du composant

---

### Priorité 2 — Capital simulé + position sizing indicatif

**Objectif** : afficher dans chaque signal "pour un compte de X€, risquer Y€ = Z unités".  
Ce n'est pas un blocage — c'est une **information affichée** pour aider le trader à calibrer sa taille réelle.

**Configuration (SettingsView)** :
```
Capital de référence   : [10 000 €]     ← saisi une fois, modifiable
Risk par trade Rockets : [1.5 %]        ← paramétrable
Risk par trade Straddle: [1.0 %]        ← par direction
```

**Calcul backend (crate `risk`)** :
```rust
pub fn position_sizing_indicatif(capital: f64, risk_pct: f64, distance_sl_pct: f64) -> PositionSize {
    let montant_risque = capital * risk_pct;          // ex: 150€
    let nb_unites = montant_risque / distance_sl_pct; // ex: 0.003 BTC
    PositionSize { montant_risque, nb_unites }
}
```

**Affiché dans le signal** : "Risque : 150€ | Taille indicative : 0.003 BTC"  
Le trader fait ce qu'il veut avec cette info.

**Fichiers** :
- `backend/crates/db/src/config.rs` (nouveau) : table `config` clé/valeur pour capital + risk_pct
- `backend/crates/risk/src/lib.rs` : fonction `position_sizing_indicatif`
- `backend/crates/api/src/rockets_sauvegarder.rs` : injecter dans le signal publié
- `frontend/src/views/SettingsView.vue` : nouveau bloc "Money Management"

---

### Priorité 3 — Tableau de bord de performance LLM

**Objectif** : permettre au trader (et au LLM via le contexte) de voir les performances réelles des signaux émis.

**Métriques clés à afficher** :
- Win rate global | par phase (prelancement / compression / breakout)
- PnL moyen en R | meilleur trade | pire trade
- Taux de signaux ignorés (le trader ne prend pas le signal → pourquoi ?)
- Conviction LLM vs résultat réel (est-ce que conviction=90 bat vraiment conviction=60 ?)
- Performance par session (london / ny / asia / off)
- Performance par ticker (BTC vs ETH vs XAUUSD)
- Évolution sur 7j / 30j / tout

**Ces métriques servent deux usages** :
1. Le trader voit objectivement ce qui fonctionne
2. Le `taux_reussite_recent` (déjà implémenté) injecte les stats 48h dans le prompt LLM

**Fichiers** :
- `backend/crates/api/src/rockets_stats_handler.rs` (nouveau) : `GET /api/rockets/stats`
- `backend/crates/db/src/rockets_feedback.rs` : nuevas fonctions `stats_par_phase`, `stats_par_ticker`, `stats_par_session`
- `frontend/src/views/HistoryView.vue` : section stats sous le tableau

---

### Priorité 4 — Enrichissement automatique du feedback

**Objectif** : enrichir chaque feedback avec les données de marché au moment de la sortie, pour que le LLM ait plus de contexte lors des comparaisons futures.

**Données à ajouter dans `rockets_feedback`** :
```sql
ALTER TABLE rockets_feedback ADD COLUMN prix_entree_reel REAL;   -- prix réel d'entrée trader
ALTER TABLE rockets_feedback ADD COLUMN prix_sortie_reel  REAL;  -- prix réel de sortie
ALTER TABLE rockets_feedback ADD COLUMN duree_trade_min   INTEGER; -- durée en minutes
ALTER TABLE rockets_feedback ADD COLUMN session_sortie    TEXT;   -- session au moment de la sortie
ALTER TABLE rockets_feedback ADD COLUMN notes_trader      TEXT;   -- commentaire libre
```

**Impact LLM** : `construire_few_shot` peut afficher "duree=47min | session_sortie=ny | notes=mèche rejet avant TP2" → le LLM comprend le contexte de la sortie, pas seulement le résultat.

---

### Priorité 5 — Détection de patterns d'échec récurrents

**Objectif** : identifier automatiquement les configurations qui échouent systématiquement et durcir les règles de rejet en conséquence.

**Logique backend** (job périodique, ex: toutes les 6h) :
```
Pour chaque combinaison (phase, session, atr_ratio_bucket, rsi_bucket) :
    si nb_trades >= 10 ET win_rate < 35% :
        → créer/mettre à jour une règle de rejet automatique en DB
        → le prompt LLM inclut ces règles dans la section "Leçons systémiques"
```

**Table** :
```sql
CREATE TABLE regles_rejet_apprises (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    phase       TEXT,
    session     TEXT,
    condition   TEXT NOT NULL,   -- ex: "atr_ratio > 2.0 AND rsi > 75"
    win_rate    REAL NOT NULL,
    nb_trades   INTEGER NOT NULL,
    active      INTEGER NOT NULL DEFAULT 1,
    apprise_le  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Ces règles sont injectées dans le prompt** à côté des règles de rejet manuelles — mais labellisées "apprises par expérience" pour que le LLM les distingue.

---

### Priorité 6 — UI : tableau de suivi des signaux pris

**SignauxTableau.vue** — enrichissements pour les signaux en cours et clôturés :

**Vue `en_cours`** (signaux pris, position ouverte) :
- Prix d'entrée réel saisi | PnL simulé en R (calculé avec le prix live WS)
- Bouton "Clôturer" → formulaire de sortie → génère le feedback

**Vue `cloturées`** :
- Colonne PnL R colorée (vert/rouge)
- Colonne Conviction LLM vs résultat (badge ✅ si conviction élevée + gagnant)
- Filtre par phase / session / ticker

**Widget performance global** (DashboardHome.vue) :
- WR 7j | WR 30j | PnL total simulé en R
- Barre de progression "qualité LLM" = conviction moyenne des gagnants vs perdants

---

### Ce qui est hors scope (rappel)

| Élément | Pourquoi exclu |
|---|---|
| Exécution automatique des trades | L'app est signal-only |
| Connexion API Binance/MT5 pour le solde | Pas de trading réel |
| Trailing stop automatique | Pas d'exécution |
| Blocage automatique de nouveaux signaux | Le trader décide toujours |
| Fine-tuning du modèle LLM | Trop lourd, hors scope actuel |

---

### Priorité 7 — Calibration automatique des paramètres de détection ⭐ NOUVEAU

> **C'est la troisième boucle d'apprentissage — la seule qui améliore le générateur de signaux, pas seulement le filtre.**

Les boucles LLM (P1-P6) et ML (P0-P0b) améliorent le **filtre** appliqué aux signaux existants.
Mais si les critères de détection dans `rockets_indicateurs.rs` sont mal calibrés (seuil compression trop large, score minimum trop bas, mauvais ATR ratio), les signaux générés sont structurellement mauvais — et aucun filtre ne peut corriger ça.

**Principe :**
```
Analyser rockets_signaux (trades clôturés)
  → Quels paramètres au moment du signal corrèlent avec TP ?
  → Quels paramètres corrèlent systématiquement avec SL/invalide ?
  → Ajuster automatiquement les seuils de génération en DB
  → Chaque nouveau signal est généré avec des critères plus précis
```

**Paramètres candidats à calibrer automatiquement :**
- `score_min` : score minimum pour qu'un signal soit émis (actuellement fixe)
- `atr_ratio_max` : ratio ATR au-delà duquel le marché est trop volatil pour Rockets
- `rsi_min` / `rsi_max` : plages RSI favorables par phase
- `nb_bougies_compression_min` : durée minimale de compression valide
- `volume_seche_max` : seuil d'assèchement volume (actuellement 0.75)

**Logique backend (job toutes les 24h, min 30 trades clôturés) :**
```
Pour chaque paramètre P :
    calculer corrélation(P au moment du signal, verdict=TP)
    si corrélation > seuil_confiance ET nb_trades >= 30 :
        proposer nouveau seuil = percentile_optimal(P, trades_gagnants)
        enregistrer dans config DB avec flag "calibration_auto"
        activer si win_rate prédit > win_rate actuel
```

**Table :**
```sql
CREATE TABLE calibration_parametres (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    parametre       TEXT NOT NULL,       -- ex: "score_min", "atr_ratio_max"
    valeur_actuelle REAL NOT NULL,
    valeur_proposee REAL NOT NULL,
    win_rate_avant  REAL NOT NULL,
    win_rate_predit REAL NOT NULL,
    nb_trades_base  INTEGER NOT NULL,
    calcule_le      TEXT NOT NULL DEFAULT (datetime('now')),
    applique        INTEGER NOT NULL DEFAULT 0,
    applique_le     TEXT
);
```

**Validation humaine obligatoire :** les nouvelles valeurs sont proposées mais **pas appliquées automatiquement** — le trader valide dans un panneau dédié (`SettingsView` → "Calibration automatique"). Affichage : "Score min : 65 → 72 (basé sur 43 trades, WR prédit +8%)".

**Fichiers :**
- `backend/crates/db/migrations/` : table `calibration_parametres`
- `backend/crates/api/src/rockets_calibration.rs` : job de calcul (existe déjà, à enrichir)
- `backend/crates/db/src/calibration.rs` : lecture/écriture des propositions
- `frontend/src/views/SettingsView.vue` : panneau "Calibration — propositions en attente"
- `frontend/src/components/common/RocketsReglages.vue` : afficher les paramètres actifs vs proposés

**Dépend de :** P0 (snapshot features) + P1 (résultats saisis) + P0b (labels fiables)

---

### Priorité 8 — Extension des boucles d'apprentissage à Straddle et SMC ⭐ NOUVEAU

> **Les 3 boucles (LLM, ML, calibration) ne concernent actuellement que Rockets. Straddle et SMC ont le même problème architectural.**

Les stratégies Straddle et SMC Directionnel émettent des signaux, ont des trades clôturés avec verdicts, mais n'ont aucune boucle d'apprentissage. Leurs paramètres sont figés. Leurs résultats ne réalimentent jamais le modèle.

**Ce qui doit être mutualisé pour les 3 stratégies :**

| Boucle | Rockets | Straddle | SMC |
|---|---|---|---|
| Snapshot features à l'émission | P0 (planifié) | ❌ absent | ❌ absent |
| Ré-entraînement sur trades clôturés | P0b (planifié) | ❌ absent | ❌ absent |
| Feedback loop trader | P1 (planifié) | ❌ absent | ❌ absent |
| Calibration paramètres | P7 (planifié) | ❌ absent | ❌ absent |

**Approche :** une fois les mécanismes validés sur Rockets, les dupliquer (ou généraliser) pour Straddle et SMC. Les tables peuvent être mutualisées avec une colonne `strategie TEXT` pour éviter la prolifération de tables.

**Tables généralisées (remplacement possible) :**
```sql
-- Au lieu de rockets_features_snapshot → une table unifiée
CREATE TABLE signaux_features_snapshot (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id   INTEGER NOT NULL,
    strategie   TEXT NOT NULL,   -- 'rockets' | 'straddle' | 'smc'
    ticker      TEXT NOT NULL,
    cree_le     TEXT NOT NULL DEFAULT (datetime('now')),
    features_json TEXT NOT NULL
);

-- Calibration unifiée
-- calibration_parametres : ajouter colonne strategie TEXT NOT NULL
```

**Spécificités SMC :** les features SMC incluent les 5 indicateurs SMC (Order Blocks, Imbalance, IFVG, Fibonacci, Tendance) qui n'existent pas pour Rockets. `extraire_features_smc()` devra être créée séparément.

**Fichiers concernés :**
- `backend/crates/api/src/smc_signal_handler.rs` / `straddle_signal_handler.rs` : ajout snapshot à l'émission
- `backend/crates/db/src/signaux_features.rs` : version généralisée de `rockets_features.rs`
- `backend/crates/api/src/smc_calibration_job.rs` (existe) : enrichir avec le même pattern que P7
- `backend/crates/api/src/straddle_calibration.rs` (existe) : idem

**Dépend de :** P7 (Rockets) validé et stable

---

### Priorité 9 — Feature importance : surfacer ce que le modèle a appris ⭐ NOUVEAU

**Problème actuel :** XGBoost calcule nativement quelles features sont les plus prédictives (gain, fréquence d'utilisation dans les splits). Cette information est calculée à chaque entraînement mais complètement ignorée et perdue.

**Utilités directes :**
1. Savoir si c'est le RSI, l'ATR ratio ou le volume qui prédit réellement les TP → orienter la calibration P7
2. Identifier les features inutiles qui bruitent le modèle → les supprimer → inférence plus rapide et plus précise
3. Afficher dans ML Insights "top 5 features les plus prédictives" → compréhension trader

**Implémentation :**

`smartcore::xgboost::XGRegressor` expose les importances via `.feature_importances()` après entraînement. Les sauvegarder dans la DB à chaque cycle :

```sql
CREATE TABLE ml_feature_importance (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    calcule_le  TEXT NOT NULL DEFAULT (datetime('now')),
    strategie   TEXT NOT NULL,
    feature_idx INTEGER NOT NULL,
    feature_nom TEXT NOT NULL,   -- nom lisible depuis features.rs
    importance  REAL NOT NULL
);
```

**Affichage :** graphique barres horizontales dans ML Insights → "Ce que le modèle juge important".

**Fichiers :**
- `backend/crates/ml/src/xgboost.rs` : extraire et retourner les importances après `entrainer()`
- `backend/crates/ml/src/features.rs` : ajouter `const FEATURE_NOMS: [&str; NB_FEATURES]`
- `backend/crates/db/src/ml_insights.rs` : sauvegarder les importances
- `frontend/src/views/MlInsightsView.vue` : nouveau graphique

**Dépend de :** P0b (entraînement sur trades) pour que les importances reflètent la stratégie réelle

---

### Priorité 10 — Seuil de confiance ML dynamique par stratégie ⭐ NOUVEAU

**Problème actuel :** un score de confiance XGBoost de 0.51 et 0.95 produisent tous les deux un signal publié. Il n'y a aucun filtre de confiance minimum. Les signaux marginaux (0.50–0.65) dégradent le win rate global sans apporter de valeur.

**Principe :** chaque stratégie a un seuil configurable en DB. Tout signal dont la confiance ML est inférieure au seuil est **supprimé avant publication** (pas de signal envoyé, pas d'entrée dans `rockets_signaux`).

```
Confiance ML = 0.73  | Seuil = 0.65  → Signal publié ✅
Confiance ML = 0.58  | Seuil = 0.65  → Signal supprimé silencieusement ❌
```

**Configuration par stratégie (table `configuration` existante) :**
```
seuil_confiance_rockets  = 0.60   (par défaut)
seuil_confiance_straddle = 0.60
seuil_confiance_smc      = 0.65   (SMC plus exigeant car moins de trades)
```

**Lien avec P7/P9 :** le seuil optimal peut être proposé automatiquement par la calibration (P7) en analysant à quel niveau de confiance le win rate devient acceptable. La feature importance (P9) peut orienter la re-pondération des features avant le calcul du score.

**Fichiers :**
- `backend/crates/api/src/rockets_sauvegarder.rs` : vérification seuil avant `INSERT INTO rockets_signaux`
- `backend/crates/api/src/smc_signal_handler.rs` : idem
- `backend/crates/api/src/straddle_signal_handler.rs` : idem
- `frontend/src/views/SettingsView.vue` : sliders de seuil par stratégie

**Dépend de :** P0b (confiance XGB sur trades réels fiable)

---

### Priorité 11 — Walk-forward comme critère de rollback ML ⭐ NOUVEAU

**Problème actuel :** le mécanisme de rollback dans `ml_retrain_handler.rs` compare l'accuracy sur le **jeu d'entraînement** avant et après. C'est une validation optimiste (le modèle a vu ces données → il les mémorise partiellement). Un modèle qui overfitte aura une bonne accuracy en entraînement mais sera mauvais en production.

`walk_forward.rs` est déjà implémenté et appelé dans le scheduler — mais sa métrique n'est **jamais utilisée comme critère de rollback**. C'est un calcul orphelin.

**Correction :** utiliser le score walk-forward (validation out-of-sample) comme critère principal de rollback :

```rust
// AVANT (trop optimiste)
if accuracy_apres < accuracy_avant - 0.02 { rollback() }

// APRÈS (robuste)
if wf_score_apres < wf_score_avant - 0.02 { rollback() }
// + bonus : si accuracy_train >> wf_score → signal d'overfitting → rollback aussi
```

**Métriques à logger à chaque entraînement :**
- Accuracy train (existant)
- Score walk-forward out-of-sample (existant mais ignoré)
- Gap train/wf (nouveau) → alerte si > 15% (overfitting détecté)

**Fichiers :**
- `backend/crates/api/src/ml_retrain_handler.rs` : utiliser wf_score pour le rollback
- `backend/crates/api/src/scheduler.rs` : retourner wf_score depuis `entrainer_walk_forward()`
- `backend/crates/db/src/ml_insights.rs` : logger gap train/wf
- `frontend/src/views/MlInsightsView.vue` : afficher le gap comme indicateur de santé du modèle

**Dépend de :** aucune dépendance — peut être implémenté indépendamment

---

### Ce qui est hors scope (rappel)

| Élément | Pourquoi exclu |
|---|---|
| Exécution automatique des trades | L'app est signal-only |
| Connexion API Binance/MT5 pour le solde | Pas de trading réel |
| Trailing stop automatique | Pas d'exécution |
| Blocage automatique de nouveaux signaux | Le trader décide toujours |
| Fine-tuning du modèle LLM | Trop lourd, hors scope actuel |



