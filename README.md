# 📊 Native Trading AI

> Système d'intelligence artificielle native pour trading algorithmique

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/Vue.js-3.4+-green.svg)](https://vuejs.org/)
[![CUDA](https://img.shields.io/badge/CUDA-11.8+-76B900.svg)](https://developer.nvidia.com/cuda-toolkit)

---

## 🎯 Objectif

IA locale pour détecter volatilité et générer signaux de trading sur **XAUUSD, XAGUSD, BTC, ETH** avec :
- **Modèle hybride** LSTM + XGBoost
- **Indicateurs SMC** (Order Blocks, Imbalance, IFVG, Fibonacci)
- **2 stratégies** : Straddle + SMC Directionnel
- **Backtesting** avec métriques avancées
- **Dashboard** Vue.js temps réel

**100% local** - GPU CUDA - Aucune dépendance cloud

---

## 📋 Documentation

| Document | Description |
|----------|-------------|
| **[CAHIER_DES_CHARGES.md](CAHIER_DES_CHARGES.md)** | Spécifications complètes |
| **[ROADMAP.md](ROADMAP.md)** | Feuille de route (18 semaines) |
| **[ARCHITECTURE.md](ARCHITECTURE.md)** | Architecture technique |
| **README.md** | Ce document |

---

## ⚙️ Prérequis

- **OS:** Fedora 43 (Linux)
- **GPU:** NVIDIA RTX 3090 (CUDA 11.8+)
- **RAM:** 48 GB
- **Disque:** ~50 GB libres

**Logiciels** (installés automatiquement):
- Rust 1.70+
- **Tauri CLI** (fenêtre native — aucun navigateur)
- CUDA Toolkit 11.8
- LibTorch (PyTorch C++)
- Node.js 20+ (build Tauri uniquement)
- SQLite 3

---

## 🚀 Installation

```bash
# 1. Rendre les scripts exécutables
chmod +x scripts/*.sh

# 2. Lancer l'installation (15-20 min)
./scripts/install.sh

# 3. Configuration
cp .env.example .env
nano .env  # Ajouter vos clés API
```

---

## 🎮 Démarrage

```bash
# Lancer l'application
./scripts/run.sh
```

**Résultat:**
- 🖥️ Une fenêtre native s'ouvre (aucun navigateur)
- 🔌 API interne: http://localhost:8080 (backend, usage Tauri uniquement)
- 📡 WebSocket: ws://localhost:8080/api/stream

**Arrêter:** `Ctrl+C`

---

## 📊 Dashboards

1. **Home** - Vue d'ensemble système
2. **Charts** (priorité 1) - Graphiques 4 actifs temps réel + indicateurs
3. **P&L** (priorité 2) - Profit & Loss live, positions
4. **History** (priorité 3) - Historique signaux/trades
5. **Settings** (priorité 4) - Configuration paramètres
6. **Heatmap** (priorité 5) - Volatilité multi-actifs

---

## 🏗️ Structure

```
native-trading-ai/
├── backend/                # Rust workspace (9 crates)
│   ├── crates/
│   │   ├── api/           # REST API + WebSocket
│   │   ├── data/          # Acquisition données
│   │   ├── ml/            # Machine Learning
│   │   ├── indicators/    # Indicateurs techniques
│   │   ├── smc/           # Indicateurs SMC
│   │   ├── strategies/    # Stratégies trading
│   │   ├── backtest/      # Backtesting
│   │   ├── risk/          # Risk management
│   │   ├── db/            # SQLite
│   │   └── common/        # Types communs
│   └── models/            # Modèles ML (.pt)
├── frontend/              # Vue.js 3 + TypeScript
├── data/                  # SQLite + backups
├── scripts/               # install.sh, run.sh, backup.sh
├── CAHIER_DES_CHARGES.md
├── ROADMAP.md
├── ARCHITECTURE.md
└── README.md
```

---

## 🤖 Intelligence Artificielle

**Modèle hybride:**
- **LSTM:** 3 couches (128-64-32) - Patterns temporels
- **XGBoost:** 200 trees - Classification rapide
- **Fusion:** Voting pondéré 60/40

**Outputs:**
- Classification binaire/ternaire
- Régression multi-horizon (T+5min, T+15min, T+1h)
- Confidence score 0-100%

**Training:**
- Walk-forward validation
- GPU CUDA <1h
- Réentraînement quotidien auto (Phase 3)

---

## 📈 Stratégies

### 1. Straddle (Volatilité extrême)
- **Conditions:** ATR >150% + IA indécis
- **Exécution:** Long + Short simultanés
- **SL:** 20 pips / 0.5%
- **TP:** Trailing stop 2× ATR

### 2. SMC Directionnel (Confluence)
- **Scoring:** IA 35% + Tendance 20% + OB 15% + ...
- **Seuil:** ≥70/100
- **TP pyramidal:** TP1 (40%) + TP2 (40%) + TP3 trailing (20%)

---

## 🧪 Tests

```bash
./scripts/test.sh
```

---

## 💾 Backup

```bash
# Backup manuel
./scripts/backup.sh

# Automatiser (cron hebdo)
crontab -e
# 0 2 * * 0 /path/to/scripts/backup.sh
```

---

## 📈 Métriques cibles

| Métrique | MVP | Production |
|----------|-----|------------|
| ROI 1 an | >0% | >15% |
| Sharpe | >0.5 | >1.5 |
| Max DD | <30% | <20% |
| Win rate | >45% | >55% |

---

## 🔧 Configuration

**Backend:** `.env`
```bash
DATABASE_URL=sqlite:./data/trading.db
RUST_LOG=info
CUDA_VISIBLE_DEVICES=0
```

**Frontend:** `frontend/vite.config.ts`
```typescript
server: {
  port: 3000,
  proxy: { '/api': 'http://localhost:8080' }
}
```

---

## 🚧 État actuel

**Version:** 0.1.0 - Structure initiale  
**Phase:** 1 - MVP en développement

**Prochaines étapes:**
1. ✅ Setup infrastructure (terminé)
2. ⏳ Acquisition données Binance
3. ⏳ Détection volatilité
4. ⏳ ML modèle basique
5. ⏳ Stratégie Straddle
6. ⏳ Backtesting
7. ⏳ Dashboard minimal

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
