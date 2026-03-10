# 📋 CAHIER DES CHARGES - NATIVE TRADING AI

**Version:** 1.0  
**Date:** 9 mars 2026  
**Projet:** Système IA native pour trading algorithmique  

---

## 1. OBJECTIF GLOBAL

Développer une **intelligence artificielle locale** de trading pour détecter et exploiter :
- **Volatilité forte et récurrente** sur XAUUSD, XAGUSD, BTC, ETH
- **Opportunités SMC (Smart Money Concept)** via analyse multi-indicateurs
- **Backtesting et simulation** de stratégies

scalping directionnel et straddle

Le système doit **analyser les marchés**, **prédire les tendances**, et **générer des signaux de trading** en temps quasi-réel (<10s), tout en restant **100% local**.

---

## 2. EXIGENCES COMPLÈTES

Voir le document complet pour toutes les spécifications techniques, fonctionnelles et non-fonctionnelles détaillées.

**Configuration système:**
- OS: Fedora 43
- GPU: NVIDIA RTX 3090 (48 GB RAM)
- Stack: Rust + Vue.js 3 + SQLite + CUDA

**Données:**
- Sources: MT5 (AXI), Binance, APIs multiples
- Actifs: XAUUSD, XAGUSD, BTC, ETH
- Timeframes: M1, M5, M15
- Historique: 6 mois

**IA:**
- Modèle hybride: LSTM + XGBoost
- Outputs multiples: Classification binaire/ternaire + Régression
- Entraînement quotidien automatique

**Stratégies:**
1. **Straddle** - Volatilité extrême (positions opposées)
2. **SMC Directionnel** - Confluence indicateurs (score ≥70/100)

**Indicateurs SMC (priorités 1-5):**
1. Tendances
2. Order Blocks
3. Imbalance
4. IFVG
5. Fibonacci

**Gestion risques:**
- Risque par trade: 0.5-2% capital
- Max 3 positions simultanées
- Exposition max 25% par actif
- Drawdown max 20% (arrêt auto)

**Interface:**
- 5 dashboards Vue.js
- Graphiques temps réel
- Alertes combinées (notifications natives système + sons + emails)
- Export PDF/CSV

**Performance:**
- Inférence IA: <200ms
- Latence signaux: <10s
- Entraînement: <1h

**Métriques cibles:**
- ROI 1 an: >15%
- Sharpe ratio: >1.5
- Max drawdown: <20%
- Win rate: >55%

**Planning:**
- Phase 1 MVP: 6 semaines
- Phase 2 Production: 8 semaines
- Phase 3 Optimisation: 4 semaines
- Phase 4 Production réelle: Continue

---

**Documents complémentaires:**
- [ROADMAP.md](ROADMAP.md) - Feuille de route détaillée
- [ARCHITECTURE.md](ARCHITECTURE.md) - Architecture technique
- [README.md](README.md) - Guide d'installation et usage
