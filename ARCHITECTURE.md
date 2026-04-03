# 🏛️ Architecture Technique — Native Trading AI

**Stack:** Rust (10 crates) + Vue 3 + Tauri | **DB:** SQLite | **ML:** XGBoost + LSTM hybride | **GPU:** CUDA 11.8+ (optionnel)

---

## 1. Vue d'Ensemble

> ⚠️ **Application 100% native.** Aucun navigateur. L'interface s'ouvre dans une fenêtre **Tauri** (1440×900). Le frontend Vue.js est embarqué dans le binaire Tauri. Le backend Rust écoute sur le port **8080 interne** uniquement.

```
┌────────────────────────────────────────────────────────────────────┐
│               Fenêtre Tauri Native (1440×900, maximisée)           │
│                Vue.js 3 + TypeScript + TailwindCSS                 │
│  21 dashboards │ Charts TradingView │ Pinia Stores │ Composables  │
└─────────────────────────────┬──────────────────────────────────────┘
                              │ HTTP + WebSocket (localhost:8080)
┌─────────────────────────────▼──────────────────────────────────────┐
│                  Backend Rust — Actix-Web 4 (port 8080)            │
│  75+ endpoints REST │ 2 WebSocket streams │ Signal Engine (Tokio)  │
│  Dépend de 9 crates: data, db, ml, indicators, smc, strategies,    │
│                       backtest, risk, common                       │
└───────┬────────────────────┬───────────────────────┬───────────────┘
        │                    │                       │
        ▼                    ▼                       ▼
┌───────────────┐  ┌──────────────────┐  ┌──────────────────────┐
│ SQLite (WAL)  │  │  ML Models       │  │  Providers externes  │
│ 24 migrations │  │ modele_xgboost   │  │  Binance WebSocket   │
│ 10+ tables    │  │ modele_lstm      │  │  IB Gateway (TCP)    │
│ WAL + 10s TO  │  │ modele_rf.json   │  │  Ollama LLM (local)  │
└───────────────┘  └──────────────────┘  └──────────────────────┘
```

---

## 2. Backend Rust — Workspace (10 Crates)

```toml
# Cargo.toml workspace
resolver = "2"
[profile.release]
lto = true
opt-level = 3
codegen-units = 1
strip = true
```

### Couches DAG (Dependency Acyclic Graph)

```
Layer 4 — API Handlers (commands)
    crate: api  ←  dépend de tous les autres
         │ imports only ↓
Layer 3 — Services (Strategies, ML)
    crates: strategies, backtest, ml
         │ imports only ↓
Layer 2 — Data Access (DB, Providers)
    crates: db, data, risk
         │ imports only ↓
Layer 1 — Modèles partagés
    crates: common, indicators, smc
```

**Règle absolue :** Pas d'import horizontal entre crates de même niveau. Pas de cycle. Tout type partagé remonte dans `common`.

---

### Crate `common` — Types partagés (Layer 1)

Types métier fondamentaux :

```rust
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64, pub high: f64, pub low: f64, pub close: f64,
    pub volume: f64,
}

pub struct Signal {
    pub id: Uuid,
    pub asset: String,
    pub timeframe: String,
    pub direction: Direction,
    pub score: f64,             // 0–100
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: Vec<f64>,  // [tp1, tp2, tp3]
    pub strategie: String,
    pub cree_le: DateTime<Utc>,
}

pub enum Timeframe { M1, M5, M15, M30, H1, H4, D1, W1 }
pub enum Direction { Long, Short, Both }

// 35 assets : Crypto Binance + Métaux IB + Forex IB + Indices IB
pub enum Asset { BTC, ETH, SOL, XAUUSD, XAGUSD, EURUSD, ... }
```

**Dépendances :** serde, chrono, uuid, thiserror

---

### Crate `data` — Acquisition Marché (Layer 2)

```rust
pub trait DataProvider: Send + Sync {
    async fn fetch_candles(
        &self, asset: &str, timeframe: Timeframe, limit: usize
    ) -> Result<Vec<Candle>>;
}
```

**Implémentations :**

| Provider | Assets | Transport |
|----------|--------|-----------|
| `BinanceProvider` | BTC, ETH, SOL, ... (Crypto) | WebSocket temps réel + REST fallback |
| `IbGatewayProvider` | XAUUSD, XAGUSD, Forex, Indices | IB API async (TCP port configurable) |
| `DataAggregator` | Tous | Dispatch : crypto → Binance, autres → IB |

**Dépendances :** common, tokio, reqwest, ibapi 2.10

---

### Crate `db` — Persistance SQLite (Layer 2)

- **Engine :** SQLite via SQLx 0.8, WAL mode, `busy_timeout = 10s`
- **Migrations :** 24 fichiers, appliquées au démarrage automatiquement

**Tables principales :**

```sql
-- OHLCV
bougies(id, asset, timeframe, timestamp, open, high, low, close, volume)
  UNIQUE(asset, timeframe, timestamp)
  INDEX(asset, timeframe, timestamp DESC)

-- Signaux
signaux(id UUID, asset, timeframe, direction, score, prix_entree,
        stop_loss, take_profit JSON, strategie, statut, cree_le, ferme_le)
  INDEX(asset, statut, cree_le DESC)

-- Positions
positions(id, signal_id FK, asset, direction, taille, prix_entree,
          prix_sortie, pnl, statut, ouverte_le, fermee_le)
```

**Tables spécialisées (24 migrations) :**

| Table | Migration | Usage |
|-------|-----------|-------|
| `rockets_signaux` | 0008 | Signaux Rockets + verdict |
| `straddle_creneaux` | 0015 | Créneaux Straddle (jour, heure, ATR, fréq) |
| `entrainements` | 0004 | Historique ML |
| `assets` | 0005 | Catalogue assets |
| `calendrier` | 0002 | Événements économiques |
| `news_*` | 0003, 0020, 0021 | News, lus, sentiments |
| `strategies_params` | 0023 | Paramètres persistés |

**Modules :** bougies (INSERT OR IGNORE), signaux, rockets, straddle, volatilite, strategies_params

---

### Crate `indicators` — Indicateurs Techniques (Layer 1)

Calculs pur Rust sur `Vec<Candle>` :

| Fonction | Description |
|----------|-------------|
| `calculer_atr(bougies, period)` | Average True Range |
| `calculer_rsi(bougies, period)` | RSI (14 par défaut) |
| `calculer_macd(bougies, 12, 26, 9)` | MACD + Signal + Histogramme |
| `calculer_bollinger(bougies, 20, 2.0)` | Bandes de Bollinger |
| `calculer_sma(bougies, period)` | Simple Moving Average |
| `calculer_ema(bougies, period)` | Exponential Moving Average |
| `calculer_supertrend(bougies, period, mult)` | SuperTrend |

**Dépendances :** common, ta 0.5

---

### Crate `smc` — Smart Money Concept (Layer 1)

Scoring de confluence 100 points :

```
scorer(bougies: &[Candle]) -> Option<ScoreSmc>
    Retourne None si score < 70
```

**Décomposition du score :**

| Module | Poids | Logique |
|--------|-------|---------|
| `tendances` | 25 pts | Structure HH/HL (haussier) ou LH/LL (baissier) |
| `order_blocks` | 25 pts | Dernière bougie avant impulsion, volume élevé |
| `ifvg` | 20 pts | FVG + break of structure |
| `imbalance` | 15 pts | Gap ≥3 pips sans retrace |
| `fibonacci` | 15 pts | Niveaux 38.2%, 50%, 61.8% |

**Modules annexes :** kill_zone, sweep, liquidites

**Dépendances :** common, indicators

---

### Crate `ml` — Pipeline ML Hybride (Layer 3)

Architecture d'inférence :

```
60 bougies M15 (900 min = 15h)
        ↓
50+ Features extraites :
  OHLCV normalisés + ATR, RSI(14), MACD(12,26,9)
  Bollinger(20,2) + SMA(9,20,50) + Volume profile
  SMC scores (tendance, OB, IFVG, imbalance, fib)
  Quantiles prix + Volatilité (std)
        ↓
  ┌─────────────────────┬──────────────────────┐
  │   XGBoost (40%)     │     LSTM (60%)        │
  │  100 trees          │  3 layers: 128→64→32  │
  │  max_depth = 6      │  seq_len = 10         │
  │  100% CPU           │  CPU ou CUDA optionnel│
  │  modele_xgboost.json│  modele_lstm.json     │
  └──────────┬──────────┴──────────┬────────────┘
             └──────────┬──────────┘
                        ↓
              score_final = 0.4×XGB + 0.6×LSTM
                        ↓
               PredictionML {
                 direction: Direction,
                 confiance: f64,     // 0.0–1.0
                 est_confiant: bool, // confiance > 0.6
               }
```

**Performance cible :**

| Scénario | Latence cible |
|----------|---------------|
| Inférence CPU | < 200ms |
| Inférence CUDA (RTX 3090) | < 50ms |
| Accuracy fusion | > 65% |
| F1-Score | > 0.55 |

**Fichiers spéciaux :** `walk_forward.rs` (validation walk-forward), `raffinement.rs` (update weights post-backtest)

**Dépendances :** common, ndarray 0.15, polars 0.35, tch 0.14 (optionnel CUDA), smartcore 0.4, statrs 0.16

---

### Crate `strategies` — Logique Métier (Layer 3)

```rust
pub trait Strategy: Send + Sync {
    fn analyze(&self, bougies: &[Candle]) -> Result<Option<Signal>>;
}
```

**3 Stratégies implémentées :**

#### SMC Directionnel
- **Déclencheur :** ScoreSmc ≥ 70/100 ET ML confiant (`est_confiant = true`)
- **Direction :** Long ou Short selon confluence SMC
- **TP :** Pyramidal 3 niveaux (tp1, tp2, tp3)
- **SL :** ATR × multiplicateur configurable
- **Risk :** 1.5% capital par trade

#### Straddle Volatilité
- **Déclencheur :** ATR > 150% de sa moyenne ET ML indécis (`est_confiant = false`)
- **Direction :** `Direction::Both` (LONG + SHORT simultanés)
- **TP :** ATR × 2
- **SL :** ATR × 0.5
- **Risk :** 1% par direction (2% total)

#### Rockets Momentum
- **Déclencheur :** Volume ratio élevé + ATR ratio extrême + RSI
- **Direction :** Long ou Short selon momentum
- **Usage :** Scanner multi-actifs, signaux court terme

**Dépendances :** common, indicators, smc, ml

---

### Crate `risk` — Gestion du Risque (Layer 2)

```rust
pub struct GestionnaireRisque {
    pub capital: f64,
    pub drawdown_actuel: f64,
    pub positions_ouvertes: Vec<Position>,
    pub exposition_par_actif: HashMap<String, f64>,
}
```

**Limites non-négociables :**

```
MAX_RISK_PAR_TRADE        = 2%   (capital)
MAX_POSITIONS_SIMULTANEES = 3
MAX_EXPOSITION_ACTIF      = 25%  (capital)
MAX_DRAWDOWN              = 20%  → arrêt auto trading
```

**Méthodes :**
- `valider_signal(signal: &Signal) -> Result<bool>` — Vérifie les 4 règles ci-dessus
- `calculer_taille_position(prix_entree, stop_loss) -> f64` — 1% capital / risque par unité
- `mettre_a_jour_drawdown(drawdown_pct)`

**Dépendances :** common, serde, tracing

---

### Crate `backtest` — Simulation & Métriques (Layer 3)

```rust
pub struct BacktestEngine {
    pub capital_initial: f64,
    pub cout_friction_pct: f64,      // Spread + slippage réalistes
    pub risk_par_trade_pct: f64,     // 0.02 = 2%
    pub horizon_bougies: usize,      // Expiration post-entrée
    pub trailing_atr_mult: Option<f64>,
    pub be_atr_mult: Option<f64>,    // Break-even
    pub vente_partielle: bool,       // ⅓ TP1 + ⅓ TP2 + ⅓ trailing
}
```

**Méthodes :**
- `run(bougies, strategy) -> Result<BacktestResults>`
- `run_avec_feedback(...) -> Result<(BacktestResults, Vec<FeedbackTrade>)>` — Pour ML refinement

**Résultats retournés :**

```rust
pub struct BacktestResults {
    pub total_trades: usize,
    pub win_rate: f64,           // 0.55 = 55%
    pub roi_pct: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown_pct: f64,
    pub profit_factor: f64,      // Gains/Pertes
    pub equity_curve: Option<Vec<EquityPoint>>,
    // ... tp1/tp2/sl/expiration counts
}
```

**Logic walk-forward :** fenêtre 60 bougies, sliding window i=60..N-1, friction simulée à chaque trade.

**Dépendances :** common, strategies, indicators, chrono, serde_json, statrs

---

### Crate `api` — Serveur REST + WebSocket (Layer 4)

```
Dépend de : common + db + data + ml + strategies + backtest + risk + smc + indicators
```

**AppState (partagé via `Arc<AppState>`) :**

```rust
pub struct AppState {
    pub db: Arc<Database>,
    pub ml_pipeline: Arc<Mutex<PipelineML>>,
    pub ib_port: u16,
    pub ib_client_id: i32,
    pub data_provider: Arc<DataAggregator>,
    pub engine_actif: Arc<AtomicBool>,
}
```

**Organisation :** `main.rs` (~50L) → `routes.rs` (~250L) → 50+ handlers spécialisés

---

## 3. API REST — 75+ Endpoints

### A. Santé & Infrastructure
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/health` | `health_check()` |
| GET | `/api/ib/status` | Vérification TCP port IB Gateway |

### B. Assets & Marché
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/assets` | `lister_assets()` |
| POST | `/api/assets` | `ajouter_asset()` |
| DELETE | `/api/assets/{id}` | `supprimer_asset()` |
| GET | `/api/prix` | `get_prix()` |
| GET | `/api/prix-actuel?ticker=BTC` | `get_prix_actuel()` |
| GET | `/api/candles?asset=&timeframe=&limit=&force=` | `get_candles()` |

### C. Signaux Trading
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/signaux` | `get_signaux()` |
| POST | `/api/signal-engine/start` | `demarrer_engine()` |
| POST | `/api/signal-engine/stop` | `arreter_engine()` |
| GET | `/api/signal-engine/status` | `statut_engine()` |
| GET | `/api/signal-engine/stream` | **WebSocket — signaux live** |

### D. ML & Entraînement
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/ml/predict?asset=&timeframe=` | `predict_ml()` → `PredictionML` |
| POST | `/api/ml/train` | `entrainer_ml()` → `ReponseEntrainement` |
| GET | `/api/ml/status` | `statut_ml()` |
| POST | `/api/ml/raffine-ml` | `raffiner_ml()` — Post-backtest |
| GET | `/api/ml/history` | `historique_ml()` |

### E. Indicateurs & Analyse Technique
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/indicators?asset=&...` | `get_indicators()` |
| GET | `/api/tendance/multi-tf?asset=` | `tendance_multi_tf()` |
| GET | `/api/smc/analyse?asset=&...` | `analyse_smc()` → `ScoreSmc` |

### F. Stratégie SMC
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/smc/params` | `get_smc_params()` |
| PUT | `/api/smc/params` | `put_smc_params()` |
| GET | `/api/smc/analyse-llm` | Dernière analyse LLM |
| POST | `/api/smc/analyse-llm` | Lancer analyse LLM |

### G. Stratégie Straddle
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| POST | `/api/ia/signal/straddle` | `generer_signal_straddle()` — avec LLM |
| POST | `/api/straddle/analyser` | `analyser()` |
| GET | `/api/straddle/creneaux` | `lister_creneaux()` |
| PATCH | `/api/straddle/creneaux/{id}` | `mettre_a_jour_creneau()` |
| POST | `/api/straddle/creneaux/{id}/precision` | Feedback précision |
| POST | `/api/straddle/backtest` | `handler_backtest_slot()` |
| GET/PUT | `/api/straddle/params` | Paramètres Straddle |

### H. IA & LLM (Ollama / Anthropic)
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| POST | `/api/ia/analyse` | Texte → Ollama |
| POST | `/api/ia/chat` | Chatbot LLM |
| POST | `/api/ia/chart` | Vision chart (llava) |
| POST | `/api/ia/chart/local` | Vision locale |
| GET | `/api/ia/status` | Disponibilité Ollama |
| POST | `/api/ia/signal` | Génération signal via LLM |
| POST | `/api/ia/ajustements` | Ajuste paramètres via LLM |

### I. Rockets (Momentum Scan)
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/rockets/scan` | `get_scan()` — scan multi-actifs |
| GET | `/api/rockets/scan/debug` | `scan_momentum_debug()` |
| POST | `/api/rockets/signal` | `sauvegarder_signal()` |
| GET | `/api/rockets/historique` | `get_historique()` |
| GET/PUT | `/api/rockets/config` | Config Rockets |
| GET/POST | `/api/rockets/analyse-llm` | Analyse LLM Rockets |
| POST | `/api/rockets/sync` | `sync_verdicts()` |

### J. Backtest & Export
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| POST | `/api/backtest` | `run_backtest()` |
| POST | `/api/signaux/export` | `exporter_signaux_csv()` |

### K. News & Calendrier
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/calendar` | `get_calendar()` |
| GET | `/api/news/alertes` | `get_news_alertes()` |
| GET | `/api/news/contenu` | `get_contenu_article()` |
| GET | `/api/news/traduire` | `get_traduire()` |
| GET | `/api/news/fear-greed` | Fear & Greed Index |
| POST | `/api/news/lu` | `marquer_lu()` |
| GET | `/api/news/contexte-marche` | Contexte marché global |
| GET | `/api/sentiment/marche` | Sentiment marché |

### L. Data Management
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET | `/api/data/coverage` | `get_coverage()` |
| POST | `/api/data/collect` | `post_collect()` |
| POST | `/api/data/import-mt5` | `post_import_mt5()` — Import MT5 CSV/TSV |

### M. Config & Paramètres
| Méthode | Endpoint | Handler |
|---------|----------|---------|
| GET/POST | `/api/config` | Config globale |
| GET/PUT | `/api/strategies-params/*` | Paramètres stratégies |
| GET | `/api/volatility/patterns` | Patterns volatilité |
| POST | `/api/ia/ab-test` | AB testing stratégies |
| GET | `/api/stream` | **WebSocket — market data live** |

---

## 4. Frontend Vue.js 3

```
frontend/src/
├── App.vue               # Layout : sidebar rétractable + <router-view>
├── main.ts               # Point d'entrée, montage app
├── router/               # Vue Router — 21 routes
├── views/                # 21 dashboards (pages)
├── components/           # 20+ composants réutilisables
│   └── common/           # Composants partagés entre views
├── composables/          # 30+ hooks (useChartXxx, useHeatmapXxx, ...)
├── stores/               # 8 Pinia stores
├── services/             # Axios client + types TypeScript
└── data/                 # Fixtures statiques
```

### Views (21 Dashboards)

| Vue | Priorité | Responsabilité |
|-----|----------|----------------|
| `DashboardHome.vue` | Home | Alertes news + Market Clocks + État système |
| `ChartsView.vue` | ⭐ P1 | TradingView Charts 4 actifs + indicateurs superposés |
| `PnLView.vue` | ⭐ P2 | P&L live, positions ouvertes, trades récents |
| `HistoryView.vue` | ⭐ P3 | Historique signaux filtré/exportable |
| `SettingsView.vue` | ⭐ P4 | Configuration globale |
| `HeatmapView.vue` | ⭐ P5 | Radar ATR volatilité multi-actifs + patterns horaires |
| `ChartImportView.vue` | — | Import manuel OHLCV |
| `DataManagementView.vue` | — | Gestion couverture + import MT5 |
| `StraddleView.vue` | — | Straddle créneau manager |
| `StraddleBacktestView.vue` | — | Backtests Straddle par créneau |
| `StraddleSignauxView.vue` | — | Signaux Straddle historique |
| `SMCAnalyzerView.vue` | — | Analyseur SMC interactif |
| `SmcView.vue` | — | SMC principal + score live |
| `SmcBacktestsView.vue` | — | Historique backtests SMC |
| `SmcCoachView.vue` | — | Coach SMC (LLM) |
| `SmcDefinitionView.vue` | — | Définitions SMC (éducatif) |
| `RocketsView.vue` | — | Scanner Rockets momentum multi-actifs |
| `RocketsDefinitionView.vue` | — | Définitions Rockets (éducatif) |
| `VolatiliteDefinitionView.vue` | — | Guide volatilité (éducatif) |
| `PromptsIAView.vue` | — | Gestion prompts LLM (éditeur) |
| `LexiqueView.vue` | — | Lexique trading (éducatif) |

### Stores Pinia (8 Stores)

```typescript
// Pattern standard — composition API
export const useSignalStore = defineStore('signals', () => {
  const signaux = ref<Signal[]>([])
  const chargement = ref(false)

  async function chargerSignaux(limit = 20) {
    chargement.value = true
    try {
      signaux.value = await apiService.getSignaux(limit)
    } catch (err) {
      useAlertStore().afficherErreur(`Erreur: ${err.message}`)
    } finally {
      chargement.value = false
    }
  }
  return { signaux, chargement, chargerSignaux }
})
```

| Store | État Global |
|-------|-------------|
| `signal.store.ts` | Signaux actifs, prédictions, backtests, scores SMC |
| `market.store.ts` | OHLCV temps réel, WebSocket, erreurs |
| `prix.store.ts` | Prix actuels multi-actifs |
| `settings.store.ts` | Configuration utilisateur |
| `news.store.ts` | Articles, alertes, Fear & Greed |
| `assets.store.ts` | Catalogue assets + tri |
| `alerte.store.ts` | Notifications globales (toasts) |
| `signal-alarme.store.ts` | Alarmes signaux (son + toast) |

### Services (9 Fichiers)

| Fichier | Responsabilité |
|---------|----------------|
| `api.service.ts` | Client Axios central — 50+ méthodes |
| `api.types.ts` | Types core : Signal, Candle, PredictionML, ScoreSmc |
| `api.types.marche.ts` | Types marché : RequeteAnalyseIA, AssetInfo, CalendarEvent, News |
| `api.types.rockets.ts` | Types Rockets : RocketSignal, RocketAnalyseLlm, RocketsConfig |
| `api.types.indicators.ts` | Types indicateurs : IndicatorsParams, ZoneOb, Fibonacci |
| `api.engine.ts` | Signal Engine streaming + commands |
| `api.straddle.ts` | Straddle API calls |
| `api.rockets.ts` | Rockets API calls |
| `api.news.ts` | News API calls |

### Composables (30+ Hooks)

**Chart TradingView Lightweight :**

| Composable | Rôle |
|-----------|------|
| `useChartOrchestration.ts` | Orchestration complète (init, data, indicateurs) |
| `useChartAnalyse.ts` | Analyse LLM vision (llava) |
| `useChartImport.ts` | Import données |
| `useChartEcoCal.ts` | Calendrier économique overlay |
| `useChartIndicators.ts` | Indicateurs dynamiques |
| `useChartLimite.ts` | Rate limiting |
| `chartMainOverlays.ts` | Overlays SMC (order blocs, zones) |
| `chartSubgraphs.ts` | Sous-graphiques (volume, momentum) |
| `chartAtrSlTp.ts` | Affichage ATR + SL/TP visuels |
| `chartSignauxRendu.ts` | Rendu signaux sur chart |
| `chartTimeScale.ts` | Gestion échelle temps |
| `chartIndicatorsConfig.ts` | Configuration indicateurs |

**Business :**

| Composable | Rôle |
|-----------|------|
| `useBacktestDuree.ts` | Calcul horizon backtest |
| `useHeatmapConfluence.ts` | Détection confluences patterns horaires (cache TTL 5min) |

### Composants Réutilisables (20+)

| Composant | Rôle |
|----------|------|
| `DashboardPrixStrip.vue` | Strip prix 4 actifs + variations multi-TF |
| `VeilleRockets.vue` | Moniteur Rockets temps réel |
| `StrategiesParamsPanel.vue` | Éditeur paramètres stratégies |
| `RocketsAnalyseModal.vue` | Modale analyses Rockets LLM |
| `HeatmapAnalyseModal.vue` | Modale analyse volatilité (top volatil/calme + tendances) |
| `HeatmapTendancesTable.vue` | Table tendances SMC/MT (court vs long TF) |
| `HoraireHeatmap.vue` | Patterns horaires volatilité (dropdown filtré par assets ATR) |
| `NewsArticleModal.vue` | Lecteur articles news |
| `MarketClocks.vue` | Horloges sessions forex |
| `EconomicCalendar.vue` | Calendrier macro |
| `EcoCalTooltip.vue` | Tooltip événement macro |

---

## 5. Tauri — Fenêtre Native

**Config :** `frontend/src-tauri/tauri.conf.json`

```json
{
  "productName": "Native Trading AI",
  "version": "0.1.0",
  "identifier": "ai.native-trading.app",
  "app": {
    "windows": [{
      "width": 1440, "height": 900,
      "minWidth": 1200, "minHeight": 700,
      "resizable": true, "maximized": true,
      "decorations": true, "center": true
    }],
    "security": {
      "csp": "default-src 'self'; connect-src 'self' http://localhost:8080 ws://localhost:8080 https://api.binance.com; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:"
    }
  },
  "bundle": {
    "resources": ["sounds/"]
  }
}
```

**Points clés :**
- Frontend embarqué : `../dist` (build Vite)
- Backend autorisé par CSP : `localhost:8080` + `api.binance.com`
- Ressources bundlées : sons alertes
- Aucun navigateur, aucun port exposé au réseau

---

## 6. Flux de Données Complet

```
┌──────────────────────────────────────────────────────────────┐
│  Sources Marché                                               │
│  Binance WS (BTC, ETH, SOL, ...)                             │
│  IB Gateway TCP (XAUUSD, XAGUSD, Forex, Indices)             │
└────────────────────────────┬─────────────────────────────────┘
                             │ crate data (DataAggregator)
                             ▼
                    DB Cache — bougies (SQLite)
                             │
                             ▼
               Signal Engine (background Tokio task)
               ┌─────────────────────────────────┐
               │ 1. Obtenir 60 bougies / asset   │
               │ 2. Calculer indicateurs (ATR,   │
               │    RSI, MACD, ...) ← indicators │
               │ 3. Scoring SMC ← crate smc      │
               │ 4. ML predict ← crate ml        │
               │    (XGBoost 40% + LSTM 60%)     │
               │ 5. Détection stratégie :        │
               │    - SMC (score ≥70 + ML ok)   │
               │    - Straddle (ATR>150% + !ML) │
               │    - Rockets (momentum)         │
               │ 6. Risk validation ← crate risk │
               │ 7. Sauvegarde DB si valide      │
               │ 8. Broadcast WebSocket →        │
               └─────────────────────────────────┘
                             │ WS /api/signal-engine/stream
                             ▼
                 Frontend Vue.js (Tauri)
                 ├── Chart TradingView + signal overlay
                 ├── alerteStore toast
                 └── Utilisateur peut :
                     ├── Lancer backtest → crate backtest
                     │   └── FeedbackTrade → ML refinement
                     └── Modifier paramètres → PUT /api/*-params
```

---

## 7. Pipeline ML Détaillé

### Entraînement

```
Données : XAUUSD/XAGUSD M15 historique
Horizon prédiction : 30 bougies post-entrée
Seuil signal valide : ±2% mouvement

Phase 1 : XGBoost (features statiques)
Phase 2 : LSTM (séquences T=10)
Phase 3 : Walk-forward refinement (post-backtest)
```

### Modèles sur disque
- `data/modele_xgboost.json` — XGBoost sérialisé
- `data/modele_lstm.json` — LSTM sérialisé (poids)
- `data/modele_rf.json` — Random Forest (backup)

---

## 8. Risk Management — Règles Non-Négociables

```rust
pub fn valider_signal(signal: &Signal, etat: &GestionnaireRisque) -> Result<bool> {
    // 1. Drawdown global
    if etat.drawdown_actuel >= 20.0 { return Ok(false); }

    // 2. Positions simultanées
    if etat.positions_ouvertes.len() >= 3 { return Ok(false); }

    // 3. Exposition par actif
    let expo = etat.exposition_par_actif.get(&signal.asset).unwrap_or(&0.0);
    let taille = calculer_taille_position(signal, etat.capital)?;
    if expo + taille > etat.capital * 0.25 { return Ok(false); }

    // 4. Risk par trade
    let risque = (signal.prix_entree - signal.stop_loss).abs() * taille;
    if risque > etat.capital * 0.02 { return Ok(false); }

    Ok(true)
}
```

---

## 9. Dépendances Rust — Workspace

| Crate | Version | Usage |
|-------|---------|-------|
| tokio | 1 | Runtime async (Actix, DB) |
| actix-web | 4 | Serveur HTTP |
| sqlx | 0.8 | SQLite ORM + migrations auto |
| serde / serde_json | 1 | Sérialisation |
| chrono | 0.4 | Timestamps UTC |
| uuid | 1 v4 | IDs signaux |
| ndarray | 0.15 | Algèbre linéaire (features ML) |
| polars | 0.35 | DataFrames |
| ta | 0.5 | Indicateurs techniques |
| tch | 0.14 | PyTorch LSTM (CPU/CUDA optionnel) |
| smartcore | 0.4 | XGBoost (train + serde) |
| statrs | 0.16 | Stats (Sharpe, std) |
| aes-gcm | 0.10 | Chiffrement configs sensibles |
| tracing | 0.1 | Logging structuré |
| config | 0.13 | Parsing .env |
| ibapi | 2.10 | IB Gateway client async |

**Frontend npm :**

| Package | Version | Usage |
|---------|---------|-------|
| vue | ^3.4.0 | UI framework |
| vue-router | ^4.2.0 | Routing SPA |
| pinia | ^2.1.0 | State management |
| @tauri-apps/api | ^2 | IPC natif |
| axios | ^1.6.0 | HTTP client |
| lightweight-charts | ^4.1.0 | Charts TradingView |
| tailwindcss | ^3.4.0 | CSS utility-first |
| typescript | ^5.3.0 | Typage strict |
| vite | ^5.0.0 | Build tool |
| vitest | ^4.1.2 | Tests unitaires |
| jspdf | ^4.2.1 | Export PDF |

---

## 10. Convention de Code

### Rust — Zéro Panic en Production

```rust
// ❌ Interdit
.unwrap()
.expect()
panic!()

// ✅ Requis
use anyhow::Result;
fn calculer_signal() -> Result<Signal, TradingError> {
    let data = obtenir_donnees()?;
    Ok(Signal::from(data))
}
```

### Vue/TypeScript — Zéro Console

```typescript
// ❌ Interdit
console.log()
debugger
alert()

// ✅ Requis
try {
  const signal = await api.obtenirSignal()
  useAlertStore().afficherSucces('Signal reçu')
} catch (err) {
  useAlertStore().afficherErreur(`Échec: ${err.message}`)
}
```

### Limites de Fichiers

| Seuil | Action |
|-------|--------|
| > 250 lignes | Alerte — envisager split |
| > 300 lignes | Split obligatoire |
| Fonction > 30 lignes | Extraire sous-fonctions |
| Complexité > 10 | Simplifier |

### Séparation Frontend / Backend

**Interdit en frontend :** Calcul ATR, RSI, MACD, Fibonacci, SL/TP, R:R, scoring SMC, logique risk. Tout calcul métier est fait par le backend Rust et consommé via l'API.

**Autorisé en frontend :** Formatage affichage (`toFixed(2)`, dates), tri/filtrage liste reçue, calcul couleur selon seuil, agrégations visuelles légères.

---

## 11. Commandes Utiles

```bash
# Lancer l'application (Tauri + backend)
./scripts/run.sh

# Tests
cargo test --workspace
cd frontend && npm run test

# Qualité
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
./scripts/check-file-size.sh

# Audit sécurité
cargo audit

# Backup
./scripts/backup.sh
```

---

**Documents :** [ROADMAP.md](ROADMAP.md) | [README.md](README.md)
