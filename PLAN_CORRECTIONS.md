# PLAN DE CORRECTIONS — RESTANT A FAIRE

Contexte produit retenu:
- L'application sert a generer des signaux.
- L'utilisateur execute manuellement les trades.
- Aucun objectif de trading automatique live broker dans ce plan.

Date de mise a jour: 28 mars 2026

---

## Priorite 1 - Phase 3 (Qualite IA/ML) [BLOQUANT AVANT PHASE 4]

Objectif:
- Rendre la qualite du modele mesurable, fiable et controlable.

### 3.1 - Validation OOS (holdout) dans le pipeline ML ✅ TERMINÉ
Fichier principal:
- backend/crates/ml/src/lib.rs

A faire:
1. Reserver les 20% dernieres donnees comme jeu de validation (holdout chronologique).
2. Calculer et journaliser separement:
   - accuracy_train
   - accuracy_val
3. Exposer accuracy_val cote API puis UI dashboard ML.
4. Ajouter alerte UI si accuracy_val < 52%.

Criteres de fin:
- L'entrainement affiche train + val distincts.
- L'UI montre accuracy_val.
- Le warning faible fiabilite est fonctionnel.

### 3.2 - Retirer l'estimation RF heuristique / code mort ✅ TERMINÉ
Fichiers principaux:
- backend/crates/ml/src/modele.rs
- backend/crates/ml/src/lib.rs

A faire:
1. Supprimer l'approche d'estimation probabiliste par perturbation de features (si encore utilisee).
2. Supprimer ModeleRandomForest si non utilise en production.
3. Garder une sortie coherente centree sur le pipeline en usage (XGBoost/LSTM).

Criteres de fin:
- Aucun export/usage actif de ModeleRandomForest dans le backend.
- Pipeline ML garde un comportement fonctionnel et teste.

### 3.3 - Robustesse d'entrainement LSTM
Fichier principal:
- backend/crates/ml/src/lstm/mod.rs

A faire:
1. Ajouter gradient clipping (norme max, ex: 5.0).
2. Detecter NaN/inf dans les poids et journaliser explicitement.
3. Ajouter early stopping (ex: patience 3 epochs sans amelioration).

Criteres de fin:
- Entrainement ne diverge pas silencieusement.
- Logs explicites en cas de derive.

### 3.4 - Rate limiting centralise Ollama
Fichiers principaux:
- backend/crates/api/src/ollama/mod.rs
- backend/crates/api/src/state.rs

A faire:
1. Ajouter un Semaphore Tokio partage (ex: 2 appels concurrents max).
2. Forcer tous les appels Ollama a passer par ce controle.
3. Ajouter timeout global appel (60s) en plus du timeout HTTP.

Criteres de fin:
- Pas d'emballement concurrent sur appels Ollama.
- Degradation propre sous charge.

### 3.5 - Reentrainement periodique pilote metrique
Fichiers principaux:
- backend/crates/api/src/state.rs
- backend/crates/ml/src/lib.rs
- backend/crates/db (nouvelle table perf)

A faire:
1. Worker periodique (24h) qui surveille accuracy_val.
2. Declencher reentrainement auto si sous seuil.
3. Persister l'historique des performances (table ml_performances).
4. Exposer l'etat en UI (derniere perf, dernier retrain).

Criteres de fin:
- Boucle de surveillance active.
- Historique de perf visible.

### Tests de sortie Phase 3
1. Lancer entrainement: verifier train/val distincts.
2. Forcer cas accuracy_val bas: warning UI actif.
3. Verifier absence de concurrence Ollama non controlee.
4. Verifier declenchement worker de reentrainement (simulation seuil).
5. Verifier non-regression:
   - cargo test --workspace

---

## Priorite 2 - Phase 4 (Architecture/Nettoyage)

Objectif:
- Assainir le front et finaliser la coherence runtime.

### 4.1 - Supprimer les doublons .js dans frontend/src
Zone:
- frontend/src/**

A faire:
1. Identifier les fichiers .js doublons de .ts/.vue.
2. Supprimer les doublons non necessaires.
3. Verifier qu'aucun import runtime ne depend des fichiers supprimes.
4. Renforcer exclusions de build dans la config adaptee.

Criteres de fin:
- Aucun doublon .js residuel non justifie dans frontend/src.
- Build frontend propre.

### 4.2 - Finaliser suppression code ML mort
Zone:
- backend/crates/ml

A faire:
1. Verifier et supprimer references restantes au RandomForest si obsolete.
2. Maintenir API/types propres apres suppression.

Criteres de fin:
- Backend ML sans references mortes.

### 4.3 - Poursuivre extraction logique metier hors couche API
Zone:
- backend/crates/api/src
- backend/crates/backtest/src
- backend/crates/strategies/src

A faire:
1. Continuer le deplacement des blocs metier encore presents dans API.
2. Garder handlers API minces: parse -> appel service -> reponse.

Criteres de fin:
- Couche API centree transport HTTP.

### 4.4 - WebSocket signaux
Statut:
- Deja en place via composable useSignalEngine.

Action restante:
1. Verification de robustesse et stabilite en usage prolonge.

### 4.5 - Corriger ml-pret hardcode dans SmcView
Fichier principal:
- frontend/src/views/SmcView.vue

A faire:
1. Lire /api/ml/status au montage.
2. Passer la vraie valeur au composant DashboardSystemStatus.

Criteres de fin:
- Affichage ML pret/non pret fiable dans SmcView.

### Tests de sortie Phase 4
1. Verifier suppression doublons .js.
2. npm run build sans erreurs d'import.
3. Verifier affichage ml-pret reel dans SmcView.
4. Verifier signaux temps reel recus en UI via WS.
5. Verifier non-regression:
   - cargo test --workspace
   - cargo clippy --workspace -- -D warnings

---

## Priorite 3 - Phase 5 (Parametrage MQ5 + Backtest pilotable)

Objectif:
- Permettre l'edition de parametres style MQ5 et relancer les backtests depuis l'UI.

### 5.1 - Parametres Straddle (BiDiParams) + endpoints
A faire:
1. Struct params dediee.
2. GET/PUT API params Straddle.
3. Persistance DB.
4. Backtest Straddle pilote par ces parametres.

### 5.2 - Parametres SMC (UniParams) + endpoints
A faire:
1. Struct params dediee.
2. GET/PUT API params SMC.
3. Persistance DB.
4. Backtest SMC pilote par ces parametres.

### 5.3 - Fonctionnalites de backtest manquantes
A implementer:
1. Break-even.
2. Trailing stop.
3. Expiration.
4. TP partiels.

### 5.4 - UI de pilotage backtest
A faire:
1. Formulaires parametres dans vues Straddle/SMC.
2. Debounce + relance auto du backtest a chaque modification.
3. Affichage immediat des resultats recalcules.

### Tests de sortie Phase 5
1. Changer ATRPeriod / TP ratio -> resultats backtest changent de facon coherente.
2. Verifier persistance des parametres apres reload UI.
3. Comparaison Rust vs MT5 sur cas temoin (ecart acceptable defini).
4. Verifier non-regression:
   - cargo test --workspace

---

## Ordre d'execution impose
1. Terminer entierement Priorite 1 (Phase 3).
2. Puis Priorite 2 (Phase 4).
3. Puis Priorite 3 (Phase 5).

Ne pas demarrer la priorite suivante tant que les tests de sortie de la priorite en cours ne sont pas valides.
