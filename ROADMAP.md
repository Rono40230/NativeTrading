# 🗺️ ROADMAP - NATIVE TRADING AI

**Durée totale:** 18 semaines | **Version:** 1.0 | **Dernière mise à jour:** 10 mars 2026

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

## 🏗️ PHASE 2 - PRODUCTION (Semaines 7-16)

> **Statut : 🔵 EN COURS** — Démarrage semaine 7

### Semaine 7-8: MT5 + Métaux
- [ ] Intégration MetaTrader 5
- [ ] XAUUSD + XAGUSD historique
- [ ] Multi-sources données

### Semaine 9-10: Indicateurs SMC
- [ ] Tendances, Order Blocks, Imbalance, IFVG, Fibonacci
- [ ] Réimplémentation Rust depuis ex5 (crate `smc` déjà scaffoldé)
- [ ] Tests validation

### Semaine 11-12: IA Hybride
- [ ] LSTM 3 couches (128-64-32) via tch-rs/CUDA RTX 3090
- [ ] Fusion LSTM 60% + RandomForest 40%
- [ ] Training GPU + serialisation modèle
- [ ] Endpoint `POST /api/ml/train` + bouton Dashboard

### Semaine 13-14: SMC Directionnel
- [ ] Scoring confluence (seuil 70/100) (`strategies/src/smc_directional.rs` scaffoldé)
- [ ] TP pyramidal (TP1/TP2/TP3)
- [ ] Backtests comparatifs straddle vs SMC

### Semaine 15-16: UI complète
- [ ] 5 dashboards complets (Charts ✅, P&L, History, Settings, Heatmap)
- [ ] Système alertes combiné
- [ ] Export PDF/CSV

### Semaine 16b: SMC IA — Analyste & Coach (Claude API)
**Objectif:** Intégrer "Le petit robo" comme outil d'assistance SMC

**Backend Rust (`api` crate) :**
- Proxy sécurisé `POST /api/smc/analyze` — image → analyse SMC complète
- Proxy sécurisé `POST /api/smc/chat` — chat multi-turn Coach SMC
- Module `AnthropicClient` (clé API dans `.env`, jamais exposée)
- Rate limiting sur endpoints `/api/smc/*`

**Frontend Vue.js 3 :**
- `SMCAnalyzerView.vue` — upload screenshot + analyse institutionnelle
- `SMCCoachView.vue` — chat + diagrammes HTML interactifs animés
- `DiagramFrame.vue` — WebView sandboxée Tauri pour diagrammes HTML générés (dans la fenêtre native)
- `useSMCStore.ts` (Pinia) + `smc.service.ts`
- Ajout dans NavBar + Router

**Prompts embarqués :**
- `ANALYST_PROMPT` — analyse Structure, Liquidité, POI, Scénarios, Confluence
- `COACH_PROMPT` — pédagogie SMC + génération diagrammes HTML animés

**Validation :**
- Analyse screenshot → réponse structurée <10s
- Coach génère diagrammes Order Block, FVG, BOS/ChoCH
- Aucune clé API exposée côté frontend

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
