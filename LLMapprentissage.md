# Plan d'optimisation IA — Native Trading AI (Tâches Restantes)

## ÉTAPE 1 — Activer la feature CUDA pour le LSTM
**Prérequis :** L'entraînement CPU multi-thread est pleinement opérationnel (220s). Implémentation GPU déjà codée.
**Objectif :** Transférer l'entraînement LSTM du CPU vers le GPU (RTX 3090).
- Activer la feature dans la compilation (`--features ml/cuda`) via le `Cargo.toml` global ou les scripts de build (`scripts/run.sh`).
- Dans `scheduler.rs` (après le chargement initial), invoquer `pipeline.activer_gpu_si_pret()` pour basculer le contexte sur la carte graphique.
- **Validation** : Load VRAM visible via `nvidia-smi`, temps d'entraînement LSTM réduit sous 100ms.

## ÉTAPE 2 — Migration XGBoost GPU (libxgboost C++)
**Prérequis :** Étape 1 validée (nécessite de réentraîner les modèles de zéro).
**Objectif :** Remplacer le XGBoost CPU basique par l'implémentation GPU native.
- Remplacer le crate `smartcore` par les bindings natifs `xgboost`.
- Configurer les paramètres GPU : `tree_method: "gpu_hist"`, `device: "cuda:0"`.
- Refactoriser en profondeur les implémentations dans `xgboost.rs`, ainsi que les appels dans les trainers respectifs (`rockets_trainer.rs`, `smc_trainer.rs`, `straddle_trainer.rs`).
- Réentraîner et exporter tous les modèles au nouveau format binaire/json du crate natif.
- **Validation** : Inférence fonctionnelle, temps d'entraînement XGBoost écrasé par le GPU (cible < 1s par run sur max samples).
