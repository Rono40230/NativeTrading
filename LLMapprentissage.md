# Plan d'optimisation IA — Native Trading AI

## Matériel disponible
- CPU : Intel i9, 20 cœurs
- RAM : 48 GB
- GPU : NVIDIA RTX 3090, 24 GB VRAM

---

## Situation actuelle (baseline réelle)

### Composants IA de l'application

| Composant | Fichier(s) clés | Matériel utilisé | État |
|-----------|----------------|------------------|------|
| Feature extraction (52 features) | `ml/features.rs` | CPU séquentiel | ✅ pré-calcul O(N) `features_precalc.rs` |
| Walk-forward XGBoost + LSTM | `ml/walk_forward.rs` | CPU séquentiel | ✅ pré-calcul branché |
| Scheduler 58 combinaisons | `api/scheduler.rs` | CPU séquentiel (une par une) | ⏳ 17h UTC ✅, parallélisme Étape 5 |
| LSTM entraînement | `ml/lstm/` | CPU (feature `cuda` inactive) | ✅ déjà CUDA actif en prod |
| LSTM inférence | `ml/lstm/` | CUDA:0 | ✅ actif |
| XGBoost entraînement + inférence | `ml/xgboost.rs` | CPU 1 cœur (`smartcore`) | ❌ non optimisé (Étape 7) |
| Fine-tuning Rockets/SMC/Straddle | `ml/*_trainer.rs` | CPU 1 cœur | ❌ non optimisé |
| Ollama filtre Rockets | `ollama/rockets_filtre.rs` | `num_gpu:99` + semaphore global | ✅ GPU forcé |
| Ollama filtre SMC (+ retry) | `ollama/smc_filtre.rs` | `num_gpu:99` + retry | ✅ GPU forcé |
| Ollama analyse Straddle | `ollama/straddle_analyse.rs` | `num_gpu:99` + semaphore | ✅ GPU forcé |
| Ollama vision (chart) | `ollama/vision.rs` | `num_gpu:99`, semaphore global | ✅ GPU forcé |
| Ollama coach / diagram | `ollama/mod.rs` | `num_gpu:99`, semaphore global | ✅ GPU forcé |
| Calibration adaptative Rockets/SMC | `*_calibration.rs` | CPU (grid search léger) | ✅ acceptable |
| Anthropic Claude (fallback vision) | `anthropic.rs` | Cloud | ✅ external |

### Conséquences concrètes
- 58 combinaisons × ~74s = **~71 minutes** pour un cycle de réentraînement complet
- CPU utilisé à ~5%, GPU à ~1%, RAM à ~1% — la machine est quasi idle
- Ollama s'exécute probablement en CPU → filtres LLM lents (~8-15s au lieu de 1-2s)
- Heure du réentraînement : minuit UTC = **2h du matin heure de Paris** (inadapté)

---

## ÉVOLUTION PRÉVUE — Straddle : prédiction de pics de volatilité par les news

### Objectif
Aujourd'hui, la Straddle détecte les créneaux de forte volatilité **uniquement à partir de l'ATR historique**. L'évolution consiste à alimenter le modèle avec la **revue de presse économique et macro** pour anticiper les pics *avant* qu'ils ne se produisent, et non après.

### Principe
Un LLM (Ollama local) analyse quotidiennement les annonces économiques prévues (calendrier économique, publications macro) et évalue leur impact potentiel sur la volatilité de chaque asset :
- Publications macro à fort impact : NFP, CPI, PPI, FOMC, PIB, ISM
- Événements géopolitiques détectés dans un flux de nouvelles
- Corrélations croisées : annonce USD → impact sur XAUUSD et BTC

### Ce qui est nécessaire
1. **Source de données news** : intégrer un flux calendrier économique (ex. Forex Factory API ou scraping) + flux nouvelles (RSS économiques)
2. **Nouveau composant** `ollama/straddle_news.rs` : le LLM reçoit les annonces des 48h à venir et produit un score de risque de volatilité par créneau horaire + par asset
3. **Fusion avec le score ATR** : `score_final = 0.6 × score_atr_historique + 0.4 × score_news_llm`
4. **Prompt Straddle** à redéfinir pour intégrer ce contexte news (revoir `PROMPT_ANALYSE_STRADDLE`)
5. **Données d'entraînement** : labelliser rétrospectivement des pics ATR avec les annonces qui les ont causées, pour fine-tuner `XgbStraddle`

### Architecture cible
```
Calendrier économique (API) ──→ straddle_news.rs (LLM Ollama GPU)
                                        │ score_news par créneau
ATR historique ──────────────→ straddle_analyse.rs
                                        │ score_atr
                                        └──→ Fusion ──→ Créneaux Straddle enrichis
```

### Contraintes
- Le calcul news doit rester côté **backend Rust** (règle DAG stricte)
- Le LLM news opère en batch quotidien, pas en temps réel (coût VRAM)
- Le semaphore global Ollama absorbe naturellement cet appel supplémentaire
- **Non bloquant pour les étapes 4-7** — à planifier après l'optimisation hardware

---

## Règle de progression

> **Valider chaque étape avant de passer à la suivante.**
> Critère minimum : `cargo test --workspace` vert + métriques de validation atteintes.
> En cas d'échec : rollback de l'étape en cours uniquement.

---

## ÉTAPE 0 — Diagnostic : mesurer les baselines ✅
**Résultats mesurés :**
- GPU : RTX 3090, 19 304 MiB libre au repos, charge 14-16%
- Ollama filtre Rockets : ~1.4s | SMC filtre : ~6.4s (contexte plus riche)
- LSTM déjà sur CUDA:0 en production
- Scheduler : déclenchement à minuit UTC (2h Paris)
- nvidia-smi pendant inférence Ollama avec num_gpu:99 : charge montée à 39%, VRAM 22.9 GB
**Risque : zéro | Durée : 30 minutes**

Avant de toucher quoi que ce soit, consigner les références actuelles :

- Temps d'un cycle de réentraînement complet (logs scheduler)
- Latence Ollama par type d'appel (filtre Rockets, SMC, vision, coach)
- Utilisation GPU pendant un filtre Ollama : `watch -n1 nvidia-smi`
- Accuracy actuelle (endpoint `GET /api/ml/retrain/status`)
- RAM et VRAM utilisées au repos et pendant l'entraînement

**Validation** : valeurs de référence consignées ici pour comparaison ultérieure.

| Métrique | Valeur baseline | Objectif final |
|----------|----------------|----------------|
| Latence filtre Rockets (Ollama) | à mesurer | < 2s |
| Latence vision chart (Ollama) | à mesurer | < 5s |
| Temps 1 combinaison ML | ~74s | ~2s |
| Temps 58 combinaisons total | ~71 min | < 1 min |
| GPU Load pendant filtres | à mesurer | > 70% |
| GPU Load pendant entraînement | à mesurer | > 70% |

---

## ÉTAPE 1 — Ollama : forcer le GPU et unifier la configuration ✅
**Implémenté — tous les appels Ollama passent désormais par `num_gpu:99` + `OLLAMA_SEMAPHORE` global + `OLLAMA_HTTP_CLIENT` statique.**
- Retry ajouté sur `smc_filtre.rs` (identique à Rockets)
- Straddle : `Corps` struct remplacé par `serde_json::json!` avec options complètes
- Vision : semaphore local supprimé → semaphore global
- `MessageOllama` / `RequeteOllama` (dead code) supprimés de `types.rs`
**Risque : faible | Gain : latence LLM divisée par 5 à 10**

### Problème
- Ollama s'exécute en CPU par défaut si `num_gpu` n'est pas spécifié dans les options
- Chaque module (Rockets, SMC, Straddle, vision, coach) a ses propres paramètres en dur, incohérents
- Le semaphore de concurrence (`Semaphore::new(2)`) n'existe que dans `vision.rs` — les filtres Rockets/SMC peuvent se chevaucher sans limite

### Ce qui change
- Créer une structure `OptionsOllama` centrale dans `ollama/types.rs` avec `num_gpu`, `num_ctx`, `num_predict`, `temperature`
- Tous les appels Ollama passent par cette structure avec des presets par type :

| Type d'appel | `num_ctx` | `num_predict` | `num_gpu` | `temperature` |
|-------------|-----------|---------------|-----------|---------------|
| Filtre JSON (Rockets, SMC) | 4 096 | 300 | 99 | 0.1 |
| Vision chart (1 TF) | 32 768 | 2 048 | 99 | 0.2 |
| Vision chart (multi-TF) | 65 536 | 3 072 | 99 | 0.2 |
| Coach / Diagram | 16 384 | 2 048 | 99 | 0.3 |

- Créer un **semaphore global unique** pour tous les appels Ollama : `Semaphore::new(3)`
- Supprimer le semaphore local de `vision.rs`

### Validation
1. `nvidia-smi` pendant un filtre Rockets → GPU Load > 70%
2. Latence filtre : < 2s
3. Latence vision : < 5s
4. Aucun signal bloqué sous charge (timeout Ollama toujours respecté)
5. `cargo test --workspace` vert

---

## ÉTAPE 2 — Heure du scheduler : 2h Paris → 19h Paris ✅
**Implémenté dans `scheduler.rs` — `HEURE_CIBLE = 17` (17h UTC = 19h Paris été / 18h Paris hiver).**
**Risque : minimal | Durée : 15 minutes**

### Problème
`secondes_jusqu_a_minuit_utc()` dans `scheduler.rs` déclenche le réentraînement à **minuit UTC = 2h du matin heure de Paris** — inadapté à une surveillance active.

### Ce qui change
- Renommer la fonction `secondes_jusqu_a_17h_utc()`
- Cibler **17h UTC = 19h Paris (heure d'été) / 18h Paris (heure d'hiver)**
- Même logique de calcul, seule la cible horaire change

### Validation
Au démarrage, le log doit afficher `"prochain entraînement dans Xh Xm"` cohérent avec 17h UTC.

---

## ÉTAPE 3 — Refactoring `features.rs` : pré-calcul des indicateurs ✅
**Implémenté — `features_precalc.rs` créé. `lib.rs` et `walk_forward.rs` utilisent `precalculer()` + `extraire_depuis_series()`. Inférence temps réel inchangée. Tests : 84/84 ✅**
**Risque : moyen | Prérequis obligatoire pour l'étape 4**

### Problème (erreur dans l'ancien plan)
L'ancien plan affirmait que les features sont "indépendantes entre elles" et qu'on pouvait simplement ajouter `par_iter()`. **C'est faux.**

`extraire_features(&bougies[..=i])` recalcule EMA, RSI, ATR, MACD, Bollinger **depuis le début** pour chaque indice `i`. Sur 50 000 bougies, cela représente ~50 000 recalculs complets au lieu d'un seul passage. C'est pourquoi on ne peut pas paralléliser naïvement : chaque appel relit tout le slice.

### Ce qui change
1. Créer `extraire_features_precalc(indicateurs: &IndicateursPrecalcs, i: usize)` qui reçoit les vecteurs déjà calculés et lit uniquement l'indice `i`
2. Créer `struct IndicateursPrecalcs { ema9, ema21, ema50, rsi14, atr14, macd, bollinger }` — calculés une seule fois avant la boucle
3. Dans `walk_forward.rs` : pré-calculer, puis itérer avec la nouvelle fonction
4. L'ancienne `extraire_features()` reste pour l'inférence unitaire (un signal = une bougie)

### Validation
- `cargo test --workspace` vert
- Sortie **bit-à-bit identique** entre ancienne et nouvelle implémentation (test unitaire dédié)
- Temps extraction 50k bougies : objectif < 2s (contre ~20s)

---

## ÉTAPE 4 — Parallélisation Rayon : feature extraction
**Risque : moyen | Gain : ×8 à ×15 sur l'extraction**
**Prérequis : Étape 3 ✅ terminée**

### Ce qui change
- Ajouter `rayon` dans `ml/Cargo.toml` (il est dans le workspace `backend/Cargo.toml` mais pas dans le crate `ml`)
- Dans `walk_forward.rs` : la boucle `for i in 60..train.len()` devient `(60..train.len()).into_par_iter().map(...).collect()`
- Possible car chaque indice `i` est maintenant **réellement indépendant** grâce au pré-calcul de l'étape 3
- Le LSTM reste séquentiel (ses poids internes ne sont pas `Send`) — seule la phase feature extraction est parallélisée
- `Vec<features>` et `Vec<labels>` collectés via `par_iter`

### Validation
- `cargo test --workspace` vert
- Accuracy identique ±0.001 (légère variation possible due à l'ordre de collecte des flottants)
- Temps d'extraction mesuré avec `Instant::now()` : objectif < 2s pour 50k bougies

---

## ÉTAPE 5 — Parallélisation du scheduler : 58 combinaisons simultanées
**Risque : moyen-élevé | Gain : ×10 à ×15 sur le temps total**
**Prérequis : Étapes 3 et 4 validées**

### Problème architectural
Le `pipeline_ml: Arc<Mutex<PipelineML>>` est partagé et verrouillé pendant tout l'entraînement de chaque combinaison — il ne peut pas être utilisé tel quel par plusieurs tâches simultanées.

### Ce qui change dans `scheduler.rs`
- La boucle `for combinaison in combinaisons` devient un ensemble de `tokio::spawn` (async, pas Rayon — pour la compatibilité SQLx)
- Un `Semaphore::new(8)` limite à 8 tâches simultanées (adapté aux 20 cœurs disponibles)
- **Chaque tâche instancie son propre `PipelineML::new()` local** : entraîne dessus, sauvegarde le résultat en DB, se termine
- Le pipeline global (`Arc<Mutex<PipelineML>>`) n'est mis à jour **qu'une seule fois** après que toutes les tâches sont terminées, en chargeant le meilleur modèle sauvegardé
- SQLite en mode WAL : les lectures parallèles sont OK, les écritures s'enchaînent naturellement

### Validation
- Logs : plusieurs combinaisons démarrées simultanément visibles
- Les 58 résultats sont présents en DB
- Accuracy globale identique à ±2%
- Temps total : objectif < 5 minutes (puis < 1 min avec étapes 6+7)
- Le réentraînement manuel (endpoint `POST /api/ml/retrain`) fonctionne toujours
- Le rollback automatique fonctionne si accuracy dégradée

---

## ÉTAPE 6 — Activer la feature CUDA pour le LSTM
**Risque : faible (feature isolée) | Gain : entraînement LSTM ×10**

### Problème
La feature `cuda` existe dans `ml/Cargo.toml` et le code GPU est déjà écrit (`lstm/gpu.rs`, `lstm/entrainement_gpu.rs`), mais elle n'est jamais activée dans le build → LSTM tourne en CPU pur.

### Ce qui change
- Dans la commande de build et `scripts/run.sh` : ajouter `--features ml/cuda`
- Vérifier que LibTorch est bien compilé avec support CUDA 11.8+ (variable `LIBTORCH` ou `LIBTORCH_USE_PYTORCH`)
- Dans `scheduler.rs` : après le chargement des poids, appeler `pipeline.activer_gpu_si_pret()` pour transférer sur CUDA

### Validation
- `nvidia-smi` pendant l'entraînement LSTM : GPU Load > 70%
- Temps entraînement LSTM par combinaison : objectif < 100ms (contre ~800ms)
- Accuracy identique (les poids GPU = poids CPU transférés)

---

## ÉTAPE 7 — Migration XGBoost GPU (libxgboost C++)
**Risque : élevé | Gain : ×10 à ×20 sur XGBoost**
**À faire en dernier — migration complète de librairie**

### Ce qui change
- Remplacer `smartcore::xgboost` par le crate `xgboost` (bindings C++ libxgboost natif)
- Paramètres : `tree_method: "gpu_hist"`, `device: "cuda:0"`
- Les modèles `.json` existants doivent être **réentraînés** (format incompatible — pas de migration possible)
- Fichiers à réécrire intégralement : `xgboost.rs`, `rockets_trainer.rs`, `smc_trainer.rs`, `straddle_trainer.rs`
- L'interface publique (fonctions `entrainer`, `predire`) reste identique pour ne pas impacter le reste du code

### Validation
- `cargo test --workspace` vert
- Accuracy XGBoost OOS comparable ±3% (algorithme identique, implémentation différente)
- Temps XGBoost par combinaison : objectif < 2s (contre ~120s)
- `nvidia-smi` : GPU utilisé pendant XGBoost

---

## Tableau des gains cumulatifs attendus

| Étape | Action | Temps 1 combinaison | Temps 58 combinaisons | Ollama filtres |
|-------|--------|--------------------|-----------------------|---------------|
| Baseline | Actuel | ~74s | ~71 min | ~8-15s |
| Étape 1 | Ollama GPU | ~74s | ~71 min | **~1-2s** |
| Étape 2 | Heure scheduler | ~74s | ~71 min | ~1-2s |
| Étapes 3+4 | Pré-calcul + Rayon | **~8s** | ~8 min | ~1-2s |
| Étape 5 | Parallélisme scheduler | ~8s | **~50s** | ~1-2s |
| Étape 6 | LSTM CUDA | **~5s** | ~30s | ~1-2s |
| Étape 7 | XGBoost GPU | **~2s** | **~15s** | ~1-2s |

---

## Ce qui ne peut PAS être optimisé facilement

- **SQLite écritures** : pas conçu pour l'écriture concurrente. Le mode WAL suffit pour les lectures parallèles, les écritures restent séquentielles — acceptable.
- **tch/LibTorch thread-safety** : les tenseurs GPU ne sont pas `Send+Sync`. Solution : 1 GPU context par process, pas par thread. Avec le parallélisme du scheduler, l'entraînement LSTM reste séquentiel par tâche (chaque tâche a son propre contexte CPU), seule l'inférence utilise le GPU partagé.
- **Anthropic Claude** : cloud externe, non optimisable côté matériel.
