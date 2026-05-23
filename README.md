# Native Trading AI

[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/Vue.js-3.4+-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2-blue.svg)](https://tauri.app/)
[![CUDA](https://img.shields.io/badge/CUDA-11.8+-76B900.svg)](https://developer.nvidia.com/cuda-toolkit)

Application de trading algorithmique **100 % locale** — aucun cloud, aucun abonnement.  
Elle surveille les marchés en continu, détecte les setups, génère des signaux et permet de backtester les stratégies, le tout depuis une fenêtre native (Tauri).

---

## Actifs suivis

| Actif | Source |
|-------|--------|
| BTC/USDT, ETH/USDT | Binance WebSocket |
| XAUUSD, XAGUSD | MetaTrader 5 |

Timeframes analysés : **M1 · M5 · M15**

---

## 3 stratégies de trading

### 1. Straddle — Volatilité extrême
Déclenché quand l'ATR dépasse 150 % de sa moyenne et que le modèle IA est indécis.  
L'app ouvre deux positions opposées simultanément (Long + Short) pour capturer le mouvement dans n'importe quelle direction.

- Risk : 1 % par jambe (2 % total)
- TP : 2 × ATR · SL : 0,5 × ATR
- Machine à états : suivi indépendant de chaque jambe jusqu'à clôture

### 2. SMC Directionnel — Confluence Smart Money
Score de 0 à 100 calculé à partir de 6 composantes : tendance structurelle, Order Block, Imbalance, IFVG, Fibonacci, Kill Zone.  
Un signal est émis uniquement si le score ≥ 70/100.

- Risk : 1,5 % par trade
- Take-profits pyramidaux : TP1 / TP2 / TP3 trailing

### 3. Rockets — Contraction de volatilité (VCP)
Détecte les compressions de range (ATR actuel < 60 % de l'ATR moyen 20 périodes) suivies d'un breakout au-dessus du plus-haut des 10 dernières bougies.

- Risk : configurable par trade
- SL : dernier bas des 5 bougies · TP : 2 × (entrée − SL)

---

## Intelligence artificielle embarquée

**Modèle hybride LSTM + XGBoost** entraîné et exécuté localement sur GPU (RTX 3090).

| Modèle | Rôle | Poids |
|--------|------|-------|
| LSTM 3 couches (128→64→32) | Patterns temporels sur 60 bougies | 60 % |
| XGBoost 100 arbres | Classification rapide sur 50+ features | 40 % |

Features : OHLCV normalisé, ATR, RSI, MACD, Bollinger, tendance SMC, Order Blocks, Imbalance, IFVG, Fibonacci.  
Inférence < 200 ms · Réentraînement automatique planifiable.

**Coach IA** : analyse de graphiques importés par l'utilisateur (vision Claude / Ollama) avec commentaire en langage naturel.

---

## Interface — vues disponibles

| Vue | Contenu |
|-----|---------|
| **Dashboard** | État global du système, positions actives, alertes |
| **SMC — Graphiques** | Charts TradingView temps réel + indicateurs SMC superposés |
| **SMC — Signaux** | Liste des signaux actifs et historique SMC |
| **Straddle — Live** | Positions Straddle en cours, machine à états par jambe |
| **Straddle — Signaux** | Historique et scores des setups Straddle |
| **Straddle — Backtest** | Lancement backtest, résultats, courbe equity, stats par heure/jour |
| **Rockets** | Signaux VCP actifs et historique |
| **Heatmap** | Volatilité comparée multi-actifs / multi-timeframes |
| **ML Insights** | Métriques du modèle, accuracy, feature importance |
| **IA — Analyse chart** | Import d'un screenshot → analyse IA visuelle |
| **IA — Coach SMC** | Chat avec l'IA pour comprendre un setup ou affiner une stratégie |
| **Historique** | Tous les trades passés, filtres, export |
| **Gestion des données** | Import CSV, backfill, état de la base SQLite |
| **Paramètres** | Clés API, seuils de risque, configuration par stratégie |
| **Prompts IA** | Édition des prompts Ollama utilisés par l'app |
| **Lexique** | Définitions SMC, Straddle, Rockets pour référence rapide |

---

## Prérequis

- **OS :** Linux (Fedora 43+)
- **GPU :** NVIDIA avec CUDA 11.8+ (RTX 3090 recommandée)
- **RAM :** 16 GB minimum, 32 GB recommandé
- Rust 1.82+ · Node.js 20+ · LibTorch · SQLite 3

---

## Démarrage

```bash
# Installation (première fois — 15-20 min)
chmod +x scripts/*.sh
./scripts/install.sh

# Lancer l'application
./scripts/run.sh
```

Une fenêtre native s'ouvre. Aucun navigateur nécessaire.  
Le backend écoute sur `localhost:8080` (usage interne Tauri uniquement).

```bash
# Tests
cargo test --workspace

# Backup des données
./scripts/backup.sh
```

---

## Architecture

```
backend/crates/
  api/          REST + WebSocket (port 8080)
  common/       Types partagés (Candle, Asset, Direction…)
  data/         Connecteurs Binance / MetaTrader 5
  db/           Couche SQLite (SQLx)
  ml/           Pipeline LSTM + XGBoost (tch-rs + xgboost)
  indicators/   ATR, RSI, MACD, Bollinger…
  smc/          5 indicateurs SMC (OB, Imbalance, IFVG, Fib, Tendance)
  strategies/   Straddle · SMC · Rockets
  backtest/     Moteur de backtesting
  risk/         Position sizing, drawdown, garde-fous

frontend/src/
  views/        20 pages Vue 3
  stores/       État global Pinia
  services/     Clients API (types générés automatiquement depuis Rust via ts-rs)
  composables/  Logique réutilisable (charts, indicateurs…)
```

---

## Métriques cibles

| Métrique | Cible |
|----------|-------|
| ROI annualisé | > 15 % |
| Sharpe ratio | > 1,5 |
| Win rate | > 55 % |
| Drawdown max | < 20 % |
| Inférence ML | < 200 ms |

---

## ⚠️ Avertissement

**Système éducatif/expérimental**
- Pas de conseil financier
- Trading = risque de perte totale
- Backtests ≠ résultats futurs
- Paper trading recommandé avant capital réel

---

## 🎯 Vision

**Objectifs finaux:**
- ROI >20%, Sharpe >2, Max DD <15%
- Support multi-actifs
- Trading auto 24/7
- Dashboard mobile

---

**Créé avec ❤️ et 🤖 Copilot - Mars 2026**
