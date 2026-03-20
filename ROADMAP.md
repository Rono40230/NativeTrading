# 🗺️ ROADMAP - NATIVE TRADING AI

**Durée totale:** 18 semaines | **Version:** 3.1 | **Dernière mise à jour:** 20 mars 2026

---

## ✅ PHASE 1 - MVP — TERMINÉE (Semaines 1-6)

> **Statut : 🟢 COMPLÉTÉE** — App native validée le 10 mars 2026

### ✅ Semaine 1-2: Fondations
- [x] Setup environnement (Rust, Node.js 22, Tauri CLI)
- [x] Structure projet — workspace 10 crates Rust + Vue.js 3 Tauri
- [x] App native Tauri — aucun navigateur, fenêtre GDK_BACKEND=x11
- [x] Acquisition données Binance REST (BTC/ETH OHLCV)
- [x] SQLite via SQLx avec migrations (4 tables : bougies, signaux, etc.)

### ✅ Semaine 3: Détection volatilité + ML
- [x] Indicateurs : EMA, ATR, RSI, MACD, Bollinger (crate `indicators`)
- [x] Feature engineering 52 features (`ml/src/features.rs`)
- [x] Modèle RandomForest via smartcore 0.4 (`ml/src/modele.rs`)
- [x] Pipeline ML : `PipelineML` avec inférence + entraînement
- [x] `tendance.rs` dans `indicators`

### ✅ Semaine 4: Stratégie + Backtesting
- [x] Stratégie Straddle complète (`strategies/src/straddle.rs`)
- [x] Backtest engine walk-forward avec métriques complètes (`backtest/src/lib.rs`)
  - ROI, Sharpe, Max Drawdown, Win Rate, Profit Factor
  - Friction réaliste 0.03%, risk 2%/trade
- [x] Risk management : `GestionnaireRisque::valider_signal()` + 4 limites

### ✅ Semaine 5-6: Dashboard minimal (natif Tauri)
- [x] Vue.js 3 + Pinia + Tailwind + TradingView Lightweight Charts
- [x] Dashboard : prix temps réel, statut système, tableau signaux
- [x] ChartsView : graphique chandeliers sélecteur asset/timeframe
- [x] Pinia stores : `market.store.ts`, `signal.store.ts`
- [x] `api.service.ts` Axios vers backend port 8080
- [x] Script `run.sh` : compile -> health check -> Tauri natif
- [x] 19 tests unitaires (backtest×2, indicators×4, ml×3, risk×5, smc×2, strategies×3)

**Métriques MVP validées (live 10 mars 2026) :**
- BTC/USDT $71,240 | ETH/USDT $2,069 (Binance temps réel)
- Backend API Online | Binance Feed Connecté | SQLite Online

---

## ✅ PHASE 2 - FONCTIONNALITÉS CORE — TERMINÉE (Semaines 7-16)

> **Statut : 🟢 COMPLÉTÉE** — Terminée entre le 10 et le 14 mars 2026

### ✅ Semaine 7-8: IB Gateway + 13 Assets
> Note : Twelvedata initialement prévu, remplacé par IB Gateway (ibapi 2.10.0) — plus robuste, sans limite API
- [x] `IbGatewayProvider` — crate `data`, trait `DataProvider`
- [x] 13 assets : BTC, ETH, XAUUSD, XAGUSD, EURUSD, GBPJPY, CADJPY, NZDJPY, USDCAD, USDJPY, DAX, NAS100, SP500
- [x] Forex -> SecurityType::ForexPair / IDEALPRO
- [x] Métaux -> SecurityType::Commodity / SMART
- [x] Indices : DAX (Index/EUREX), SPX (Index/CBOE), NQ/NAS100 (ContinuousFuture/CME)
- [x] WhatToShow::Trades pour indices, MidPoint pour le reste
- [x] Asset enum 13 variants, `parse_asset()` mis à jour
- [x] `GET /api/assets` — liste dynamique des assets
- [x] Dashboard : bande de prix 13 actifs + auto-refresh toutes les 30s

### ✅ Semaine 9-10: Indicateurs SMC
- [x] Tendances HH/HL/LH/LL (`smc/src/tendances.rs`)
- [x] Order Blocks — last candle avant impulsion (`smc/src/order_blocks.rs`)
- [x] Imbalance / FVG 3-candles (`smc/src/imbalance.rs`)
- [x] IFVG — FVG + mitigation + BOS (`smc/src/ifvg.rs`)
- [x] Fibonacci — niveaux 23.6/38.2/50/61.8/78.6 (`smc/src/fibonacci.rs`)
- [x] Facade `ScoreSmc` — scoring 100pts, seuil 70 (`smc/src/lib.rs`)
- [x] Tests : tendances + fibonacci

### ✅ Semaine 11-12: IA Hybride (ML pure Rust)
> Note : XGBoost non implémenté — remplacé par LSTM pure Rust (à corriger en Phase 3)
- [x] LSTM 3 couches (128-64-32) pure Rust — `ml/src/lstm.rs`
- [x] Fusion LSTM 60% + RandomForest 40% — `PipelineHybride`
- [x] Entraînement SGD + sérialisation serde_json
- [x] `POST /api/ml/train` + `GET /api/ml/status`
- [x] Bouton Entraîner RF + LSTM dans ChartsView

### ✅ Semaine 13-14: SMC Directionnel
- [x] `SmcDirectionalStrategy` — scoring confluence >= 70 (`strategies/src/smc_directional.rs`)
- [x] ATR-based TP1 (x1.5) / TP2 (x3.0) / TP3 (x5.0) / SL (x1.0)
- [x] `take_profit_2` + `take_profit_3` dans `strategies::Signal`
- [x] Endpoint `GET /api/smc/analyse`
- [x] `SmcScoreCard.vue` composant extrait

### ✅ Semaine 15-16: UI complète
- [x] `PnLView.vue` — courbe equity, ROI/Sharpe/WinRate/Drawdown/ProfitFactor
- [x] `HistoryView.vue` — tableau signaux filtrables (asset/direction/stratégie), pagination
- [x] `HeatmapView.vue` — grille ATR volatilité x timeframes, refresh 60s
- [x] `AlerteStore` + `ToastAlerte.vue` — système de notifications in-app
- [x] `GET /api/signaux/export` — export CSV

### ✅ Semaine 16b: SMC IA — Analyste & Coach (Ollama local)
- [x] `ollama.rs` — client HTTP vers localhost:11434
- [x] `POST /api/ia/analyse` — analyse narrative signal SMC
- [x] `POST /api/ia/chat` — coach conversationnel multi-turn (max 40 messages)
- [x] `POST /api/ia/chart` — analyse visuelle capture TradingView
- [x] `GET /api/ia/status` — vérifie disponibilité Ollama
- [x] `SMCAnalyzerView.vue` + `SMCCoachView.vue`
- [x] Modèle qwen2.5:14b (9 Go) — RTX 3090 GPU locale

### ✅ Semaine 16c: Refonte UI Dashboard + Charts (14 mars 2026)
- [x] Layout plein écran (w-full — suppression container mx-auto)
- [x] Dashboard : Capital + Statut Système sur même ligne
- [x] Statut Système : mini-cartes en grille 2x2 (label + valeur en colonne)
- [x] Bande de prix dynamique 13 actifs (flex-wrap + flex-1, pleine largeur)
- [x] Auto-refresh prix toutes les 30s (setInterval + cleanup onUnmounted)
- [x] Prédiction IA + Score SMC déplacés dans ChartsView (côte à côte)
- [x] ChartsView : rechargement IA/SMC à chaque changement d'asset/timeframe

---

## 🟡 PHASE 3 - OPTIMISATION & NOUVELLES FONCTIONNALITÉS (Semaines 17-24)

> **Statut : 🔴 A FAIRE** — Classé par complexité croissante

---

### ✦ COMPLEXITÉ 1 — Frontend seul, zéro backend

#### ✅ Semaine 17a: Horloges ouvertures de marché (Dashboard) — TERMINÉE 14 mars 2026
> Composant statique pur, aucun backend nécessaire

- [x] Composant `MarketClocks.vue` — affiché sur le Dashboard
- [x] 4 sessions avec horaires UTC fixes :
  - Asie (Tokyo) : 00h00 – 09h00 UTC
  - Hong Kong / Shanghai : 01h00 – 09h00 UTC (UTC+8)
  - Londres / Europe : 08h00 – 17h00 UTC
  - New York : 13h30 – 20h00 UTC
- [x] Badge coloré : vert = session active, gris = fermée, orange = ouverture dans <30min
- [x] Countdown live (setInterval 1s) vers la prochaine ouverture / fermeture

---

### ✦ COMPLEXITÉ 2 — Port de code existant (App.jsx → Vue)

#### ✅ Semaine 17b: Analyse de charts importés (Image Upload → Ollama) — TERMINÉE 14 mars 2026
> Port App.jsx → Vue avec bonus multi-TF top-down. Commits : ba9eaef → 057313f

- [x] Drag & drop multi-images dans `ChartImportPanel.vue` (FileReader, previews grille)
- [x] Sélecteur TF par image pour analyse top-down (H4 → M15 → M5)
- [x] Envoi multi-images base64 vers `POST /api/ia/chart` — toutes images en un seul message Ollama
- [x] `PROMPT_VISION_ANALYST` (1 image) et `PROMPT_VISION_MULTI_TF` (N images) — `ollama.rs`
- [x] Modèle vision upgradé : `llama3.2-vision:11b` (7.8 GB, RTX 3090)
- [x] Parser `<htmldiagram>...</htmldiagram>` multi-blocs — `useChartImport.ts`
- [x] Rendu diagrammes HTML interactifs dans `<iframe>` sandboxés
- [x] Notes contextuelles optionnelles (textarea)
- [x] Sous-menu **IAnalyse** dans la sidebar (Signal + Chart Import comme routes séparées)
- [x] `renderMd()` — markdown → HTML sécurisé (headers, code, listes, bold, italic)

---

### ✦ COMPLEXITÉ 3 — Prompt engineering + configuration

#### ✅ Semaine 18a: Affinement du prompt SMC pour signaux précis — TERMINÉE
> `PROMPT_SIGNAL_JSON` dans `ollama/prompts.rs` · Endpoint `POST /api/ia/signal` dans `ollama_handlers.rs`

- [x] Prompt signal dédié (`PROMPT_SIGNAL_JSON`) : JSON strict avec direction, prix_entree, stop_loss, TP1/TP2/TP3, confluences, score_confiance /10, niveau_invalidation
- [x] Prompt analyse narrative conservé (`PROMPT_VISION_ANALYST`) pour SMCAnalyzerView
- [x] Endpoint `POST /api/ia/signal` — parse JSON LLM → construit `common::Signal` injecté dans le pipeline (`strategie = "SMC-IA"`)
- [ ] Test A/B — **reporté à S19** (nécessite données historiques massives S19 — collecte)

#### ✅ Semaine 18d: Mise à niveau prompts + Kill Zone filter + Liquidity Sweep — TERMINÉE 20 mars 2026
> Carences identifiées lors de l'analyse comparative des prompts SMC/Straddle (20 mars 2026)
> Prérequis pour S19-S21 : sans ces filtres, les signaux entraînés contiennent du bruit hors-session
> Commit : `ce630bb`

- [x] **promptSMC.md → `PROMPT_SIGNAL_SMC`** : remplace `PROMPT_SIGNAL_JSON` dans `ollama/prompts.rs` — conditions bloquantes (kill_zone + sweep + score ≥60 + ML ≥0.60), scoring enrichi /10
- [x] **promptSTRADDLE.md → `PROMPT_SIGNAL_STRADDLE`** : nouveau prompt dans `ollama/prompts.rs`, 3 déclencheurs (annonce HIGH impact / ATR ratio ≥1.4 / pattern récurrent), output `direction = "Both"`
- [x] **Kill Zone filter** (`smc/src/kill_zone.rs`) : `est_en_kill_zone(ts) -> bool` — London 07h-10h UTC, NY 13h30-16h30 UTC, 7 macros ICT (fenêtres 20 min), weekend inactif — 4 tests unitaires
- [x] **Liquidity Sweep detector** (`smc/src/sweep.rs`) : `detecter_sweep() -> Option<SweepLiquidite>` — faux breakout swing high/low + close retour + confirmation bougie suivante
- [x] **Endpoint `POST /api/ia/signal/straddle`** : `straddle_handlers.rs` — injecte `PROMPT_SIGNAL_STRADDLE` + ATR ratio + sessions + annonces imminentes, `Direction::Both`, `strategie = "Straddle"`
- [x] Kill Zone et Sweep intégrés dans `SmcDirectionalStrategy::analyze()` comme garde-fous pre-signal (conditions bloquantes, return Ok(None) si absent)

---

### ✦ COMPLEXITÉ 4 — Endpoint API + composant frontend

#### ✅ Semaine 18b: Indicateurs visuels paramétrables sur les graphiques — TERMINÉE
> Endpoint `GET /api/indicators` + overlays TradingView + panneau config + liquidités BSL/SSL

- [x] Serialize ajouté aux 5 structs SMC (`OrderBlock`, `Imbalance`, `Ifvg`, `NiveauxFibonacci`, `ResultatTendance`)
- [x] Module `smc::liquidites` — détection BSL/SSL (swing high/low LOOKBACK=3, tolerance 0.1%, max 10 niveaux)
- [x] Endpoint `GET /api/indicators?asset=BTC&tf=M15&ema=true&...` — retourne séries EMA/RSI/MACD/Bollinger/ATR + zones SMC + liquidités
- [x] Overlay TradingView via prix lines (EMA couleur ambre, Bollinger indigo, OB/FVG/IFVG zones colorées, Fibonacci niveaux, BSL/SSL lignes)
- [x] `IndicatorPanel.vue` — 2 sections (Techniques + SMC), toggle + périodes configurables
- [x] `useChartIndicators.ts` — composable gérant création/suppression séries overlays
- [x] `settings.store.ts` étendu — `indicateurs` (13 préférences) persistées localStorage
- [x] `ChartsView.vue` intégré — bouton toggle + panel + rechargement overlays sur changement bougies (295L < 300L ✅)

#### ✅ Semaine 18c: Calendrier économique (Dashboard) — TERMINÉE 19 mars 2026
> ForexFactory JSON feed (thisweek + nextweek) — cache SQLite TTL 1h

- [x] Endpoint backend `GET /api/calendar?days=3` — fetch + filtre impact High/Medium
- [x] Cache SQLite des annonces (TTL 1h, `INSERT OR REPLACE`) — migration `0002_calendar_cache.sql`
- [x] Composant `EconomicCalendar.vue` sur le Dashboard (sous `<MarketClocks />`) :
  - Grille compacte 12 colonnes, hauteur fixe h-20, cartes responsive
  - Devise + titre + countdown par carte, tooltip détail au survol (heure locale, UTC, préc, prévis)
  - Toasts persistants (fermeture manuelle uniquement)
- [x] Alerte toast automatique 15min avant une annonce à fort impact (+ `annoncesAlertees` Set anti-doublon)
- [x] ROADMAP S21 : enrichissement contexte LLM Straddle via `/api/calendar`

---

### ✦ COMPLEXITÉ 5 — Infrastructure backend lourde

#### Semaine 19a: Signal Engine automatique (background task)
> Génération autonome des signaux SMC et Straddle sans intervention utilisateur
> Prérequis : S18d complète (Kill Zone + Sweep + prompts) ✅

- [ ] `signal_engine.rs` dans `api/src/` — boucle `tokio::spawn` toutes les 5 minutes
- [ ] Pour chaque (asset × timeframe) configuré : charger candles → `SmcDirectionalStrategy::analyze()` → si OK → appel Ollama → persist DB
- [ ] Anti-doublon : ne pas générer si signal identique dans les 30 dernières minutes
- [ ] Guard global : désactivé hors Kill Zone (pas d'appel Ollama inutile)
- [ ] `POST /api/signal-engine/start` + `POST /api/signal-engine/stop` + `GET /api/signal-engine/status`
- [ ] Push WebSocket aux clients connectés sur nouveau signal (`/api/stream`)
- [ ] Indicateur visuel dans le Dashboard : "🟢 Signal Engine actif — prochaine analyse dans Xmin"

#### Semaine 19b: Données historiques + Collecte massive
> Prérequis indispensable à tout entraînement ML réaliste et à la détection de volatilité

- [ ] Endpoint `POST /api/data/collect` — collecte bulk N mois de bougies par asset/tf
- [ ] Stockage DB optimisé (INSERT OR IGNORE) pour éviter les doublons
- [ ] Script de collecte initiale : 6 mois x 13 assets x M1/M5/M15 (~500k bougies)
- [ ] Indicateur de progression frontend pendant la collecte
- [ ] Vue `DataManagementView.vue` — statut couverture données par asset

#### Semaine 20: Entraînement automatique + Monitoring ML
- [ ] Scheduler Rust (tokio::time) — reentraînement quotidien a 00h00
- [ ] Walk-forward optimization : fenêtre glissante 3 mois train / 1 mois test
- [ ] Détection dérive modèle : alerte si accuracy < seuil sur 7 derniers jours
- [ ] `GET /api/ml/history` — historique entraînements (date, accuracy, durée)
- [ ] Affichage courbe d'accuracy dans `PnLView.vue`

#### Semaine 21: Détection automatique de volatilités récurrentes (Straddle IA)
> Dépend de la collecte S19 — nécessite 6 mois d'historique minimum
> Le prompt Straddle définitif (`promptSTRADDLE.md`) est prêt — S21 l'alimente en données historiques

- [ ] Analyse distribution ATR par heure du jour et jour de la semaine
- [ ] Identification patterns récurrents : ouvertures marché, annonces économiques
- [ ] Clustering k-means sur features temporelles (heure, jour, session)
- [ ] Calibration automatique des seuils ATR pour la stratégie Straddle
- [ ] Rapport `GET /api/volatility/patterns` — heatmap horaire des pics ATR
- [ ] Visualisation dans `HeatmapView.vue` (axe heure du jour en plus de l'axe asset)
- [ ] Enrichissement contexte LLM Straddle : injecter les annonces High (<2h) dans
      le prompt Ollama avant décision (`/api/calendar` → champ `annonces_imminentes`)

---

### ✦ COMPLEXITÉ 6 — Refactoring ML profond + GPU

#### Semaine 22: XGBoost + Accélération CUDA
> Remplacement du RandomForest + activation GPU RTX 3090 pour le ML trading

- [ ] Intégration crate `xgboost` (bindings C++) — `ml/src/xgboost.rs`
- [ ] Fusion LSTM 60% + XGBoost 40% (remplace RandomForest dans PipelineHybride)
- [ ] Accélération GPU pour LSTM via `candle` (Hugging Face) ou `cudarc`
- [ ] Benchmark inférence : objectif <200ms sur GPU vs CPU actuel
- [ ] Tests : accuracy XGBoost vs RF sur le même jeu de données historiques

---

### ✦ COMPLEXITÉ 7 — Finalisation + tests + alertes système

#### Semaine 23-24: Alertes OS, Export PDF, Coverage tests >80%
- [ ] Notifications OS natives via Tauri (tauri-plugin-notification)
- [ ] Alertes sonores sur nouveau signal (fichier .ogg embarqué)
- [ ] Export PDF P&L via printpdf ou capture HTML->PDF
- [ ] Coverage tests >80% : smc (4 modules), strategies (SMC Directionnel), api, data, db
- [ ] Documentation technique complète

---

## 🚀 PHASE 4 - PRODUCTION RÉELLE

> **Statut : A VENIR** — après validation Phase 3

### Paper Trading (2-4 semaines)
- [ ] Simulateur d'exécution d'ordres (prix réel, sizing réel, sans envoi broker)
- [ ] Journal des trades simulés avec P&L en temps réel
- [ ] Validation métriques : Win Rate >55%, Sharpe >1.5, Drawdown <20%

### Trading Réel Progressif
- [ ] Connexion IB Gateway mode LIVE (port 4001 — actuellement port 4002 paper)
- [ ] Exécution d'ordres via ibapi : place_order, cancel_order
- [ ] Gestion positions ouvertes : `GET /api/positions`
- [ ] Monitoring 24/7 — watchdog process + redémarrage auto

---

## 📊 ÉTAT COUVERTURE TESTS (14 mars 2026)

| Crate | Tests | Couverture estimée |
|-------|-------|--------------------|
| backtest | 2 | ~60% |
| indicators | 4 | ~70% |
| ml | 3 | ~40% |
| risk | 5 | ~80% |
| smc | 2 | ~20% — Order Blocks, Imbalance, IFVG non testés |
| strategies | 3 | ~50% — SMC Directionnel non testé |
| api, data, db | 0 | 0% |
| **Total** | **19** | **~35%** — objectif Phase 3 : >80% |

---

## 📈 MÉTRIQUES CIBLES

| Phase | ROI | Sharpe | Max DD | Win Rate | Latence |
|-------|-----|--------|--------|----------|---------|
| MVP | >0% | >0.5 | <30% | >45% | <10s |
| Prod | >15% | >1.5 | <20% | >55% | <5s |

---

**Documents complémentaires :** [ARCHITECTURE.md](ARCHITECTURE.md) · [CAHIER_DES_CHARGES.md](CAHIER_DES_CHARGES.md)
