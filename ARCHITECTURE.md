# 🏛️ Architecture Technique — Native Trading AI

**Stack:** Rust (9 crates) + Vue 3 + Tauri | **DB:** SQLite (42 migrations) | **ML:** XGBoost + LSTM hybride | **GPU:** CUDA 11.8+ (optionnel)

---

## 1. Vue d'Ensemble

> ⚠️ **Application 100% native.** Aucun navigateur. L'interface s'ouvre dans une fenêtre **Tauri** (1440×900). Le frontend Vue.js est embarqué dans le binaire Tauri. Le backend Rust écoute sur le port **8080 interne** uniquement.

```
┌────────────────────────────────────────────────────────────────────┐
│               Fenêtre Tauri Native (1440×900, maximisée)           │
│                Vue.js 3 + TypeScript + TailwindCSS                 │
│  18 vues │ 70+ composants │ 11 stores Pinia │ 39 composables       │
└─────────────────────────────┬──────────────────────────────────────┘
                              │ HTTP + WebSocket (localhost:8080)
┌─────────────────────────────▼──────────────────────────────────────┐
│                  Backend Rust — Actix-Web 4 (port 8080)            │
│  97 endpoints REST (3 fichiers routes) │ 2 WebSocket streams       │
│  Signal Engine (Tokio) │ Workers périodiques (hebdo/5min)          │
│  Dépend de 8 crates: data, db, ml, indicators, smc, strategies,    │
│                       risk, common                                 │
└───────┬────────────────────┬───────────────────────┬───────────────┘
        │                    │                       │
        ▼                    ▼                       ▼
┌───────────────┐  ┌──────────────────┐  ┌──────────────────────────┐
│ SQLite (WAL)  │  │  ML Models       │  │  Providers externes      │
│ 42 migrations │  │ modele_xgboost   │  │  Binance WebSocket       │
│ WAL + 10s TO  │  │ modele_lstm      │  │  IG Markets (REST+LS)    │
│               │  │ modele_rf.json   │  │  Ollama LLM (local)      │
└───────────────┘  └──────────────────┘  └──────────────────────────┘
```

---

## 2. Backend Rust — Workspace (9 Crates)

```toml
[workspace]
resolver = "2"
members = [
  "crates/api",
  "crates/common",
  "crates/data",
  "crates/db",
  "crates/indicators",
  "crates/ml",
  "crates/risk",
  "crates/smc",
  "crates/strategies",
]
```

### Couches DAG (Dependency Acyclic Graph)

```
Layer 4 — API Handlers + Workers
    crate: api  ←  dépend de tous les autres
         │ imports only ↓
Layer 3 — Services (Strategies, ML, Risk)
    crates: strategies, ml, risk
         │ imports only ↓
Layer 2 — Data Access (DB, Providers)
    crates: db, data
         │ imports only ↓
Layer 1 — Modèles partagés
    crates: common, indicators, smc
```

**Règle absolue :** Pas d'import horizontal entre crates de même niveau. Pas de cycle. Tout type partagé remonte dans `common`.

---

### Crate `common` — Types partagés (Layer 1)

```rust
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64, pub high: f64, pub low: f64, pub close: f64,
    pub volume: f64,
}

pub enum Timeframe { M1, M5, M15, M30, H1, H4, D1, W1 }
pub enum Direction { Long, Short, Both }
pub enum Asset { BTC, ETH, SOL, XAUUSD, XAGUSD, EURUSD, GBPUSD, ... }
```

**Dépendances :** serde, chrono, uuid, thiserror, anyhow

---

### Crate `smc` — Indicateurs SMC (Layer 1)

5 modules d'indicateurs :
- `bpr` — Break of Structure + Price Reversal
- `fibonacci` — Niveaux 38.2%, 50%, 61.8%
- `imbalance` — Fair Value Gap (FVG ≥3 pips)
- `order_blocks` — OB avec volume élevé
- `tendances` — Structure HH/HL (haussier) vs LH/LL (baissier)

---

### Crate `indicators` — Indicateurs techniques (Layer 1)

ATR, RSI, MACD, Bollinger Bands, Volume Profile.

---

### Crate `data` — Providers (Layer 2)

- **Binance** : WebSocket temps réel (BTC, ETH, SOL — OHLCV M1/M5/M15)
- **IG Markets** : REST + Lightstreamer (métaux, forex, indices)
- Trait `DataProvider` → implémentation interchangeable

---

### Crate `db` — SQLite (Layer 2)

- Pool SQLx avec WAL mode, timeout 10s
- 42 migrations (schema versionné)
- Modules : `bougies`, `bougies_ext`, `calendrier`, `signaux`, `straddle`, `rockets`, `smc_*`, `ml_*`, `assets`, `volatilite`, `config`

---

### Crate `ml` — Pipeline ML (Layer 3)

**Modèle hybride :**
```
Score_Final = 0.6 × LSTM + 0.4 × XGBoost
```

- **LSTM** : 3 couches (128→64→32), séquences 60 bougies
- **XGBoost** : 100 arbres, max_depth=6
- **Features** : 50+ (OHLCV normalisés, ATR, RSI, MACD, Bollinger, SMC)
- **Contrainte** : inférence < 200ms (mesurée avec `Instant::now()`)

---

### Crate `strategies` — Stratégies de trading (Layer 3)

3 stratégies métier :

**1. Straddle (Volatilité extrême)**
- Déclencheur : ATR > 150% moyenne + IA indécise
- Exécution : positions opposées simultanées (LONG + SHORT)
- TP/SL : ATR × 2 / ATR × 0.5
- Risk : 1% par direction (2% total)

**2. SMC Directionnel (Confluence)**
- Scoring : Tendance + IA + Order Block + Imbalance + FVG + Fibonacci
- Seuil : ≥ 70/100
- Exécution : direction unique, TP pyramidal (3 niveaux)
- Risk : 1.5% par trade

**3. Rockets (Momentum)**
- Scan multi-actifs sur rupture de structure
- Scoring ML + confluence SMC
- Suivi progressif (vente partielle aux TP)

Module `straddle_precision` : analyse M1 historique pour trouver le timing exact (à la minute) du pic de volatilité dans un créneau horaire donné.

---

### Crate `risk` — Risk Management (Layer 3)

Vérifications pré-signal **non négociables** :
- Position size ≤ 2% capital
- Positions simultanées ≤ 3
- Exposition par actif ≤ 25% capital
- Drawdown max 20% → arrêt automatique du trading

---

### Crate `api` — Actix-Web (Layer 4)

**97 routes REST** réparties en 3 fichiers :
- `routes.rs` — 75 routes (signaux, straddle, SMC, IA, assets, data, settings...)
- `routes_ml.rs` — 10 routes (ML insights, retrain, calibration...)
- `routes_rockets.rs` — 12 routes (Rockets signaux, analyses, feedback...)

**AppState (état partagé) :**
```rust
pub struct AppState {
    pub db: Arc<Database>,
    pub pipeline_ml: Arc<Mutex<PipelineML>>,
    pub retrain_state: Arc<RwLock<RetainState>>,
    pub ig_session: Arc<Mutex<IgSession>>,
    pub ig_lightstreamer: Arc<IgLightstreamer>,
    pub signal_engine: Arc<SignalEngine>,
    pub fear_greed_cache: Arc<RwLock<Option<(Instant, Value)>>>,
}
```

**78 modules déclarés** dans `main.rs` dont :
- Handlers : `signaux_handlers`, `straddle_handlers`, `smc_handlers`, `rockets_handlers`, `ml_handlers`, `engine_handlers`, `calendar_handlers`, `news_handlers`, `prix_handlers`...
- Workers : `straddle_boucle`, `smc_boucle`, `straddle_feedback_job`, `smc_feedback_job`, `rockets_scan`, `scheduler`
- IA : `ollama`, `ollama_signal_ia_handler`, `ollama_chart_handler`, `ollama_chat_handler`
- Intégration : `ig_session`, `ig_lightstreamer`, `telegram`
- Nouveau : `straddle_precision_handler` (endpoint `/api/straddle/precision-horaire`)

---

## 3. Frontend — Vue 3 + TypeScript

### Navigation (SideBar)

| Groupe | Liens |
|--------|-------|
| 🏠 *(racine)* | Dashboard |
| 🗂️ **Général** | Graphiques, Historique des positions (clôturés seulement), Lexique |
| 🚀 **Rockets** | Signaux (en cours), Définition & Prompt IA |
| 📐 **SMC** | Signaux (en cours), Définition & Prompt IA |
| ⚡ **Volatilité** | Signaux (en cours), Heatmap, Définition & Prompt IA |
| 🧠 **Outils IA** | Analyse graphique, Coach IA |
| ⚙️ **Configuration** | Paramètres, Import des données, Configuration IA |

### Vues (18)

```
DashboardHome.vue          — Dashboard principal (temps réel)
RocketsView.vue            — Trades Rockets en cours
RocketsDefinitionView.vue  — Configuration stratégie Rockets
SmcView.vue                — Trades SMC en cours
SmcDefinitionView.vue      — Configuration stratégie SMC
SMCCoachView.vue           — Coach IA (chat Ollama)
StraddleView.vue           — Vue principale Straddle
StraddleSignauxView.vue    — Trades Straddle en cours
VolatiliteDefinitionView.vue — Configuration stratégie Volatilité
HeatmapView.vue            — Calendrier historique + Radar ATR temps réel
HistoryView.vue            — Historique (trades clôturés uniquement)
ChartsView.vue             — Graphiques TradingView + indicateurs SMC
ChartImportView.vue        — Import de données + analyse graphique IA
MlInsightsView.vue         — Insights ML (performances modèles)
LexiqueView.vue            — Lexique trading
DataManagementView.vue     — Gestion des données
SettingsView.vue           — Paramètres application
PromptsIAView.vue          — Configuration des prompts IA
```

### Stores Pinia (11)

| Store | Rôle |
|-------|------|
| `signal.store` | Signaux actifs + live trading state |
| `prix.store` | Prix temps réel par asset |
| `assets.store` | Catalogue assets disponibles |
| `assetParams.store` | Paramètres par asset (lot, spread...) |
| `market.store` | État des marchés (sessions, horaires) |
| `mlInsights.store` | Métriques performances ML |
| `news.store` | Actualités + sentiment marché |
| `settings.store` | Configuration utilisateur |
| `strategyParams.store` | Paramètres SMC/Straddle/Rockets |
| `alerte.store` | Toasts, erreurs, notifications |
| `signal-alarme.store` | Alertes sur signaux |

### Services API (12 fichiers)

```
api.service.ts         — Client principal (agrège tous les sous-services)
api.straddle.ts        — Endpoints Straddle + precision-horaire
api.smc.ts             — Endpoints SMC
api.rockets.ts         — Endpoints Rockets
api.engine.ts          — Signal Engine
api.news.ts            — Actualités + sentiment
api.ml_insights.ts     — ML performances
api.asset_params.ts    — Paramètres assets
api.types.ts           — Types partagés (Signal, PrecisionHoraire...)
api.types.indicators.ts — Types indicateurs
api.types.marche.ts    — Types marché (sessions, prix...)
api.types.rockets.ts   — Types spécifiques Rockets
```

### Composables clés (39 fichiers)

**Graphiques TradingView :**
`useChartTradingView`, `useChartOrchestration`, `chartMainOverlays`, `chartSubgraphs`, `chartSignauxRendu`, `chartIndicatorsConfig`, `chartAtrSlTp`, `chartTimeScale`, `chartAnalyseRenderer`

**SMC :**
`useSmcCanvas`, `useSmcFibCanvas`, `useSmcLiqCanvas`, `useSmcStats`, `useSmcAnalyseNotif`

**Heatmap :**
`useHeatmapConfluence`, `useHoraireAnalyse`, `useProbaHeatmap`

**Stratégies :**
`useStraddleStats`, `useRocketsStats`, `useRocketsHistory`, `useSignalEngine`

**Utilitaires :**
`useSignalFormat`, `useSignalTradeBox`, `useHistoryStats`, `useExportPdf`, `useNotification`

---

## 4. Règles d'Architecture

### Séparation Frontend / Backend

**Zéro calcul métier en frontend.** Tout calcul de prix, indicateurs, signaux, risk, P&L, SL/TP, scores ML est effectué par le backend Rust.

**Autorisé en frontend :** formatage d'affichage, tri/filtrage local de listes déjà reçues, calcul de couleur/état visuel, agrégations légères sur données déjà calculées.

**Interdit en frontend :** calcul d'ATR, RSI, MACD, Fibonacci, SL/TP, R:R, logique de scoring SMC/Straddle/Rockets, risk management.

### Gestion d'erreurs

```rust
// ❌ INTERDIT
.unwrap()
.expect()
panic!()

// ✅ REQUIS
pub fn calculer_signal() -> Result<Signal, TradingError> {
    let data = obtenir_données()?;
    match data.valider() {
        Some(v) => Ok(Signal::from(v)),
        None => Err(TradingError::Data("Données invalides".into())),
    }
}
```

### Taille des fichiers

- **Limite dure :** 300 lignes par fichier
- Déclencheur refactoring : > 250 lignes → split immédiat
- Fonction > 30 lignes → extraire sous-fonctions

---

## 5. Flux de données

```
Market Data (Binance WS / IG Lightstreamer)
    ↓
DataProvider (crate data)
    ↓
SQLite bougies (crate db)
    ↓
ML Pipeline — LSTM + XGBoost (crate ml)
    ↓  (score 0-100)
Strategy Engine — Straddle / SMC / Rockets (crate strategies)
    ↓  (signal candidat)
Risk Management (crate risk)
    ↓  (validé ou rejeté)
Signal Engine (api/signal_engine.rs — Tokio)
    ↓
WebSocket stream (/api/signal-engine/stream)
    ↓
Frontend Vue.js — Pinia stores → Composants
```

---

## 6. Workers Périodiques

| Worker | Fréquence | Rôle |
|--------|-----------|------|
| `straddle_boucle` | ~5min | Scan des créneaux Straddle actifs |
| `smc_boucle` | ~5min | Monitoring positions SMC |
| `rockets_scan` | ~5min | Scan opportunités Rockets multi-actifs |
| `straddle_feedback_job` | Continu | Clôture auto positions Straddle |
| `smc_feedback_job` | Continu | Clôture auto positions SMC |
| `smc_analyse_handler::worker` | Hebdo (Lun 02h UTC) | LLM analyse performances SMC |
| `scheduler` | Configurable | Orchestration des jobs |

---

## 7. Intégrations Externes

| Service | Protocole | Usage |
|---------|-----------|-------|
| Binance | WebSocket | OHLCV temps réel BTC/ETH/SOL |
| IG Markets | REST + Lightstreamer | Métaux, Forex, Indices |
| Ollama | HTTP local | LLM analyse signaux (llama3/mistral) |
| Anthropic Claude | HTTPS | LLM fallback cloud |
| ForexFactory | HTTPS | Calendrier économique (TTL 1h) |
| Telegram | HTTPS | Notifications alertes |
| Fear & Greed Index | HTTPS | Sentiment marché crypto |

---

## 8. Métriques de Performance

| Métrique | Objectif |
|----------|---------|
| Inférence ML | < 200ms |
| Latence signaux | < 10s end-to-end |
| ROI annualisé | > 15% |
| Sharpe ratio | > 1.5 |
| Win rate | > 55% |
| Max drawdown | < 20% |
| Accuracy ML classification | > 60% |
| F1-Score ML | > 0.55 |
