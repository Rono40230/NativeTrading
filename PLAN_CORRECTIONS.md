# PLAN DE CORRECTIONS — Native Trading AI
> Ordre de priorité : Sécurité du capital → Fiabilité des signaux → Qualité IA → Nettoyage
> Chaque phase se termine par un protocole de test avant de passer à la suivante.

---

## PHASE 1 — CORRECTIONS BLOQUANTES ✅ TERMINÉE
> **Objectif** : L'app ne doit pas pouvoir générer un signal qui met le capital en danger par erreur de code.

### 1.1 — Corriger les TP/SL asymétriques du signal Straddle

**Fichier** : `backend/crates/strategies/src/straddle.rs`

**Problème** : Le `Signal` retourné avec `Direction::Both` ne construit que les niveaux prix de la jambe LONG (tp = prix + ATR×2, sl = prix - ATR×0.5). La jambe SHORT n'a aucun TP ni SL dans la structure retournée.

**Ce qu'il faut faire** :
- Étendre la structure `Signal` (dans `common`) pour porter les deux jambes séparément, **OU**
- Retourner deux signaux distincts (un LONG, un SHORT) depuis la stratégie Straddle
- Vérifier que `straddle_slot_backtest.rs` et `straddle_signal_handler.rs` utilisent les mêmes niveaux
- Aligner les constantes TP/SL : le backtest utilise `TP_MULT=2.0 / SL_MULT=0.5`, l'affichage frontend et le prompt LLM mentionnent `TP1=2×ATR / TP2=3.5×ATR` — une seule source de vérité

### 1.2 — Supprimer le fallback `ia_indecise = true`

**Fichier** : `backend/crates/strategies/src/straddle.rs`

**Problème** : Sans ML entraîné, tout high-ATR déclenche un Straddle. C'est une faille de sécurité trading.

**Ce qu'il faut faire** :
- Si `pipeline_ml = None` ou modèle non prêt → retourner `Ok(None)` immédiatement
- Ajouter un état de statut ML explicite exposé via l'API (`/api/ml/status`)
- Afficher un bandeau bloquant dans l'UI si ML non prêt avant d'activer le moteur de signaux

### 1.3 — Connecter le capital réel au GestionnaireRisque

**Fichier** : `backend/crates/risk/src/lib.rs`, `backend/crates/api/src/state.rs`

**Problème** : Le capital passé au `GestionnaireRisque` est une valeur fixe (formulaire ou constante) jamais synchronisée avec le solde réel du broker.

**Ce qu'il faut faire** :
- Créer un endpoint API IB Gateway / Binance pour récupérer le solde réel au démarrage
- Stocker et rafraîchir le capital dans l'`AppState` toutes les X minutes
- Le `GestionnaireRisque` doit lire ce capital live avant chaque validation de signal

---

### 🧪 TESTS PHASE 1

**Avant de passer à la Phase 2, vérifier manuellement dans l'app :**

- [x] Démarrer l'app **sans** modèle ML entraîné → le moteur de signaux SMC et Straddle doit afficher un état "ML non prêt" et ne pas générer de signaux
- [x] Déclencher un signal Straddle manuel sur `XAUUSD` → vérifier que le signal affiché contient **deux jambes** avec des niveaux TP/SL cohérents pour Long ET Short
- [x] Vérifier dans les logs que le capital utilisé dans les calculs de taille de position correspond au solde affiché par le broker
- [x] Lancer `cargo test --workspace` → zéro régression

---

## PHASE 2 — FIABILITÉ DES SIGNAUX ✅ TERMINÉE

### 2.1 — Unifier les 3 implémentations ATR

**Fichiers** : 
- `backend/crates/api/src/rockets_indicateurs.rs` → `calc_atr()` (SMA simple)
- `backend/crates/api/src/straddle_slot_backtest.rs` → `calculer_atr()` (SMA locale)
- `backend/crates/indicators/` → implémentation de référence (EMA Wilder)

**Ce qu'il faut faire** :
- Supprimer les deux fonctions ATR locales dans `api/`
- Les remplacer par des appels à `indicators::calculer_atr()`
- Attention : cela peut modifier légèrement les résultats du backtest Rockets et Straddle (c'est voulu — on veut la réalité, pas une approximation)

### 2.2 — Aligner les barèmes SMC entre scorer Rust et prompts LLM

**Fichiers** : 
- `backend/crates/smc/src/lib.rs` → source de vérité (Tendance=25, OB=25, IFVG=20, Imb=15, Fib=15)
- `backend/crates/api/src/ollama_handlers.rs` → prompt indique Tendance=30, OB=20, IFVG=10... 

**Ce qu'il faut faire** :
- Extraire les barèmes comme constantes dans `smc/src/lib.rs`
- Générer dynamiquement la section "Détail SMC" du prompt à partir des mêmes constantes
- Vérifier que le prompt LLM de validation SMC (`smc_filtre.rs`, `smc_analyse.rs`) utilise également les vrais barèmes

### 2.3 — Activer le ML dans le SignalEngine automatique

**Fichier** : `backend/crates/api/src/signal_engine.rs`

**Problème** : `SmcDirectionalStrategy` est instancié sans `PipelineML` → le filtre ML n'est jamais appliqué en mode automatique.

**Ce qu'il faut faire** :
- Passer une référence `Arc<Mutex<PipelineML>>` au `SignalEngine` depuis l'`AppState`
- Modifier `SmcDirectionalStrategy` pour accepter un pipeline ML optionnel (même pattern que `StraddleStrategy`)
- Si ML prêt → gate confiance ≥ 60% sur le signal avant de le publier

### 2.4 — Corriger le filtre assets Straddle incohérent

**Fichiers** :
- `frontend/src/views/StraddleView.vue` → fallback `BTCUSDT`
- `frontend/src/views/StraddleBacktestView.vue` → filtre `a.id === 'BTC'`

**Ce qu'il faut faire** :
- Aligner les deux vues sur la même logique de filtre (l'identifiant utilisé dans la DB)
- Extraire la liste des assets Straddle dans un composable ou constante partagée

### 2.5 — Corriger l'envoi de `heure_fin` dans le backtest Straddle

**Fichier** : `frontend/src/views/StraddleBacktestView.vue`

**Problème** : L'utilisateur saisit `heure_fin` mais elle n'est jamais envoyée dans l'appel API `runStraddleSlotBacktest`.

**Ce qu'il faut faire** :
- Ajouter `heure_fin` dans le corps de la requête
- Vérifier que `straddle_backtest_handler.rs` et `straddle_slot_backtest.rs` utilisent correctement la plage complète

### 2.6 — Corriger `TP3 = 20×ATR` dans Rockets

**Fichier** : `backend/crates/api/src/rockets_scan.rs`

**Problème** : `tp3 = prix + 20.0 * atr14` est un objectif "moonshot" sans fondement technique qui fausse l'espérance calculée.

**Ce qu'il faut faire** :
- Remplacer par une cible basée sur le `measured move` (hauteur de base × 2) déjà calculé dans le scan
- Proposer comme paramètre configurable dans `RocketsConfig` (ex : `tp3_mult` exposé dans l'UI Réglages)

---

### 🧪 TESTS PHASE 2

- [x] Lancer un backtest Straddle sur `XAUUSD` avec une `heure_debut` ET `heure_fin` différentes → vérifier que le filtre de plage fonctionne (le nombre de trades doit changer)
- [x] Vérifier dans les logs du Signal Engine qu'un signal SMC affiché a bien passé le gate ML (gate câblé — en attente de premier signal SMC organique pour valider le log)
- [x] Ouvrir l'UI Rockets → un signal sauvegardé doit avoir un TP3 cohérent (pas 20×ATR)
- [x] Comparer le résultat d'un backtest Rockets avant/après unification ATR et noter l'écart
- [x] Vérifier dans le prompt LLM SMC (logs Ollama) que les barèmes mentionnés correspondent à ceux du scorer
- [x] `cargo test --workspace` → **59 tests, 0 échecs** ✅

---

## PHASE 3 — QUALITÉ IA ET ML

### 3.1 — Ajouter validation OOS au pipeline ML (holdout)

**Fichier** : `backend/crates/ml/src/lib.rs`

**Problème** : L'accuracy retournée est mesurée sur le jeu d'entraînement — elle est donc trompeuse.

**Ce qu'il faut faire** :
- Réserver les 20% dernières données comme holdout set avant entraînement
- Calculer et logguer séparément : `accuracy_train` et `accuracy_val`
- Exposer `accuracy_val` dans l'UI (dashboard ML)
- Si `accuracy_val < 52%` → déclencher un warning visible dans l'UI "modèle peu fiable"

### 3.2 — Supprimer l'estimation de probabilité par perturbation (RF)

**Fichier** : `backend/crates/ml/src/modele.rs`

**Problème** : `estimer_proba` perturbe les features par ±0.2% et compte les votes. C'est une heuristique sans base statistique qui biaise la confiance.

**Ce qu'il faut faire** :
- Utiliser le vote majoritaire natif du Random Forest (ratio votes/n_arbres)
- Si `smartcore` n'expose pas `predict_proba`, remplacer `ModeleRandomForest` par `ModeleXGBoost` qui retourne déjà un score continu via `XGRegressor`
- Supprimer `ModeleRandomForest` (code mort non utilisé dans le pipeline de production)

### 3.3 — Ajouter gradient clipping au LSTM

**Fichier** : `backend/crates/ml/src/lstm/mod.rs`

**Problème** : Le BPTT sur 3 couches peut diverger silencieusement (gradients explosifs absorbés par `safe()`).

**Ce qu'il faut faire** :
- Ajouter un clip de gradient (ex : norme max = 5.0) dans la boucle BPTT
- Détecter les NaN dans les poids après chaque epoch et logguer un `error!` explicite (pas silencieux)
- Ajouter un early stopping : si `loss` n'améliore pas sur 3 epochs consécutives → arrêt

### 3.4 — Ajouter une queue Ollama avec rate limiting

**Fichier** : `backend/crates/api/src/ollama/mod.rs`

**Problème** : Ollama peut être appelé simultanément depuis le Signal Engine (42 cycles), le handler de signal, le coach SMC, l'analyse Rockets — sans aucun contrôle.

**Ce qu'il faut faire** :
- Créer un `Semaphore` Tokio (ex : max 2 appels simultanés) partagé dans l'`AppState`
- Tous les appels `appeler_ollama()` doivent acquérir ce sémaphore
- Ajouter un timeout global de 60s par appel (distinct du timeout de la requête HTTP client)

### 3.5 — Mécanisme de réentraînement périodique

**Ce qu'il faut faire** :
- Créer un worker Tokio qui vérifie toutes les 24h si `accuracy_val` a chuté sous un seuil configurable
- Si oui → déclencher automatiquement `entrainer_sur_historique()` et notifier l'utilisateur dans l'UI
- Stocker l'historique des performances du modèle dans la DB (table `ml_performances`)

---

### 🧪 TESTS PHASE 3

- [ ] Lancer un entraînement ML → vérifier que l'UI affiche deux valeurs distinctes : accuracy entraînement et accuracy validation
- [ ] Si `accuracy_val < 52%` → vérifier qu'un warning s'affiche dans le dashboard
- [ ] Ouvrir les logs pendant une session normale → vérifier qu'aucun appel Ollama concurrent n'est loggué (le sémaphore limite bien)
- [ ] Simuler un `accuracy_val` bas artificiellement → vérifier que le trigger de réentraînement se déclenche
- [ ] `cargo test --workspace` → zéro régression

---

## PHASE 4 — ARCHITECTURE ET NETTOYAGE

### 4.1 — Supprimer tous les fichiers `.js` dupliqués du frontend

**Problème** : Chaque composable, vue et store existe en `.ts` ET en `.js`. Ce sont des artifacts de migration ou de build commités par erreur.

**Ce qu'il faut faire** :
- Lister tous les `.js` dupliqués : `find frontend/src -name "*.js" | grep -v node_modules`
- Les supprimer si leur équivalent `.ts` existe
- Vérifier que `.gitignore` exclut les fichiers de build Vite (`*.vue.js`)
- Lancer `npm run build` pour confirmer qu'aucun fichier `.js` manquant n'est importé

### 4.2 — Supprimer `ModeleRandomForest` (code mort)

**Fichier** : `backend/crates/ml/src/modele.rs`

**Ce qu'il faut faire** :
- Vérifier avec `grep -r "ModeleRandomForest"` qu'il n'est nulle part utilisé en production
- Supprimer la classe entière
- Garder uniquement `PredictionML` (struct de résultat) dans ce fichier ou le déplacer dans `lib.rs`

### 4.3 — Déplacer la logique métier hors de la couche API

**Fichiers concernés** :
- `api/src/rockets_scan.rs` → toute la logique de scan + calcul de phase → à déplacer dans `strategies/` ou un nouveau crate `scan/`
- `api/src/straddle_slot_backtest.rs` → à déplacer dans `backtest/`
- `api/src/straddle_precision.rs` → à déplacer dans `strategies/`

**Ce qu'il faut faire** :
- Créer les fonctions publiques dans les crates concernés
- Les handlers API ne gardent que la déserialisation de la requête + appel + sérialisation de la réponse

### 4.4 — Brancher le WebSocket frontend sur le canal broadcast backend

**Fichier** : `frontend/src/composables/useSignalEngine.ts`

**Problème** : Le frontend poll HTTP toutes les X secondes alors qu'un canal WebSocket broadcast est déjà implémenté côté backend (`ws_handlers/`).

**Ce qu'il faut faire** :
- Connecter `useSignalEngine` à une WebSocket sur `/ws/signaux`
- Chaque signal émis par le `SignalEngine` arrive en temps réel dans l'UI (pas de polling)
- Supprimer le polling HTTP des signaux (garder uniquement pour le statut engine)

### 4.5 — Corriger `ml-pret` hardcodé à `false` dans SmcView

**Fichier** : `frontend/src/views/SmcView.vue`

**Ce qu'il faut faire** :
- Appeler `GET /api/ml/status` au montage du composant
- Passer le statut réel au composant `DashboardSystemStatus`

---

### 🧪 TESTS PHASE 4

- [ ] `find frontend/src -name "*.js" | grep -v node_modules` → doit retourner 0 résultat
- [ ] `cargo test --workspace` → zéro régression
- [ ] `npm run build` → build propre sans warnings d'imports manquants
- [ ] Générer un signal SMC en mode automatique → vérifier qu'il apparaît dans l'UI **en temps réel** sans rechargement de page (WebSocket)
- [ ] Vérifier que le dashboard SMC affiche correctement "ML prêt" quand les modèles sont chargés et "ML non prêt" sinon
- [ ] `cargo clippy --workspace -- -D warnings` → zéro warning

---

## PHASE 5 — PARAMÉTRAGE MQ5 ET CONNEXION BACKTEST

> Cette phase traite le sujet de la deuxième demande du fichier A_faire.txt :
> **Pouvoir modifier les paramètres `bidi.mq5` / `uni.mq5` depuis l'UI et que le backtest réagisse.**

### 5.1 — Comprendre ce que fait le backtesting MT5

Le backtester d'Expert Advisor de MT5 fonctionne ainsi :
- L'EA reçoit ses `input` paramètres au démarrage (`OnInit`)
- MT5 Strategy Tester rejoue les ticks historiques en appelant `OnTick()` / `OnTimer()`
- On peut optimiser en balayant des plages de paramètres (mode "Optimisation")
- Le résultat : equity curve, drawdown, win rate, profit factor

Ce que tu veux : **reproduire cette logique dans ton backtest Rust**, en rendant les paramètres `bidi` et `uni` éditables depuis l'UI, et en relançant le backtest Rust à chaque changement.

### 5.2 — Paramètres de `bidi.mq5` (Straddle) à exposer

| Paramètre MQ5 | Signification | Correspondance Rust actuelle |
|---|---|---|
| `RiskPercent` | Risque total (%) | `RISK_PCT = 0.01` dans backtest (hardcodé) |
| `StopLoss` | SL en points (0 = auto ATR) | `SL_MULT × ATR` dans le backtest |
| `TakeProfitRatio` | TP / SL ratio | `TP_MULT / SL_MULT` dans le backtest |
| `ExpirationMinutes` | Durée de vie max du trade | Fenêtre bougies `(i + 5)` hardcodée |
| `BreakEvenEnabled` | Break even automatique | Non implémenté dans le backtest Rust |
| `TrailingStopEnabled` | Trailing stop ATR | Non implémenté dans le backtest Rust |
| `PartialTPLevel` | Niveau de TP partielle | `TP_MULT_2`, `TP_MULT_3` dans `StraddleParams` |
| `ATRPeriod` | Période ATR | `14` hardcodé |
| `ATRTimeframe` | Timeframe ATR | M5 ou M15 selon contexte |

### 5.3 — Paramètres de `uni.mq5` (SMC Directionnel) à exposer

| Paramètre MQ5 | Signification | Correspondance Rust actuelle |
|---|---|---|
| `RiskPercent` | Risque (%) | `1.5%` hardcodé dans les commentaires |
| `StopLoss` | SL en points (0 = ATR) | `ATR_SL = 1.0` dans `smc_directional.rs` |
| `TakeProfitRatio` | TP max / SL | `ATR_TP1/TP2/TP3` hardcodés |
| `PartialTPLevel1/2` | 2 niveaux de TP partiel | `ATR_TP1=1.5`, `ATR_TP2=3.0` |
| `PartialSize1/2` | Fraction fermée à chaque TP | Non implémenté dans le backtest |
| `TrailingStopEnabled` | Trailing stop | Non implémenté |
| `ATRPeriod` | Période ATR | `14` hardcodé |
| `ExpirationMinutes` | Durée de vie max | Non implémenté |

### 5.4 — Ce que représente la refonte

**Côté backend (effort : élevé)** :
1. Créer une struct `BiDiParams` (pour Straddle) et `UniParams` (pour SMC) qui correspondent 1:1 aux inputs MQ5
2. Exposer ces params via des endpoints `GET/PUT /api/backtest/params/straddle` et `/api/backtest/params/smc`
3. Stocker les params en DB (table `backtest_params`)
4. Le backtest Straddle (`straddle_slot_backtest.rs`) doit accepter `BiDiParams` à la place des constantes hardcodées
5. Implémenter dans le backtest Rust : **break even**, **trailing stop ATR**, **expiration**, **TP partiel** (les 4 fonctionnalités absentes)
6. Le backtest SMC (`backtest/`) doit accepter `UniParams`

**Côté frontend (effort : moyen)** :
1. Créer une section "Paramètres Backtest" dans `StraddleView` et dans `SMCAnalyzerView`
2. Formulaire avec les mêmes champs que les `input` MQ5 (labels identiques)
3. À chaque modification → `PUT` les params + relancer le backtest automatiquement (debounce 1s)
4. Afficher le résultat mis à jour sans rechargement de page

**Est-ce possible ?** Oui, techniquement. C'est une refonte significative du moteur de backtest (environ 3-4 semaines de travail sérieux) mais pas un changement d'architecture fondamental. Le pattern existe déjà partiellement : `StraddleParams` dans `straddle.rs` et `RocketsConfig` dans Rockets montrent le chemin. Il faut l'étendre et le connecter au backtest.

---

### 🧪 TESTS PHASE 5

- [ ] Modifier `ATRPeriod` de 14 à 7 dans l'UI → le backtest Straddle doit retourner des résultats différents
- [ ] Modifier `TakeProfitRatio` de 2.0 à 3.0 → le `profit_factor` et le `payoff_ratio` doivent évoluer de façon cohérente
- [ ] Activer `TrailingStopEnabled` → les trades doivent avoir une durée plus courte en moyenne (expiration plus tôt)
- [ ] Vérifier que les params sont persistés en DB (rechargement de page → params conservés)
- [ ] Comparer manuellement un trade simulé Rust vs un trade MT5 Strategy Tester sur les mêmes données → les résultats doivent être proches (±5% d'écart acceptable)
- [ ] `cargo test --workspace` → zéro régression

---

## RÉSUMÉ DES PRIORITÉS

| Phase | Criticité | Effort estimé | Impact |
|---|---|---|---|
| Phase 1 — Sécurité capital | 🔴 Bloquant | 2-3 jours | Évite des pertes réelles par bug |
| Phase 2 — Fiabilité signaux | 🔴 Haute | 3-5 jours | Signaux cohérents et corrects |
| Phase 3 — Qualité IA | 🟠 Moyenne | 4-6 jours | ML utilisable en production |
| Phase 4 — Architecture | 🟡 Normale | 3-4 jours | Maintenabilité long terme |
| Phase 5 — Paramétrage MQ5 | 🟢 Fonctionnel | 3-4 semaines | Backtesting réaliste |

**Règle absolue** : ne jamais commencer une phase si la phase précédente n'a pas passé tous ses tests. Un signal de trading incorrect sur du capital réel est irréversible.

---

*Document généré le 27 mars 2026 — à mettre à jour après chaque phase complétée.*
