# 🗺️ ROADMAP - NATIVE TRADING AI

**Durée totale:** 18 semaines | **Version:** 2.0 | **Dernière mise à jour:** 10 mars 2026

---

## ✅ PHASE 1 - MVP — TERMINÉE (Semaines 1-6)

> **Statut : 🟢 COMPLÉTÉE** — App native validée le 10 mars 2026 (screenshot confirmé)

**Objectif:** Valider le concept avec 1 stratégie fonctionnelle

### ✅ Semaine 1-2: Fondations
- [x] Setup environnement (Rust, Node.js 22, **Tauri CLI**)
- [x] Structure projet — workspace 10 crates Rust + Vue.js 3 Tauri
- [x] App native Tauri — **aucun navigateur**, fenêtre GDK_BACKEND=x11
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
- [x] Dashboard : KPIs BTC/ETH temps réel, badge ML, statut système, tableau signaux
- [x] ChartsView : graphique chandeliers sélecteur asset/timeframe
- [x] Pinia stores : `market.store.ts`, `signal.store.ts`
- [x] `api.service.ts` Axios vers backend port 8080
- [x] Script `run.sh` : compile → health check → Tauri natif
- [x] 10 tests unitaires (backtest×2, indicators×4, risk×4)

**✅ Métriques MVP validées (live 10 mars 2026):**
- BTC/USDT **$71,240** | ETH/USDT **$2,069** (Binance temps réel)
- Backend API 🟢 Online | Binance Feed 🟢 Connecté | SQLite 🟢 Online
- ML Engine 🟡 Non entraîné (données historiques nécessaires)

---

## ✅ PHASE 2 - PRODUCTION (Semaines 7-16)

> **Statut : 🟢 COMPLÉTÉE** — Terminée le 10 mars 2026

### ✅ Semaine 7-8: Twelvedata + Métaux/Forex
- [x] Intégration Twelvedata REST API (XAUUSD, XAGUSD)
- [x] `TwelvedataProvider` — crate `data`, trait `DataProvider`
- [x] Routage automatique : BTC/ETH → Binance | XAUUSD/XAGUSD → Twelvedata
- [x] Clé API stockée en SQLite (`configuration`) + modifiable depuis ⚙️ Paramètres
- [x] Bouton "Tester connexion" dans SettingsView
- [x] Charts + Heatmap : XAUUSD et XAGUSD ajoutés

### ✅ Semaine 9-10: Indicateurs SMC
- [x] Tendances HH/HL/LH/LL (`smc/src/tendances.rs`)
- [x] Order Blocks — last candle avant impulsion (`smc/src/order_blocks.rs`)
- [x] Imbalance / FVG 3-candles (`smc/src/imbalance.rs`)
- [x] IFVG — FVG + mitigation + BOS (`smc/src/ifvg.rs`)
- [x] Fibonacci — niveaux 23.6/38.2/50/61.8/78.6 (`smc/src/fibonacci.rs`)
- [x] Façade `ScoreSmc` — scoring 100pts, seuil 70 (`smc/src/lib.rs`)
- [x] Tests : tendances + fibonacci

### ✅ Semaine 11-12: IA Hybride
- [x] LSTM 3 couches (128-64-32) pure Rust — `ml/src/lstm.rs`
- [x] Fusion LSTM 60% + RandomForest 40% — `PipelineHybride`
- [x] Entraînement SGD + sérialisation serde_json
- [x] `POST /api/ml/train` + `GET /api/ml/status` + bouton Dashboard

### ✅ Semaine 13-14: SMC Directionnel
- [x] `SmcDirectionalStrategy` — scoring confluence ≥70 (`strategies/src/smc_directional.rs`)
- [x] ATR-based TP1 (×1.5) / TP2 (×3.0) / TP3 (×5.0) / SL (×1.0)
- [x] `take_profit_2` + `take_profit_3` dans `strategies::Signal`
- [x] Endpoint `GET /api/smc/analyse` (`api/src/smc_handlers.rs`)
- [x] Dashboard — bloc score SMC + 5 composants + barre progression
- [x] `SmcScoreCard.vue` composant extrait

### ✅ Semaine 15-16: UI complète
- [x] `PnLView.vue` — courbe equity, ROI/Sharpe/WinRate/Drawdown/ProfitFactor
- [x] `HistoryView.vue` — tableau signaux filtrables (asset/direction/stratégie), pagination 8/page
- [x] `HeatmapView.vue` — grille ATR volatilité (BTC+ETH × M5/M15/H1/H4/D1), refresh 60s
- [x] `AlerteStore` + `ToastAlerte.vue` — système de notifications global
- [x] `GET /api/signaux/export` — export CSV

### ✅ Semaine 16b: SMC IA — Analyste & Coach (Ollama local)
**Implémenté avec Ollama local (zéro frais, zéro données externes) — modèle `qwen2.5:14b`**

**Backend Rust (`api` crate) :**
- [x] `ollama.rs` — client HTTP vers `localhost:11434`, configurable via `.env`
- [x] `POST /api/ia/analyse` — analyse narrative signal SMC
- [x] `POST /api/ia/chat` — coach conversationnel multi-turn (max 40 messages)
- [x] `GET /api/ia/status` — vérifie disponibilité Ollama

**Frontend Vue.js 3 :**
- [x] `SMCAnalyzerView.vue` — formulaire signal + curseurs SMC + analyse IA
- [x] `SMCCoachView.vue` — chat avec questions rapides, historique scrollable
- [x] NavBar + Router mis à jour

**Infrastructure :**
- [x] Ollama installé + service systemd actif
- [x] Modèle `qwen2.5:14b` (9 Go) sur `/run/media/rono/IA/ollama/models/`
- [x] RTX 3090 utilisée pour l'inférence GPU locale

---

## ⚙️ PHASE 3 - OPTIMISATION (Semaines 17-18)

> **Statut : ⚪ À VENIR**

### Semaine 17: Auto-training
- [ ] Réentraînement quotidien
- [ ] Walk-forward optimization
- [ ] Monitoring dérive

### Semaine 18: Tests + Docs
- [ ] Coverage >80%
- [ ] Documentation complète
- [ ] Scripts automatisés

---

## 🚀 PHASE 4 - PRODUCTION RÉELLE

> **Statut : ⚪ À VENIR**

- [ ] Paper trading validation (2-4 semaines)
- [ ] Trading réel progressif (capital limité)
- [ ] Monitoring 24/7
- [ ] Itérations continues

---

## 📈 MÉTRIQUES CIBLES

| Phase | ROI | Sharpe | Max DD | Win Rate | Latence |
|-------|-----|--------|--------|----------|---------|
| MVP | >0% | >0.5 | <30% | >45% | <10s |
| Prod | >15% | >1.5 | <20% | >55% | <5s |

---

**Prochaine étape:** [ARCHITECTURE.md](ARCHITECTURE.md) pour détails techniques
