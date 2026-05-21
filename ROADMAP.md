# ROADMAP — Native Trading AI
# Feuille de route unifiée : corrections critiques + améliorations + SMC V2 + backtesting

> **Dernière mise à jour** : 21 mai 2026  
> **Intègre** : Analyse critique générale + ROADMAP_SMC_V2.md (désormais archivé)  
> **Philosophie** : chaque étape est un prérequis de la suivante. Ne pas sauter d'étape.

---

## Règles Incontournables — Valables pour Chaque Étape

Ces 5 règles s'appliquent **sans exception** à chaque modification apportée dans le cadre de cette roadmap. Elles priment sur toute considération de vitesse ou de praticité.

**Règle 1 — Valeur ajoutée prouvée**  
Chaque modification doit améliorer concrètement l'app et ses objectifs (fiabilité des signaux, qualité du risk management, précision ML, performance). Aucune fonction non réclamée ne sera ajoutée. Le doute bénéficie toujours à l'abstention.

**Règle 2 — Analyse de l'existant obligatoire**  
Avant tout code : lire et comprendre le code existant concerné. Identifier les redondances, les conflits potentiels et les doublons. Si une implémentation équivalente existe déjà, l'utiliser ou la corriger plutôt que d'en créer une nouvelle.

**Règle 3 — Zéro casse du code**  
Après chaque modification : `cargo build --workspace` et `npm run build` doivent passer sans erreur. Une modification qui casse la compilation est annulée immédiatement, sans exception.

**Règle 4 — Zéro régression**  
Après chaque modification : tous les tests existants passent. `cargo test --workspace` zéro échec. Si un test échoue après la modification, la modification est soit corrigée soit la modification est annulée — jamais le test.

**Règle 5 — Validation avant progression**  
On ne passe à l'étape N+1 que lorsque l'étape N est testée (tests auto + tests manuels listés) et validée par le développeur. Aucun travail parallèle sur deux étapes simultanément.

> **Rappel gate de validation** : chaque étape se termine par sa section **"🔒 Gate de Validation"** qui liste les conditions exactes à satisfaire avant de passer à la suivante.

---

## Objectif rappelé

L'application est un **assistant IA local de trading** destiné à un trader humain solo.  
Elle détecte les opportunités 24h/24 sur crypto (Binance), forex et métaux (IG Markets) via 3 stratégies autonomes, enrichit chaque signal d'une analyse LLM, suit les positions ouvertes et améliore ses modèles en continu par feedback loop.  
**Le trader prend la décision finale.** L'application ne passe pas d'ordres automatiquement.

---

## PHASE 0 — Corrections Critiques Bloquantes
> Ces bugs compromettent directement la fiabilité des signaux ou la sécurité du capital.  
> **À traiter avant toute amélioration.**

---

### 0.1 — Brancher le GestionnaireRisque dans les 3 boucles de génération

**Problème actuel**  
`GestionnaireRisque` existe dans le crate `risk` avec toutes ses règles (2% max par trade, 3 positions max, 25% par asset, 20% drawdown). Il est testé unitairement. Mais il n'est **jamais appelé** dans `straddle_boucle.rs`, `smc_boucle.rs`, ou `rockets_scan.rs`. Le risk management réel repose uniquement sur des seuils ad hoc dans chaque boucle, sans vérification centralisée du drawdown ou du nombre de positions agrégées.

**Conséquences du changement**  
- Impacte les 3 fichiers de boucle + `AppState` (le gestionnaire doit être partagé en `Arc<Mutex<GestionnaireRisque>>`)
- Nécessite une mise à jour en temps réel du drawdown : brancher sur les callbacks de verdict (TP/SL) pour `mettre_a_jour_drawdown()`
- Nécessite `ouvrir_position()` / `fermer_position()` sur chaque signal inséré / chaque verdict clôturé
- Risque de conflit : le gestionnaire est stateful — si le serveur redémarre, l'état est perdu. Solution : reconstruire l'état depuis la DB au démarrage (positions ouvertes + drawdown courant)

**Tests automatisés**  
- Rust : test unitaire — générer 4 signaux simultanés → le 4ème doit être refusé
- Rust : test unitaire — simuler drawdown 20% → tout signal suivant refusé
- Rust : test unitaire — reconstruction état depuis DB au démarrage

**Tests manuels**  
- Ouvrir manuellement 3 positions en DB, vérifier que le 4ème signal automatique est loggué `Signal refusé: 3 positions ouvertes`
- Vérifier l'indicateur "positions ouvertes" dans le DashboardSystemStatus

---

### 0.2 — Corriger le bug `volume_seche` dans Rockets (condition morte)

**Problème actuel**  
Dans `rockets_indicateurs.rs` fonction `calculer_phase` :
```rust
if volume_seche < 0.75 { s += 15; }
else if volume_seche < 0.55 { s += 5; }  // BUG : jamais atteint
```
La deuxième condition est logiquement impossible car `< 0.55` implique déjà `< 0.75`. Le bonus d'assèchement fort de +5 pts n'est **jamais accordé**.

**Conséquences du changement**  
- Modification mineure d'un seul fichier
- Les scores de compression seront légèrement plus élevés pour les assets avec très fort assèchement de volume (< 0.55)
- Possible déclenchement de signaux supplémentaires sur des setups VCP très propres

**Correction attendue**  
```rust
if volume_seche < 0.55 { s += 20; }      // assèchement fort : 15 + 5 bonus
else if volume_seche < 0.75 { s += 15; } // assèchement normal
```

**Tests automatisés**  
- Rust : test unitaire `calculer_phase` avec `volume_seche = 0.50` → vérifier score bonus = 20 pts
- Rust : test unitaire avec `volume_seche = 0.70` → vérifier score = 15 pts
- Rust : test unitaire avec `volume_seche = 0.80` → vérifier score = 0 pts

**Tests manuels**  
- Observer dans les logs `Scan rockets` si des setups VCP avec assèchement < 0.55 apparaissent maintenant en phase `prelancement`

---

### 0.3 — Normaliser le score LLM pour Straddle et SMC

**Problème actuel**  
Dans `straddle_signal_ollama.rs` et `ollama_signal_ia_handler.rs` :
```rust
score: brut.score_confiance * 10.0
```
Si le LLM retourne un score sur 0-1 → signal.score = 0-10 (au lieu de 0-100).  
Si le LLM retourne un score sur 0-10 → signal.score = 0-100.  
Le format de sortie JSON du LLM (Qwen2.5:14b ou autre) n'est pas contraint de manière fiable.

**Conséquences du changement**  
- Modifier les prompts pour forcer `score_confiance` sur 0.0-1.0 avec exemple JSON dans le prompt
- Ajouter une validation Rust post-parse : `if brut.score_confiance > 1.0 { brut.score_confiance /= 10.0; }`
- Impacte les filtres qui comparent `signal.score` à un seuil (ex: dashboard, calibration)

**Tests automatisés**  
- Rust : test unitaire — passer un JSON avec `score_confiance: 7.5` → vérifier normalisation → `score: 75.0`
- Rust : test unitaire — passer un JSON avec `score_confiance: 0.75` → vérifier `score: 75.0`

**Tests manuels**  
- Déclencher manuellement une analyse SMC → vérifier que `signal.score` dans la DB est dans [0, 100]
- Vérifier la colonne "Score" dans `SignauxTableau.vue` pour les nouveaux signaux

---

### 0.4 — Unifier les SL/TP Straddle entre strategy Rust et boucle automatique

**Problème actuel**  
La boucle automatique (`straddle_signal_ollama.rs`) hardcode les niveaux :
```rust
let sl_long = prix - 0.5 * atr;
let tps_long = vec![prix + 2.0 * atr, prix + 3.5 * atr, prix + 5.0 * atr];
```
La `StraddleStrategy` dans le crate `strategies` utilise `params.sl_mult`, `params.tp_mult_1/2/3` chargés depuis la DB. Ces deux implémentations peuvent produire des SL/TP différents pour le même asset.

**Conséquences du changement**  
- `straddle_signal_ollama.rs` doit recevoir les `StraddleParams` depuis la DB et les utiliser
- Les paramètres sont déjà chargés dans `straddle_boucle.rs` (ligne `let atr_ui = p.atr_seuil`) — étendre ce chargement aux multiplicateurs
- Impact sur le moniteur de position : les coefficients de trailing sont aussi dépendants des params

**Tests automatisés**  
- Rust : test d'intégration — modifier `sl_mult` à 0.8 en DB, déclencher un signal Straddle simulé, vérifier que `SL = prix - 0.8 × ATR`

**Tests manuels**  
- Changer `sl_mult` dans les paramètres Straddle via l'UI `StrategiesParamsPanel`, déclencher une analyse, vérifier les niveaux dans le tableau de signaux

---

### 0.5 — Directionnaliser le Liquidity Sweep dans le scoring SMC

**Problème actuel**  
Dans `smc_directional.rs` et `smc/src/lib.rs`, `sweep::detecter_sweep()` est utilisé comme booléen (`is_some()`). Un sweep vers le bas (piège baissier) précède un signal LONG. Un sweep vers le haut (piège haussier) précède un signal SHORT. Si la direction du sweep n'est pas réconciliée avec la direction du signal, on peut valider un signal LONG après un sweep haussier (signal contre-confluence).

**Conséquences du changement**  
- `sweep::detecter_sweep()` doit retourner `Option<Direction>` (ou une struct avec direction)
- Modifier `smc/src/sweep.rs` pour exposer la direction du sweep
- Modifier `smc/src/lib.rs` (`scorer`) pour vérifier que la direction du sweep est cohérente
- Modifier `smc_directional.rs` en conséquence
- Impact : réduction possible du nombre de signaux SMC (gate supplémentaire)

**Tests automatisés**  
- Rust : test unitaire `detecter_sweep` avec série de bougies simulant un sweep vers le haut → vérifier `Direction::Short`
- Rust : test unitaire `scorer` — sweep Long + direction Long = score positif, sweep Short + direction Long = None

**Tests manuels**  
- Surveiller les logs SMC sur 1 cycle — vérifier que les messages `sweep_detecte: true` correspondent bien à la direction affichée

---

### 🔒 Gate de Validation — Phase 0

Ces conditions doivent **toutes** être vraies avant de passer à la Phase 1 :

- [ ] `cargo test --workspace` : 0 échec, 0 erreur
- [ ] `cargo build --workspace --release` : compilation propre
- [ ] Test manuel 0.1 : un 4ème signal est bien refusé avec log `Signal refusé: 3 positions ouvertes`
- [ ] Test manuel 0.2 : score Rockets sur asset VCP avec `volume_seche < 0.55` = 20 pts bonus (vérifiable dans les logs)
- [ ] Test manuel 0.3 : `signal.score` dans la DB pour un nouveau signal Straddle ∈ [0, 100]
- [ ] Test manuel 0.4 : modifier `sl_mult` en DB → le SL du signal généré reflète la modification
- [ ] Test manuel 0.5 : aucun log `sweep_detecte: true` pour un signal dont la direction est opposée au sweep
- [ ] Aucune régression visuelle dans l'UI (dashboard, tableaux de signaux)

---

## PHASE 1 — Nettoyage et Architecture
> Réduire la dette technique. Prérequis pour les phases suivantes.

---

### 1.1 — Supprimer les indicateurs dupliqués dans Rockets

**Problème actuel**  
`rockets_indicateurs.rs` contient ses propres `calc_atr`, `calc_rsi`, `calc_ema` — copies locales des fonctions du crate `indicators`. Deux implémentations de l'ATR Wilder coexistent, risque de divergence silencieuse.

**Conséquences du changement**  
- Remplacer les appels locaux par `indicators::calculer_atr`, `indicators::calculer_rsi`, `indicators::calculer_ema`
- Vérifier que la signature des fonctions est compatible (les fonctions `indicators` travaillent sur `&[Candle]`, les versions locales sur `&[f64]`)
- Si incompatibilité de signature : adapter l'appelant, pas créer un troisième doublon
- Supprimer les fonctions locales après migration

**Tests automatisés**  
- Rust : comparer la sortie des deux implémentations ATR sur le même dataset historique → delta < 0.0001
- Rust : vérifier que les scores de phase Rockets sont stables avant/après migration

**Tests manuels**  
- Lancer un scan Rockets complet, comparer les scores avant/après pour les 5 premiers résultats

---

### 1.2 — Supprimer les fichiers `.vue.js` dupliqués du frontend

**Problème actuel**  
Chaque composant et vue Vue.js a un fichier `.vue.js` correspondant dans le VCS (ex: `DashboardHome.vue.js`). Ce sont des artefacts de compilation/transpilation qui ne doivent pas être versionnés.

**Conséquences du changement**  
- Aucun impact sur le fonctionnement
- Ajouter `*.vue.js` dans `.gitignore`
- Supprimer tous les fichiers concernés du suivi git

**Tests automatisés**  
- CI : vérifier que `git ls-files | grep '\.vue\.js'` retourne vide

**Tests manuels**  
- Rebuilder le frontend, vérifier que l'app s'ouvre normalement dans Tauri

---

### 1.3 — Retirer le module Vision / html2canvas de l'analyse SMC
*(Intégration ROADMAP_SMC_V2 — Étape 1)*

**Problème actuel**  
L'analyse SMC dispose d'un ancien chemin Vision (html2canvas + modèles Qwen-VL, Llama-Vision). Ce chemin crée des doublons de routes, de boutons et de logique dans `ollama_chart_handler.rs` et dans le frontend.

**Conséquences du changement**  
- Frontend : supprimer le dropdown de sélection de modèles Vision dans `ChartSignauxPanel.vue`
- Frontend : le bouton "Analyse SMC" appelle uniquement la nouvelle route text-first (Phase 2.1)
- Backend : `ollama_chart_handler.rs` conserve uniquement la logique de sauvegarde d'archive visuelle (html2canvas pour screenshot), supprimer la logique décisionnelle Vision
- Supprimer la route `/api/ia/analyse-vision` (ou la désactiver et la marquer deprecated)

**Tests automatisés**  
- Rust : vérifier que la route `/api/ia/analyse-vision` retourne 410 Gone ou est supprimée des routes
- Vue : vérifier que le composant `ChartSignauxPanel` ne contient plus de référence aux modèles Vision

**Tests manuels**  
- Cliquer sur "Analyse SMC" → vérifier qu'il n'y a plus de dropdown de modèle
- Vérifier que le bouton d'archive visuelle (screenshot) fonctionne toujours

---

### 1.4 — Supprimer le module export fiscal

**Problème actuel**  
`export_handlers.rs` implémente une fonctionnalité d'export non requise.

**Conséquences du changement**  
- Supprimer `export_handlers.rs` et ses routes dans `routes.rs`
- Supprimer le composant frontend lié si existant

**Tests automatisés**  
- Rust : `cargo build --workspace` passe sans erreur

**Tests manuels**  
- Navigation dans l'UI : aucun lien mort, aucune erreur 404

---

### 1.5 — Vérification de couverture WebSocket signaux

**Problème actuel**  
Les 3 stratégies appellent `signal_engine.publier()` pour broadcaster les signaux via WebSocket. Vérifier exhaustivement que 100% des chemins de génération (auto + manuel via API) passent par ce canal unifié.

**Conséquences du changement**  
- Audit des handlers HTTP qui génèrent des signaux manuellement (ex: `/api/ia/signal`, `/api/smc/signal`)
- S'assurer que tous appellent `state.signal_engine.publier(signal.clone())` après insertion DB

**Tests automatisés**  
- Rust : test d'intégration — déclencher un signal via chaque endpoint manuel, vérifier qu'un message WS est reçu côté subscriber

**Tests manuels**  
- Ouvrir l'UI, déclencher une analyse manuelle SMC → vérifier que la modale d'alerte signal apparaît en temps réel

---

### 🔒 Gate de Validation — Phase 1

Ces conditions doivent **toutes** être vraies avant de passer à la Phase 2 :

- [ ] `cargo test --workspace` : 0 échec
- [ ] `cargo clippy --workspace -- -D warnings` : 0 warning
- [ ] `npm run build` : 0 erreur TypeScript
- [ ] Test manuel 1.1 : un scan Rockets complet produit des scores identiques avant/après migration des indicateurs
- [ ] Test manuel 1.2 : `git ls-files | grep '\.vue\.js'` retourne vide
- [ ] Test manuel 1.3 : le bouton "Analyse SMC" ne propose plus de dropdown modèle Vision ; la capture screenshot fonctionne toujours
- [ ] Test manuel 1.4 : aucune route `/api/export` dans les routes actives
- [ ] Test manuel 1.5 : déclencher un signal via l'API manuelle → notification WS reçue dans l'UI

---

## PHASE 2 — Refonte SMC V2 (Text-First Architecture)
*(Intégration complète ROADMAP_SMC_V2)*

> **Objectif** : Remplacer l'analyse Vision par un pipeline algorithmique Rust + LLM de raisonnement (DeepSeek-R1-32B). Élimine les hallucinations visuelles, réduit la consommation VRAM, augmente la précision.  
> **Prérequis** : Phase 1.3 terminée.

---

### 2.1 — Enrichir le SMC Engine Rust (BOS, CHoCH, Liquidités asiatiques)
*(ROADMAP_SMC_V2 — Étape 2)*

**Problème actuel**  
Le crate `smc` implémente tendances, OB, IFVG, Imbalance, Fibonacci, Kill Zone, Sweep, Liquidités de base. Il manque :
- **BOS** (Break of Structure) : cassure d'un swing high/low sans retournement
- **CHoCH** (Change of Character) : premier retournement de structure — signal de changement de tendance
- Calcul explicite de l'**Asian High/Low** (range de session asiatique comme zone de liquidité)
- Amélioration de la **détection des Order Blocks** : état mitigated/unmitigated, force relative

**Conséquences du changement**  
- Nouveaux modules dans `smc/src/bos.rs`, `smc/src/choch.rs` (ou extension de `tendances.rs`)
- Extension du `ScoreSmc` avec les nouveaux champs
- Mise à jour du `scorer()` dans `smc/src/lib.rs`
- Mise à jour des prompts LLM qui reçoivent le JSON SMC (ils doivent intégrer BOS/CHoCH)
- Impact sur le scoring : possible hausse des scores → vérifier que le seuil 70/100 reste pertinent

**Tests automatisés**  
- Rust : test `detecter_bos` sur série simulant une cassure de swing high → résultat BOS Long
- Rust : test `detecter_choch` sur série avec retournement → résultat CHoCH Short
- Rust : test de régression `scorer()` — s'assurer que les scores existants ne régressent pas > 5 pts

**Tests manuels**  
- Vue SMC : afficher un asset en tendance haussière établie → vérifier que BOS Long apparaît dans les indicateurs
- Vue SMC : afficher un asset en retournement → vérifier détection CHoCH

---

### 2.2 — Améliorer la granularité du scoring SMC (éliminer les sauts discrets)

**Problème actuel**  
Le scoring IFVG et Imbalance utilise des sauts discrets (0 / 10 / 20 pts, 0 / 8 / 15 pts) qui créent des faux positifs de seuil : un asset peut atteindre 70/100 sans Order Block ni Fibonacci uniquement avec plusieurs IFVG et Imbalances. Ce n'est pas de la confluence — c'est du bruit agrégé.

**Conséquences du changement**  
- Ajouter une pondération continue ou par force de zone (distance au prix, taille de la zone, âge)
- Optionnel : introduire une règle de diversité (au moins 3 composantes > 0 pour scorer >= 70)
- Impact : réduction probable du nombre de signaux SMC → amélioration du win rate

**Tests automatisés**  
- Rust : test de régression sur dataset historique — compter les signaux avant/après et comparer le win rate sur les trades clôturés

**Tests manuels**  
- Observer 1 semaine de signaux SMC : les signaux à score > 70 devraient montrer des confluences visuellement évidentes sur le graphique

---

### 2.3 — Intégrer DeepSeek-R1-32B comme LLM de raisonnement SMC
*(ROADMAP_SMC_V2 — Étapes 3 et 4)*

**Problème actuel**  
Qwen2.5:14b est le modèle par défaut pour tous les contextes (Straddle, SMC, chart, chat). Pour l'analyse SMC structurée (confluences algorithmiques), un modèle de **raisonnement** (chain-of-thought) comme DeepSeek-R1-32B est plus adapté. Il vérifie des confluences logiques plutôt que de "deviner" visuellement.

**Architecture cible**  
- SMC : DeepSeek-R1-32B (raisonnement logique sur JSON SMC)
- Straddle : Qwen2.5:14b (classification probabiliste de contexte macro)
- Chat/traduction : Qwen2.5:14b (vitesse importante)

**Conséquences du changement**  
- Ajouter une variable `OLLAMA_MODEL_SMC` (défaut `deepseek-r1:32b`) distincte de `OLLAMA_MODEL`
- `smc_signal_ollama.rs` et `ollama_signal_ia_handler.rs` utilisent `OLLAMA_MODEL_SMC`
- Créer un system prompt "Auditeur SMC" strict : vérification de confluences, JSON validé, pas de spéculation
- Vérifier la cohabitation VRAM : DeepSeek-R1-32B (~20 Go) + Qwen2.5:14b (~9 Go) = ~29 Go > 24 Go RTX 3090 → **impossible simultanément**. Solution : utiliser DeepSeek-R1-32B seul, ou basculer entre modèles selon la stratégie (déchargement du modèle précédent)

**Tests automatisés**  
- Rust : test que le JSON retourné par le prompt SMC est parsable et complet (direction, tp1, tp2, tp3, sl, confluences)
- Rust : test edge case — LLM retourne "Neutre" → signal non créé

**Tests manuels**  
- Vérifier consommation VRAM avec `nvidia-smi` lors d'un cycle SMC avec DeepSeek-R1-32B
- Comparer la qualité du raisonnement affiché dans `llm_raison` pour 5 signaux avec Qwen vs DeepSeek

---

### 🔒 Gate de Validation — Phase 2

Ces conditions doivent **toutes** être vraies avant de passer à la Phase 3 :

- [ ] `cargo test --workspace` : 0 échec
- [ ] Test manuel 2.1 : BOS et CHoCH apparaissent dans les logs SMC sur un asset en tendance établie
- [ ] Test manuel 2.1 : régression scoring — les scores existants ne varient pas de plus de 5 pts sur le même dataset
- [ ] Test manuel 2.2 : aucun signal SMC > 70 avec moins de 3 composantes actives (règle de diversité)
- [ ] Test manuel 2.3 : `nvidia-smi` confirme VRAM ≤ 24 Go pendant un cycle SMC avec DeepSeek-R1-32B
- [ ] Test manuel 2.3 : JSON retourné par DeepSeek parsé sans erreur sur 10 analyses consécutives
- [ ] Aucune régression sur les stratégies Straddle et Rockets (non touchées par cette phase)

---

## PHASE 3 — Améliorations ML et Exploitation GPU

> **Prérequis** : Phase 0 (corrections critiques) terminée.

---

### 3.1 — Migrer le LSTM vers tch-rs (PyTorch natif)

**Problème actuel**  
Le LSTM est implémenté from scratch en Rust pur avec BPTT manuel. `tch-rs` (binding PyTorch) est déjà déclaré dans `Cargo.toml` mais inutilisé pour le LSTM principal. Le LSTM maison est :
- Non optimisé pour GPU pendant l'entraînement (CPU uniquement)
- Limité à 10 timesteps de séquence
- Instable (divergence détectable à chaque entraînement)
- ~500 lignes de code de bas niveau remplaçables par ~50 lignes tch-rs

**Architecture cible**  
LSTM 3 couches (128→64→32) implémenté avec `tch::nn::LSTM` sur GPU CUDA, entraînement par batch, séquences de 60 timesteps (matching la fenêtre features).

**Conséquences du changement**  
- `ml/src/lstm/` entièrement remplacé
- Les poids sont désormais des tenseurs PyTorch → format de sauvegarde change (`.pt` au lieu de `.json`)
- `modele_lstm.json` et `modele_lstm_backup.json` deviennent obsolètes
- Le chemin de chargement `CHEMIN_LSTM` change
- `PipelineML::charger_depuis_disque()` doit gérer la migration de format
- La feature `cuda` dans `pipeline.rs` doit activer tch-rs plutôt que le GPU maison

**Tests automatisés**  
- Rust : test d'inférence — même input → même direction de prédiction avec LSTM tch-rs vs ancien LSTM (les valeurs exactes diffèrent mais la direction doit être cohérente)
- Rust : test de performance — inférence < 200ms sur 60 timesteps × 52 features (GPU)

**Tests manuels**  
- Lancer un cycle d'entraînement complet, vérifier `accuracy_val` dans `RetainState`
- `nvidia-smi dmon` : vérifier que la GPU est utilisée pendant l'entraînement (utilisation > 80%)

---

### 3.2 — Paralléliser les scans multi-assets sur les 20 cœurs

**Problème actuel**  
Les boucles Straddle (`straddle_boucle.rs`) et SMC (`smc_boucle.rs`) analysent les assets séquentiellement dans une boucle `for`. Sur 20+ assets × 2 timeframes, chaque cycle est inutilement lent. Les 20 cœurs du i9 ne sont pas exploités.

**Conséquences du changement**  
- Utiliser `futures_util::future::join_all()` (pattern déjà utilisé dans `rockets_scan.rs`) pour paralléliser les analyses SMC/Straddle
- Attention aux contentions : `PipelineML` est derrière `Arc<Mutex>` — avec la parallélisation, plusieurs analyses voudraient acquérir le lock simultanément. Solution : utiliser `Arc<RwLock<PipelineML>>` pour les lectures d'inférence
- Attention aux anti-doublons : la vérification DB anti-doublon avant signal doit rester atomique

**Tests automatisés**  
- Rust : test de performance — mesurer la durée d'un cycle complet avant/après avec 20 assets simulés
- Rust : test de concurrence — vérifier qu'aucun deadlock ne se produit avec 10 assets en parallèle

**Tests manuels**  
- Mesurer le temps d'un cycle SMC dans les logs (`Boucle SMC cycle terminé`) avant/après

---

### 3.3 — Adapter les labels ML par stratégie

**Problème actuel**  
Les 3 modèles fine-tunés (xgb_rockets, xgb_straddle, xgb_smc) utilisent tous le même label binaire "hausse dans N bougies". Ce label est inadapté :
- **Straddle** : devrait labelliser l'**amplitude max de mouvement** dans un horizon (pas la direction)
- **Rockets** : devrait labelliser la **probabilité de breakout de X%** dans un horizon
- **SMC** : devrait labelliser selon la **tenue jusqu'au TP1** (signal résolu positivement)

**Conséquences du changement**  
- Modifier les fichiers `straddle_trainer.rs`, `rockets_trainer.rs`, `smc_trainer.rs` dans `ml/src/`
- Créer des fonctions de labellisation spécialisées dans `ml/src/features.rs`
- Réentraîner les 3 modèles depuis zéro (les modèles actuels avec les mauvais labels sont à jeter)
- Impact sur les gates ML dans les boucles : les scores de confiance auront une sémantique différente

**Tests automatisés**  
- Rust : test que le label Straddle pour une bougie avec mouvement de +5% ATR = 1.0
- Rust : test que le label Rockets pour un breakout de +8% en 5 bougies = 1.0
- Validation de l'accuracy sur un dataset de test holdout pour chaque modèle (seuil minimum : 55%)

**Tests manuels**  
- Après réentraînement, vérifier les métriques dans `MlInsightsView.vue` (accuracy_val, wf_score)
- Observer 50 signaux : les gates ML devraient rejeter davantage de setups faibles

---

### 3.4 — Intégrer le Walk-Forward dans le scheduler de production

**Problème actuel**  
`walk_forward.rs` existe dans `ml/src/` mais n'est pas intégré dans le scheduler de réentraînement quotidien. L'entraînement se fait sans validation out-of-sample systématique, ce qui expose au surapprentissage.

**Conséquences du changement**  
- `scheduler.rs` doit appeler le walk-forward après chaque entraînement
- `RetainState` expose déjà `wf_score_apres` et `gap_train_wf` — les alimenter
- Règle de rollback : si `gap_train_wf > 15%` (gap train vs OOS), rollback automatique

**Tests automatisés**  
- Rust : test que le rollback est déclenché si gap > 15%
- Rust : test que `wf_score_apres` est peuplé après un cycle d'entraînement

**Tests manuels**  
- Déclencher un réentraînement via `POST /api/ml/retrain`, vérifier `wf_score_apres` dans le statut retourné

---

### 🔒 Gate de Validation — Phase 3

Ces conditions doivent **toutes** être vraies avant de passer à la Phase 4 :

- [ ] `cargo test --workspace` : 0 échec
- [ ] Test manuel 3.1 : `nvidia-smi dmon` montre utilisation GPU > 80% pendant l'entraînement LSTM
- [ ] Test manuel 3.1 : inférence LSTM mesurée < 100ms sur 60 timesteps × 52 features
- [ ] Test manuel 3.2 : durée d'un cycle SMC complet (20 assets) < 60s dans les logs
- [ ] Test manuel 3.3 : accuracy validation OOS > 55% après réentraînement avec nouveaux labels (affiché dans `MlInsightsView`)
- [ ] Test manuel 3.4 : `wf_score_apres` peuplé dans la réponse de `/api/ml/retrain`
- [ ] Aucune régression sur la qualité des signaux générés (comparer 50 signaux avant/après)

---

## PHASE 4 — Analyse Approfondie et Backtesting par Stratégie

> **Prérequis** : Phase 2 (SMC V2) et Phase 3.1 (LSTM GPU) terminées.  
> Ce crate n'existe pas encore dans le workspace — c'est un nouveau développement.

---

### 4.0 — Créer le crate `backtest` dans le workspace

**Problème actuel**  
Le crate `backtest` est mentionné dans l'architecture documentée mais **absent du workspace**. Aucun moteur de replay sur données historiques n'existe.

**Architecture cible**  
```
backend/crates/backtest/
  src/
    lib.rs          → types publics (BacktestConfig, BacktestResult, Trade)
    engine.rs       → moteur de replay bougie par bougie
    metriques.rs    → calcul Sharpe, drawdown, R:R, win rate, profit factor
    straddle.rs     → adapter Straddle pour le backtest
    smc.rs          → adapter SMC pour le backtest
    rockets.rs      → adapter Rockets pour le backtest
```

**Conséquences du changement**  
- Ajouter `backtest` dans `backend/Cargo.toml` workspace members
- Les adapters appèlent les strategies Rust existantes (straddle.rs, smc_directional.rs) → réutilisation maximale, zéro duplication
- L'API expose `/api/backtest/lancer` (POST avec config) et `/api/backtest/resultats` (GET)
- Nouveau handler dans `api/src/`

**Tests automatisés**  
- Rust : test unitaire `metriques::calculer_sharpe` sur dataset de trades simulés
- Rust : test d'intégration — replay de 30 jours de bougies BTC M15 → résultats déterministes et reproductibles

**Tests manuels**  
- Lancer un backtest Straddle sur XAUUSD 6 mois → vérifier cohérence des métriques avec les trades réels clôturés

---

### 4.1 — Backtest Straddle : logique, entrées, sorties, métriques

**Analyse approfondie de la stratégie**

**Logique de déclenchement**  
- Déclencheur correct : ATR > seuil × ATR_moyen_14. Le seuil par défaut (1.5) est raisonnable.
- Le LLM valide ou refuse le contexte. C'est la bonne architecture.
- Le filtrage par créneaux historiques (horaires récurrents de forte volatilité) est une vraie valeur ajoutée.

**Points faibles à valider en backtest**  
1. Le Straddle suppose que la volatilité continuera après le signal. En réalité, un spike ATR peut être le sommet de la volatilité (mean reversion). Mesurer le ratio "ATR augmente après signal" vs "ATR diminue".
2. SL = 0.5 × ATR pour les deux jambes. Sur XAUUSD avec ATR = 150 pips, SL = 75 pips. C'est serré sur un marché volatile → taux de jambes stoppées avant TP1 probablement élevé.
3. Les deux jambes ne sont pas indépendantes : si LONG touche TP1, le marché a monté → la jambe SHORT est souvent stoppée. Le backtest doit simuler ce comportement réaliste.

**Métriques spécifiques à mesurer**  
- Taux de "double win" (les deux jambes gagnantes)
- Taux de "une jambe TP, une jambe SL" (résultat quasi neutre)
- Taux de "double SL" (les deux jambes stoppées = perte 2×)
- P&L net par catégorie (`annonce_high`, `overlap_lnd_ny`, `choc_isole`)
- Impact du filtre calendrier : win rate avec vs sans annonce HIGH impact

**Conséquences du changement**  
- `backtest/src/straddle.rs` doit simuler les deux jambes simultanément
- Les spreads réalistes doivent être intégrés (XAUUSD spread ~30-50 pips en période volatile)
- Le backtest utilise `StraddleParams` depuis la DB (idem production)

**Tests automatisés**  
- Rust : backtest sur dataset synthétique avec double win attendu → vérifier P&L positif
- Rust : backtest sur dataset synthétique avec double SL → vérifier P&L = -2R

**Tests manuels**  
- Lancer un backtest sur XAUUSD 12 mois, vérifier que le win rate est >= 50% et le profit factor >= 1.2

---

### 4.2 — Backtest SMC Directionnel : logique, entrées, sorties, métriques

**Analyse approfondie de la stratégie**

**Logique de déclenchement**  
- Score >= 70/100 sur 5 composantes + Kill Zone + Liquidity Sweep (prérequis ICT). Logiquement solide.
- Problème identifié : le seuil 70/100 avec des sauts discrets IFVG/Imbalance peut être atteint par 3 composantes seulement. Une vraie confluence ICT requiert idéalement 4+ composantes actives.

**Points faibles à valider en backtest**  
1. Le Liquidity Sweep sans directionnalisation (Phase 0.5) génère potentiellement des signaux contre-confluence.
2. L'entrée au `close` actuel est rarement optimale en SMC. L'entrée idéale est au **retour sur l'Order Block** (après le sweep). Mesurer l'écart entre "entrée market actuel" et "entrée limitée sur OB".
3. La Kill Zone filtre les heures hors London/NY mais l'analyse SMC doit aussi tenir compte des sessions asiatiques (formation des ranges de liquidité).

**Métriques spécifiques à mesurer**  
- Win rate par composante dominante (trades gagnants quand OB actif vs sans OB)
- Win rate par categorie SMC (OB+FVG vs Fib seule, etc.)
- Distribution des sorties : TP1 / TP2 / TP3 / SL / BE (SL progressif)
- Impact du gate ML sur le win rate (trades filtrés par ML vs trades passés)

**Conséquences du changement**  
- `backtest/src/smc.rs` doit utiliser `smc::scorer()` à chaque bougie de replay
- Prendre en compte le délai de Kill Zone (ne pas entrer hors fenêtre)

**Tests automatisés**  
- Rust : backtest SMC sur BTCUSDT M15 30 jours → win rate > 45% (seuil minimal acceptable)

**Tests manuels**  
- Comparer les résultats du backtest avec les trades SMC réels clôturés en DB (cohérence cross-validation)

---

### 4.3 — Backtest Rockets VCP : logique, entrées, sorties, métriques

**Analyse approfondie de la stratégie**

**Logique de déclenchement**  
- Le VCP (Volatility Contraction Pattern) de Minervini est une approche validée empiriquement sur actions. Son adaptation aux cryptos est justifiée mais avec des réserves : les cryptos ont des cycles de compression beaucoup plus courts (heures vs semaines pour les actions).

**Points faibles à valider en backtest**  
1. La détection de phase `breakout` requiert `ratio_volume >= cfg.ratio_volume_min`. Sur les cryptos USDT à faible capitalisation, les volumes sont manipulables. Le filtre VCP professionnel inclut une vérification de l'open interest (non disponible via Binance REST seul).
2. L'entrée "type_entree_rec : limite ou stop" est calculée algorithmiquement mais jamais communiquée au trader de manière proactive. Le signal en DB stocke un prix d'entrée unique — l'information de type d'entrée est dans `ScanResultat` mais disparaît après.
3. Le trailing stop basé sur ATR × coefficient peut être trop large sur un crypto à forte volatilité : `score > 80, atr_ratio > 1.5` → coefficient 4.5 × ATR. Sur BTC à ATR = 500$, trailing = 2250$ de large → pas assez protecteur.

**Métriques spécifiques à mesurer**  
- Taux de signaux "invalides" (SL touché avant entrée, jamais ouverts)
- Distribution de temps entre signal et entrée réelle (efficacité du prix d'entrée limite)
- P&L moyen par phase (breakout vs prelancement vs compression)
- Impact de la gate ML Rockets sur le win rate

**Conséquences du changement**  
- `backtest/src/rockets.rs` doit simuler la phase d'attente (signal EN ATTENTE → OUVERT)
- Prendre en compte les spreads Binance (~0.1% sur altcoins)

**Tests automatisés**  
- Rust : backtest sur 10 assets crypto 3 mois → win rate breakout > 50%, win rate compression > 40%

**Tests manuels**  
- Comparer résultats backtest avec les Rockets réels clôturés en DB

---

### 🔒 Gate de Validation — Phase 4

Ces conditions doivent **toutes** être vraies avant de passer à la Phase 5 :

- [ ] `cargo test --workspace` : 0 échec
- [ ] Test manuel 4.0 : `cargo build --workspace` intègre le crate `backtest` sans erreur
- [ ] Test manuel 4.1 : backtest Straddle XAUUSD 6 mois → métriques cohérentes avec les trades réels clôturés en DB (écart < 5%)
- [ ] Test manuel 4.1 : simulation double SL produit P&L = -2R, double TP1 produit P&L = +2R
- [ ] Test manuel 4.2 : backtest SMC BTCUSDT M15 30 jours → win rate > 45%
- [ ] Test manuel 4.3 : backtest Rockets breakout → win rate > 50%
- [ ] Les résultats backtest sont affichés dans l'UI sans erreur de rendu

---

## PHASE 5 — Génération Automatique de Types TypeScript

> **Prérequis** : Phase 0 terminée (les types Rust sont stables).

---

### 5.1 — Générer les types TypeScript depuis Rust avec `ts-rs`

**Problème actuel**  
Les types TypeScript dans `frontend/src/services/api.types.ts` sont maintenus manuellement. Toute modification de `Signal`, `ScoreSmc`, `ScanResultat` en Rust n'est pas reflétée automatiquement. Les désynchronisations causent des bugs silencieux.

**Conséquences du changement**  
- Ajouter `ts-rs` en dépendance dans les crates `common`, `smc`, `strategies`
- Annoter les structs publiques avec `#[derive(TS)]` et `#[ts(export)]`
- Générer les fichiers `.ts` dans un dossier cible (`frontend/src/generated/`)
- Remplacer `api.types.ts` par les fichiers générés
- Intégrer la génération dans le script de build (`cargo test --workspace` génère les types)

**Tests automatisés**  
- CI : vérifier que les fichiers TypeScript générés sont à jour (git diff = 0 après `cargo test`)

**Tests manuels**  
- Modifier un champ de `Signal` en Rust, relancer le build → vérifier que l'erreur TypeScript apparaît immédiatement dans le frontend

---

### 🔒 Gate de Validation — Phase 5

Ces conditions doivent **toutes** être vraies avant de passer à la Phase 6 :

- [ ] `cargo test --workspace` : 0 échec
- [ ] `npm run build` : 0 erreur TypeScript sur les types générés
- [ ] `git ls-files frontend/src/generated/` liste les fichiers `.ts` générés
- [ ] Modifier un champ Rust → erreur TypeScript immédiate dans le frontend sans intervention manuelle
- [ ] Aucune duplication entre `api.types.ts` (supprimé ou vide) et les fichiers générés

---

## PHASE 6 — Suppression du Code Mort

> **Prérequis** : Toutes les phases précédentes terminées (pour s'assurer que rien n'est encore utilisé).

---

### 6.1 — Audit et suppression du code mort dans `api/src/`

**Éléments identifiés à auditer**  
- `test_ig_multi.rs` : fichier de test dans le crate de production → déplacer dans les tests ou supprimer
- `anthropic.rs` : si Anthropic (cloud) n'est pas utilisé → supprimer
- `SmcDirectionalStrategy::analyze()` : si la boucle auto n'appelle jamais cette méthode directement, documenter ou supprimer le chemin mort
- Les boucles SMC et Straddle dans `api/` : refactorer vers leurs crates respectifs (`strategies/src/`) pour respecter le DAG (Layer 4 → Layer 3)

**Conséquences du changement**  
- Déplacement de logique métier de `api/` vers `strategies/` → attention aux dépendances circulaires
- `strategies` ne peut pas importer `db` directement (violation DAG) → les fonctions qui lisent la DB restent dans `api/`, seule la logique pure migre

**Tests automatisés**  
- Rust : `cargo test --workspace` passe après chaque suppression
- Rust : `cargo clippy --workspace -- -D warnings` zéro warning

**Tests manuels**  
- Tester chaque stratégie après chaque migration de fichier

---

### 🔒 Gate de Validation — Phase 6 (Final)

Ces conditions marquent la **roadmap complète** comme terminée :

- [ ] `cargo test --workspace` : 0 échec
- [ ] `cargo clippy --workspace -- -D warnings` : 0 warning
- [ ] `npm run build` : 0 erreur
- [ ] Aucun fichier `test_*.rs` dans un crate de production
- [ ] `grep -r 'anthropic' backend/` retourne vide (si non utilisé)
- [ ] Aucune fonction publique non appelée dans `api/src/` (détectée par clippy dead_code)
- [ ] Revue humaine des 3 stratégies : les cycles auto tournent sans erreur pendant 24h

---

## Tableau de Priorités et Prérequis

| Phase | Étape | Priorité | Prérequis | Impact Signaux |
|-------|-------|----------|-----------|----------------|
| 0 | 0.1 Risk Manager | 🔴 Critique | Aucun | Sécurité capital |
| 0 | 0.2 Bug volume_seche | 🔴 Critique | Aucun | Qualité Rockets |
| 0 | 0.3 Score LLM | 🔴 Critique | Aucun | Fiabilité scores |
| 0 | 0.4 SL Straddle | 🔴 Critique | Aucun | Cohérence niveaux |
| 0 | 0.5 Sweep direction | 🔴 Critique | Aucun | Précision SMC |
| 1 | 1.1 Indicateurs dupliqués | 🟠 Important | Phase 0 | Fiabilité calculs |
| 1 | 1.2 Fichiers .vue.js | 🟡 Mineur | Aucun | Aucun |
| 1 | 1.3 Retrait Vision | 🟠 Important | Aucun | Architecture |
| 1 | 1.4 Suppression export | 🟡 Mineur | Aucun | Aucun |
| 1 | 1.5 Couverture WS | 🟠 Important | Phase 0 | Temps réel |
| 2 | 2.1 BOS/CHoCH | 🟠 Important | 1.3 | Précision SMC |
| 2 | 2.2 Scoring granulaire | 🟠 Important | 2.1 | Précision SMC |
| 2 | 2.3 DeepSeek-R1-32B | 🟠 Important | 1.3, 2.1 | Qualité raisonnement |
| 3 | 3.1 LSTM tch-rs GPU | 🟠 Important | Phase 0 | Précision ML |
| 3 | 3.2 Parallélisation | 🟡 Mineur | 3.1 | Performance |
| 3 | 3.3 Labels adaptés | 🟠 Important | 3.1 | Précision ML |
| 3 | 3.4 Walk-Forward | 🟡 Mineur | 3.3 | Robustesse ML |
| 4 | 4.0 Crate backtest | 🟠 Important | Phase 2, 3.1 | Validation |
| 4 | 4.1 Backtest Straddle | 🟠 Important | 4.0 | Validation |
| 4 | 4.2 Backtest SMC | 🟠 Important | 4.0 | Validation |
| 4 | 4.3 Backtest Rockets | 🟠 Important | 4.0 | Validation |
| 5 | 5.1 Types TypeScript | 🟡 Mineur | Phase 0 | Stabilité |
| 6 | 6.1 Code mort | 🟡 Mineur | Toutes | Maintenabilité |

---

## Métriques de Succès Globales

À l'issue de toutes les phases, le système doit atteindre :

| Métrique | Actuel (estimé) | Cible |
|----------|----------------|-------|
| Win rate Straddle | inconnu | ≥ 55% |
| Win rate SMC | inconnu | ≥ 55% |
| Win rate Rockets | inconnu | ≥ 50% |
| Profit Factor global | inconnu | ≥ 1.5 |
| Latence inférence ML | < 200ms | < 100ms (GPU) |
| Latence cycle SMC | ~900s (15 assets séq.) | < 60s (parallèle) |
| Accuracy ML (val OOS) | ~ 52% | ≥ 58% |
| Utilisation GPU (entraînement) | ~ 0% | > 80% |
| Drawdown max réel | non mesuré | < 20% |
| Sharpe ratio backtesté | non calculé | > 1.5 |
