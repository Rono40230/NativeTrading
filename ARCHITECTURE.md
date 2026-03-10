# 🏛️ ARCHITECTURE TECHNIQUE

**Système:** Native Trading AI | **Stack:** Rust + Vue.js 3 | **Date:** 9 mars 2026

---

## Vue d'ensemble

> ⚠️ **Application 100% native** — Aucun navigateur. L'interface s'ouvre dans une fenêtre native via **Tauri**.

```
┌──────────────────────────────────────────────┐
│   Fenêtre Native Tauri (aucun navigateur)    │
│      Vue.js 3 + TS (UI embarquée)            │
│   Dashboards │ Charts │ Configuration        │
└──────────────────────────────────────────────┘
            ↕ Tauri IPC + WebSocket interne
┌──────────────────────────────────────────────┐
│       Backend Rust (Actix-Web)               │
│  Data → AI → Strategies → Risk Management    │
│  Port 8080 — interne uniquement (no browser) │
└──────────────────────────────────────────────┘
            ↕
┌──────────────────────────────────────────────┐
│       SQLite + ML Models + CUDA              │
└──────────────────────────────────────────────┘
```

---

## Backend Rust - Architecture modulaire

```
backend/
├── Cargo.toml (workspace)
└── crates/
    ├── api/          # Actix-Web REST + WebSocket
    ├── data/         # Acquisition (Binance, MT5, APIs)
    ├── ml/           # PyTorch (tch-rs) + XGBoost
    ├── indicators/   # ATR, RSI, MACD, Bollinger
    ├── smc/          # Order Blocks, Imbalance, IFVG, Fib
    ├── strategies/   # Straddle + SMC Directionnel
    ├── backtest/     # Backtesting engine
    ├── risk/         # Risk management
    ├── db/           # SQLite layer
    └── common/       # Types, erreurs partagés
```

### Modules clés

**API (Actix-Web):**
- Port 8080 — **interne uniquement**, non exposé au navigateur
- Endpoints: `/api/*`
- WebSocket: `/api/stream`
- CORS désactivé en production (Tauri = même origine)

**Data Acquisition:**
- Trait `DataProvider` générique
- Implémentations: BinanceProvider, MT5Provider
- Websocket temps réel + polling fallback
- Reconnexion auto

**ML Engine:**
- LSTM (3 couches) + XGBoost
- Device: CUDA (RTX 3090)
- Outputs: Binaire, Ternaire, Régression, Confidence
- Inférence <150ms

**Strategies:**
- Trait `Strategy`
- Straddle: ATR >150% + IA indécis
- SMC: Scoring confluence ≥70/100

**Backtesting:**
- Simulation bougie-par-bougie
- Spreads + slippage réalistes
- Métriques: ROI, Sharpe, DD, Win rate

---

## Frontend Vue.js 3

```
frontend/
├── src/
│   ├── views/         # 5 dashboards
│   │   ├── DashboardHome.vue
│   │   ├── ChartsView.vue
│   │   ├── PnLView.vue
│   │   ├── HistoryView.vue
│   │   ├── SettingsView.vue
│   │   └── HeatmapView.vue
│   ├── components/    # Composants réutilisables
│   ├── stores/        # Pinia state management
│   ├── services/      # API calls (Axios)
│   └── router/        # Vue Router
├── vite.config.ts
└── package.json
```

**Tech:**
- **Tauri** — wrapper natif Linux (fenêtre native, pas de navigateur)
- Vue 3 Composition API
- TypeScript
- Tailwind CSS
- TradingView Lightweight Charts
- Axios + WebSocket (interne vers backend port 8080)

---

## Base de données SQLite

**Tables principales:**
- `candles` - OHLCV historique (6 mois × 4 actifs × 3 timeframes)
- `indicators` - ATR, Bollinger, RSI, etc.
- `smc_indicators` - OB, Imbalance, Fibonacci
- `signals` - Signaux générés
- `trades` - Trades simulés/réels
- `models` - Métadata modèles ML
- `settings` - Configuration système
- `api_keys` - Clés chiffrées AES-256

**Indexation:**
```sql
CREATE INDEX idx_candles ON candles(asset, timeframe, timestamp);
```

---

## Machine Learning Pipeline

1. **Feature Extraction**
   - OHLC normalisés
   - Indicateurs techniques (50+ features)
   - Indicateurs SMC (proximité, statuts)

2. **Training**
   - Walk-forward: Train 4 mois → Test 1 mois
   - Batch size: 64
   - Epochs: 50 (early stopping)
   - Device: CUDA

3. **Inference**
   - Load model au démarrage
   - Prediction pipeline: <200ms
   - GPU accélération

4. **Outputs**
   - Classification: [P(up), P(down)]
   - Régression: Prix T+5min, T+15min, T+1h
   - Confidence: 0-100%

---

## Communications

**REST API:**
```
GET  /api/assets
GET  /api/candles/:asset/:timeframe
POST /api/predict
GET  /api/signals
POST /api/backtest/run
GET  /api/settings
PUT  /api/settings
```

**WebSocket:**
```json
{
  "type": "candle|prediction|signal",
  "asset": "BTC",
  "data": { ... }
}
```

---

## Performance

| Opération | Cible | Max acceptable |
|-----------|-------|----------------|
| Inférence IA | <150ms | <300ms |
| API latency | <50ms | <200ms |
| WebSocket | <20ms | <100ms |
| Backtest 6 mois | <30s | <2min |
| Training | <1h | <2h |

**Optimisations:**
- Release mode: `lto = true`, `opt-level = 3`
- Parallel processing (rayon)
- SIMD pour calculs vectoriels
- GPU CUDA pour ML

---

## Sécurité

- **Chiffrement:** AES-256 pour clés API
- **Auth:** Aucune (localhost only)
- **Logs:** Minimal, pas de données sensibles
- **Backup:** Hebdomadaire auto

---

## Déploiement

**Installation:**
```bash
./scripts/install.sh  # Rust, CUDA, Node.js (Tauri), LibTorch
```

**Lancement:**
```bash
./scripts/run.sh      # Lance fenêtre native Tauri
```

**Accès:**
- 🖥️ Fenêtre native Tauri (aucun navigateur requis)
- API interne: http://localhost:8080 (backend uniquement, pas pour navigateur)

---

**Documents:** [CAHIER_DES_CHARGES.md](CAHIER_DES_CHARGES.md) | [ROADMAP.md](ROADMAP.md) | [README.md](README.md)
