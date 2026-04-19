# Plan d'optimisation matérielle — Native Trading AI

## Matériel disponible
- CPU : Intel i9, 20 cœurs
- RAM : 48 GB
- GPU : NVIDIA RTX 3090, 24 GB VRAM

---

## Situation actuelle

### Ce qui est fait
- LSTM inférence : GPU (tch/LibTorch CUDA) ✅
- LSTM entraînement : GPU (cuDNN via tch) ✅
- Extraction de features : CPU, multi-cœurs (Rayon) ✅
- XGBoost : CPU, 1 cœur, librairie Rust pure (smartcore)
- Réentraînement : 58 combinaisons traitées **une par une**, séquentiellement

### Conséquence
58 combinaisons × ~74s = ~71 minutes pour un cycle complet.
Le CPU est utilisé à ~5%, le GPU à ~1%, la RAM à ~1%.

---

## Plan d'optimisation — 3 niveaux

---

### Niveau 1 — Parallélisme CPU (Rayon)
**Gain estimé : ×10 à ×15 sur le temps total**
**Risque : modéré**
**Priorité : Phase 3**

#### Principe
Traiter les 58 combinaisons en parallèle sur les 20 cœurs au lieu de séquentiellement.
Librairie : `rayon` (déjà dans l'écosystème Rust, zéro overhead).

#### Ce qui change
- `scheduler.rs` : la boucle `for combinaison in combinaisons` devient `par_iter()`
- Chaque thread instancie son propre `PipelineML` local → pas de conflit
- SQLite : lecture en parallèle OK (connexions séparées par thread via SQLx)
- GPU : **problème** — tch/LibTorch n'est pas thread-safe par défaut.
  Solution : chaque thread fait l'entraînement CPU, le GPU reste pour l'inférence uniquement
  (ou : 1 Mutex global GPU, les threads font la queue — moins efficace)

#### Résultat attendu
58 combinaisons en ~5 minutes au lieu de 71 minutes.

---

### Niveau 2 — XGBoost GPU natif
**Gain estimé : ×5 à ×20 sur la partie XGBoost**
**Risque : élevé (remplacement complet de librairie)**
**Priorité : Phase 4**

#### Principe
Remplacer `smartcore` (XGBoost Rust pur, 1 cœur) par les bindings C++ `libxgboost`
qui supportent `tree_method=gpu_hist` → entraînement sur la 3090.

#### Ce qui change
- `xgboost.rs` entièrement réécrit avec la lib `xgboost` crate (bindings libxgboost C++)
- Format de sérialisation des modèles change (JSON propriétaire XGBoost vs actuel)
- Migration des modèles existants nécessaire
- Les 58 combinaisons XGBoost s'entraîneraient en quelques secondes au total

#### Résultat attendu
Partie XGBoost : de ~120s/combinaison à ~2s/combinaison.

---

### Niveau 3 — Extraction de features parallèle (Rayon/Polars) ✅ TERMINE
**Gain estimé : ×8 à ×15 sur l'extraction**
**Risque : faible (module isolé)**
**Statut : Implémenté dans `ml/src/lib.rs` et `walk_forward.rs` via `rayon::par_iter**

#### Principe
La boucle `for i in 60..bougies.len()` qui calcule ATR/RSI/MACD/etc. sur 50k bougies
est 100% séquentielle. Elle peut être découpée en chunks parallèles avec `rayon::par_iter`.

#### Ce qui change
- `ml/src/features.rs` uniquement
- Les features sont indépendantes entre elles (pas d'état partagé à t+1)
- Résultat : même `Vec<features>` en sortie, juste calculé en parallèle

#### Résultat attendu
Extraction 50k bougies : de ~20s à ~2s par combinaison.

---

## Ordre d'implémentation recommandé

```
Phase 3 (après stabilisation stratégies) :
  1. ~~Niveau 3 — Feature extraction parallèle (faible risque, bon gain)~~ ✅ Fait
  2. Niveau 1 — Rayon sur les 58 combinaisons (risque modéré, gain ×10)

Phase 4 (production stable) :
  3. Niveau 2 — XGBoost GPU natif (risque élevé, migration complète)
```

---

## Résultat final attendu (tout activé)

| Étape | Avant | Après |
|---|---|---|
| Feature extraction (50k bougies) | ~20s | ~2s |
| XGBoost entraînement | ~120s | ~2s |
| LSTM entraînement | ~800ms (GPU) | ~800ms (déjà GPU) |
| 1 combinaison totale | ~74s | ~5s |
| 58 combinaisons (parallèle) | ~71 min | ~30s |

---

## Ce qui ne peut PAS être optimisé facilement

- **SQLite** : pas conçu pour l'écriture concurrente. Acceptable en lecture parallèle,
  l'écriture reste séquentielle (WAL mode suffisant).
- **tch/LibTorch thread-safety** : les tenseurs GPU ne sont pas `Send+Sync`.
  Solution définitive = 1 GPU context par process, pas par thread.
  Donc avec Rayon : entraînement CPU parallèle + inférence GPU séquentielle.
