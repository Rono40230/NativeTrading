# 📘 PROJET.MD - NATIVE TRADING AI

**Contexte Complet pour Agent IA & Développement Vibe Coding**

---

## 📊 RÉSUMÉ EXÉCUTIF

**Type** : Système de trading algorithmique avec IA locale  
**Objectif** : Détecter volatilité et générer signaux haute précision sur marchés financiers  
**Stack** : Rust (Backend) + Vue.js 3 (Frontend) + CUDA (ML)  
**Philosophie** : Vibe Coding (Flow + Qualité) + 100% Local (Zero Cloud)  

---

## 🎯 VISION & OBJECTIFS

> ⚠️ **Application 100% native sous Linux** — L'interface s'ouvre dans une fenêtre Tauri native. **Aucun navigateur requis, aucune URL `localhost:3000`.**

### Objectif Principal
Créer une **IA de trading locale** capable de :
1. **Analyser** 4 actifs (XAUUSD, XAGUSD, BTC, ETH) en temps réel
2. **Prédire** mouvements via modèle hybride LSTM + XGBoost
3. **Générer** signaux exploitables avec latence <10s
4. **Backtester** stratégies avec simulation réaliste
5. **Optimiser** continuellement via réentraînement automatique

### Métriques de Succès

**MVP (Phase 1 - 6 semaines)** :
- ✅ ROI backtest >0%
- ✅ Latence signaux <10s
- ✅ Dashboard fonctionnel
- ✅ 1 stratégie validée (Straddle)

**Production (Phase 2-3 - 12 semaines)** :
- ✅ ROI annualisé >15%
- ✅ Sharpe ratio >1.5
- ✅ Win rate >55%
- ✅ Max drawdown <20%
- ✅ Inférence ML <200ms

---

## 🏗️ ARCHITECTURE TECHNIQUE

### Stack Complète

**Backend (Rust)** :
- **API** : Actix-Web 4 (REST + WebSocket port 8080)
- **Data** : Binance WebSocket, MetaTrader 5, APIs multiples
- **ML** : tch-rs (PyTorch bindings), XGBoost, CUDA 11.8+
- **DB** : SQLite + SQLx (migrations, query safety)
- **Indicators** : ta-rs, statrs, ndarray (calculs numériques)
- **SMC** : 5 indicateurs custom (Order Blocks, Imbalance, IFVG, Fibonacci, Tendances)

**Frontend (Vue.js 3 + Tauri)** :
- **Framework** : Vue 3 Composition API (`<script setup>`)
- **Wrapper natif** : **Tauri** — fenêtre système Linux native (aucun navigateur)
- **State** : Pinia stores
- **Charts** : TradingView Lightweight Charts
- **Styling** : Tailwind CSS + Dark Mode natif
- **Build** : Vite + TypeScript strict
- **Communication** : Tauri IPC + WebSocket vers backend

**Infrastructure** :
- **OS** : Fedora 43 Linux
- **GPU** : NVIDIA RTX 3090 (24GB VRAM)
- **RAM** : 48 GB
- **Storage** : 50 GB SSD pour données + models

### Architecture Backend (Rust Workspace)

```
backend/
├── crates/
│   ├── api/          # Actix-Web handlers + WebSocket
│   ├── common/       # Types partagés (Candle, Asset, TradingError)
│   ├── data/         # DataProvider trait + implémentations
│   ├── db/           # SQLite layer (SQLx)
│   ├── ml/           # LSTM + XGBoost pipeline
│   ├── indicators/   # ATR, RSI, MACD, Bollinger
│   ├── smc/          # 5 indicateurs SMC custom
│   ├── strategies/   # Straddle + SMC Directionnel
│   ├── backtest/     # Backtesting engine
│   └── risk/         # Risk management (position sizing, stops)
```

**Principe DAG (Dependency Acyclic Graph)** :
```
Layer 4: API Handlers
   ↓
Layer 3: Services (strategies, ml, indicators)
   ↓
Layer 2: Data Access (db, providers)
   ↓
Layer 1: Models (common types)
```

**Règle** : Jamais d'imports horizontaux (services ne s'importent pas entre eux).

### Architecture Frontend

```
frontend/
├── src/
│   ├── views/          # 6 dashboards
│   │   ├── DashboardHome.vue      # Vue d'ensemble
│   │   ├── ChartsView.vue         # Graphiques temps réel (priorité 1)
│   │   ├── PnLView.vue            # Profit & Loss (priorité 2)
│   │   ├── HistoryView.vue        # Historique signaux (priorité 3)
│   │   ├── SettingsView.vue       # Configuration (priorité 4)
│   │   └── HeatmapView.vue        # Heatmap volatilité (priorité 5)
│   ├── components/
│   │   ├── charts/                # Composants graphiques
│   │   ├── signals/               # Affichage signaux
│   │   └── common/                # UI réutilisable
│   ├── stores/         # Pinia
│   │   ├── useSignalStore.ts
│   │   ├── useMarketDataStore.ts
│   │   ├── useSettingsStore.ts
│   │   └── useAlertStore.ts
│   ├── services/       # API calls (Axios)
│   └── router/         # Vue Router
```

---

## 🧠 MODÈLE MACHINE LEARNING

### Architecture Hybride

**LSTM (60% poids)** :
- 3 couches : 128 → 64 → 32 neurones
- Dropout 0.2 entre couches
- Entrée : Séquence 60 bougies (features normalisées)
- Sortie : Probabilités [P(up), P(down), P(neutral)]

**XGBoost (40% poids)** :
- 100 arbres, max_depth=6
- Entrée : Features instantanées (indicateurs techniques + SMC)
- Sortie : Classification ternaire

**Fusion** :
```
Score_Final = 0.6 × LSTM_Proba + 0.4 × XGBoost_Proba
```

### Features (50+ total)

**Prix OHLCV normalisés** :
- Open, High, Low, Close, Volume (5 features × 4 actifs)

**Indicateurs Techniques** (20+ features) :
- ATR (14, 21)
- RSI (14)
- MACD (12, 26, 9)
- Bollinger Bands (20, 2)
- Moving Averages (SMA 20, 50, 200)
- Volume Profile

**Indicateurs SMC** (15+ features) :
- Tendance (haussière/baissière/range)
- Order Blocks : Distance au plus proche, statut (mitigé/actif)
- Imbalance : Présence, taille (pips)
- IFVG : Présence, gap size
- Fibonacci : Niveaux actifs (38.2%, 50%, 61.8%)

### Training Pipeline

**Dataset** :
- Historique : 6 mois par actif
- Split : 80% train, 20% validation
- Walk-forward : Train 4 mois → Test 1 mois

**Optimisation** :
- Loss : Categorical Cross-Entropy
- Optimizer : Adam (lr=0.001)
- Batch size : 64
- Epochs : 50 (early stopping patience=5)
- Device : CUDA (GPU RTX 3090)

**Métriques Évaluation** :
- Accuracy globale
- F1-Score par classe
- Confusion matrix
- ROC-AUC

**Réentraînement** :
- Quotidien (nuit, via cron)
- Walk-forward sliding window

---

## 📈 STRATÉGIES DE TRADING

### 1. Stratégie Straddle (Volatilité Extrême)

**Principe** : Exploiter explosions de volatilité avec positions opposées.

**Déclencheurs** :
- ATR > 150% de moyenne mobile 14 périodes
- IA indécise : `max(P(up), P(down)) < 60%`
- Volume > 120% moyenne

**Exécution** :
- Ouvrir simultanément : LONG + SHORT
- TP : ATR × 2 chaque direction
- SL : ATR × 0.5 chaque direction
- Fermer position première à TP, laisser second runner

**Risk Management** :
- Capital risqué : 1% par direction (2% total)
- Max 1 straddle actif par actif

### 2. Stratégie SMC Directionnel (Confluence)

**Principe** : Exploiter zones SMC en confluence avec tendance et IA.

**Scoring Confluence (0-100)** :
```
Score = Tendance (20pts)
      + IA confidence (30pts)
      + Order Block (20pts)
      + Imbalance (15pts)
      + IFVG (10pts)
      + Fibonacci (5pts)
```

**Déclencheurs** :
- Score ≥ 70/100
- Direction alignée : Tendance + IA + OB

**Exécution** :
- Direction : LONG ou SHORT
- Entry : Au touch Order Block + confirmation
- TP pyramidal : TP1 (ATR × 1), TP2 (ATR × 2), TP3 (ATR × 3)
- SL : Sous/sur Order Block (ATR × 0.8)

**Risk Management** :
- Capital risqué : 1.5% par trade
- Max 2 positions SMC simultanées

---

## 🔒 RISK MANAGEMENT (CRITIQUE)

### Règles Absolues

**Limites Globales** :
- **Max Risk par Trade** : 2% capital
- **Max Positions Simultanées** : 3 total
- **Max Exposition par Actif** : 25% capital
- **Max Drawdown** : 20% (arrêt trading auto)

**Calcul Position Size** :
```rust
position_size = (capital × risk_pct) / (entry_price - stop_loss)
```

**Vérifications Pré-Trade** :
1. Position size respecte 2% capital ?
2. Nombre positions < 3 ?
3. Exposition actif < 25% ?
4. Drawdown actuel < 20% ?
5. Stop-loss valide et calculé ?

**Si UNE seule échoue** → Rejet signal.

### Monitoring Continu

- Tracking drawdown temps réel
- Alerte si approche 15%
- Stop automatique à 20%
- Logs toutes décisions risk management

---

## 📊 INDICATEURS SMC (Smart Money Concept)

### 1. Tendances
**Détection** : Structure HH/HL (haussier) ou LH/LL (baissier)  
**Scoring** : Clarté tendance (0-20 points)

### 2. Order Blocks (Priorité 1)
**Définition** : Dernière bougie avant mouvement impulsif  
**Validation** : Volume > moyenne, taille > ATR × 0.5  
**États** : Actif, Mitigé (prix traversé), Invalidé

### 3. Imbalance (Priorité 2)
**Définition** : Gap entre high[i] et low[i-1] (ou inverse)  
**Validation** : Gap ≥ 3 pips + pas de retrace  
**Usage** : Zone retour prix attendu

### 4. IFVG - Inversion Fair Value Gap (Priorité 3)
**Définition** : Gap + break of structure  
**Validation** : IFVG rempli à 50% minimum  
**Usage** : Zone entry après pullback

### 5. Fibonacci (Priorité 4)
**Niveaux** : 38.2%, 50%, 61.8%, 78.6%  
**Référence** : Dernier swing high/low significatif  
**Usage** : Confluence avec OB/Imbalance

---

## 🗄️ BASE DE DONNÉES (SQLite)

### Schema Principales Tables

**`candles`** :
```sql
CREATE TABLE candles (
    id INTEGER PRIMARY KEY,
    asset TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    UNIQUE(asset, timeframe, timestamp)
);
CREATE INDEX idx_candles ON candles(asset, timeframe, timestamp);
```

**`indicators`** :
```sql
CREATE TABLE indicators (
    id INTEGER PRIMARY KEY,
    candle_id INTEGER NOT NULL,
    atr REAL,
    rsi REAL,
    macd REAL,
    macd_signal REAL,
    bb_upper REAL,
    bb_middle REAL,
    bb_lower REAL,
    FOREIGN KEY(candle_id) REFERENCES candles(id)
);
```

**`smc_indicators`** :
```sql
CREATE TABLE smc_indicators (
    id INTEGER PRIMARY KEY,
    candle_id INTEGER NOT NULL,
    tendance TEXT, -- 'haussier', 'baissier', 'range'
    order_block_id INTEGER,
    imbalance_id INTEGER,
    ifvg_id INTEGER,
    fib_level REAL,
    FOREIGN KEY(candle_id) REFERENCES candles(id)
);
```

**`signals`** :
```sql
CREATE TABLE signals (
    id INTEGER PRIMARY KEY,
    asset TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    strategy TEXT NOT NULL, -- 'straddle', 'smc'
    direction TEXT NOT NULL, -- 'long', 'short', 'both'
    entry_price REAL NOT NULL,
    tp REAL NOT NULL,
    sl REAL NOT NULL,
    confidence REAL NOT NULL,
    score INTEGER, -- Pour SMC scoring
    status TEXT DEFAULT 'pending' -- 'pending', 'executed', 'cancelled'
);
```

**`trades`** :
```sql
CREATE TABLE trades (
    id INTEGER PRIMARY KEY,
    signal_id INTEGER NOT NULL,
    open_timestamp TIMESTAMP NOT NULL,
    close_timestamp TIMESTAMP,
    open_price REAL NOT NULL,
    close_price REAL,
    quantity REAL NOT NULL,
    pnl REAL,
    status TEXT DEFAULT 'open', -- 'open', 'closed'
    FOREIGN KEY(signal_id) REFERENCES signals(id)
);
```

**`models`** :
```sql
CREATE TABLE models (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    path TEXT NOT NULL,
    trained_at TIMESTAMP NOT NULL,
    metrics JSON, -- Accuracy, F1, etc.
    active BOOLEAN DEFAULT 0
);
```

---

## 🌐 DATA SOURCES

### Binance (BTC, ETH)
**API** : WebSocket REST  
**Endpoint** : `wss://stream.binance.com:9443/ws`  
**Data** : Klines (OHLCV) temps réel  
**Latence** : ~100-300ms

### MetaTrader 5 (XAUUSD, XAGUSD)
**Broker** : AXI  
**API** : MT5 Python API via bridge Rust  
**Data** : Ticks + Klines  
**Latence** : ~500ms-1s

### Fallback APIs
- CoinGecko (crypto)
- Alpha Vantage (métaux)
- Yahoo Finance (backup)

---

## 🚦 WORKFLOW DÉVELOPPEMENT VIBE CODING

### Phase 1 : CRÉATION (Accumulation)
**Durée** : Variable (heures, jours)  
**Actions** :
- Coder, itérer, refactorer
- Tests locaux : `cargo test`, `npm run test`
- Sentinelle active : Formatage/linting auto
- **PAS de commit**

**Outils** :
- `.vibe/bin/sentinel.sh` (tourne en background)
- IDE alerts en temps réel

### Phase 2 : VALIDATION (Audit)
**Déclencheur** : Utilisateur dit "Valide", "Ready", "Audit"  
**Actions** :
1. Lancer `.vibe/bin/audit.sh`
2. Résultats :
   - ✅ VERT → Passer Phase 3
   - 🔴 ROUGE → Corriger erreurs, relancer
3. Vérifications :
   - Tests passent (Rust + Vue)
   - Pas de `unwrap()`, `console.log()`
   - Limites fichiers respectées (<300 lignes)
   - Complexité <10
   - Security audit OK

**Script** : `./scripts/validate-phase2.sh` (ou `.vibe/bin/audit.sh`)

### Phase 3 : COMMIT (Production)
**Condition** : Audit = ✅ APPROUVÉ uniquement  
**Actions** :
1. Impact detection : `./scripts/impact-detection/validate-phase2.sh`
2. Commit avec message conventionnel
3. Push si nécessaire

**Format Message** :
```
feat(module): description courte

- Détail 1
- Détail 2

Refs: #123
```

---

## 📜 RÈGLES CRITIQUES (RAPPEL)

### 1. Error Handling (ZERO PANIC)
```rust
// ❌ INTERDIT
.unwrap()
.expect()
panic!()

// ✅ REQUIS
use anyhow::Result;
fn ma_fonction() -> Result<T> {
    let data = obtenir()?;
    Ok(data)
}
```

### 2. File Size Limits
- Rust Service/Module : <300 lignes
- Vue Component : <250 lignes
- Fonction : <30 lignes
- Complexité : <10

**Si dépassement** → Split immédiat.

### 3. DAG Architecture
```
Commands (L4) → Services (L3) → DB/Providers (L2) → Models (L1)
```
**JAMAIS** d'imports horizontaux entre services.

### 4. Naming Convention
**Métier en français** :
- `calculer_volatilite()`, `detecter_signal()`, `prix_ouverture`

**APIs standard en anglais OK** :
- `parse()`, `serialize()`, `request()`

### 5. Performance
- Inférence ML : <200ms
- Signal latency : <10s
- DB queries : <100ms
- UI : 60fps

---

## 🛠️ SCRIPTS UTILES

```bash
# Installation
./scripts/install.sh

# Lancer app
./scripts/run.sh

# Tests
./scripts/test.sh
cargo test --workspace
cd frontend && npm run test

# Audit complet
./.vibe/bin/audit.sh

# Backup données
./scripts/backup.sh

# Vérifier taille fichiers
./scripts/check-file-size.sh

# Clippy Rust
cargo clippy --workspace -- -D warnings

# Format Rust
cargo fmt --all --check
```

---

## 📚 RÉFÉRENCES DOCUMENTATION

**Interne** :
- `.clinerules` : Règles absolues métier
- `.vibe/config.toml` : Configuration technique
- `CAHIER_DES_CHARGES.md` : Spécifications complètes
- `ARCHITECTURE.md` : Architecture détaillée
- `ROADMAP.md` : Planning 18 semaines
- `README.md` : Installation & usage

**Externe** :
- Rust: https://doc.rust-lang.org/
- Actix-Web: https://actix.rs/
- Vue.js 3: https://vuejs.org/
- TradingView Charts: https://tradingview.github.io/lightweight-charts/
- Polars: https://pola-rs.github.io/polars/
- tch-rs: https://github.com/LaurentMazare/tch-rs

---

## 🎯 PRIORITÉS ACTUELLES (Phase MVP)

**Semaine 1-2** : Fondations ✅
- [x] Setup projet Rust + Vue
- [x] Configuration Vibe Coding
- [ ] Acquisition données Binance WebSocket
- [ ] Schema SQLite + migrations

**Semaine 3** : ML Pipeline
- [ ] Feature extraction (OHLCV + indicateurs)
- [ ] Training XGBoost classification binaire
- [ ] Inférence <200ms validation

**Semaine 4** : Stratégie Straddle
- [ ] Détection volatilité extrême
- [ ] Logique positions opposées
- [ ] Risk management basic

**Semaine 5-6** : Dashboard & Tests
- [ ] ChartView temps réel TradingView
- [ ] Signaux affichage live
- [ ] Backtesting engine validation
- [ ] Tests intégration complets

---

**STATUS** : 🟢 Prêt pour développement  
**DERNIÈRE MAJ** : 10 mars 2026  
**VERSION** : 0.1.0 MVP
