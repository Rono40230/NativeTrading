# Roadmap — État d'avancement (mise à jour 2026-08-13)

> Ce document remplace la roadmap précédente comme **tableau de bord de progression**.
> La spécification détaillée reste dans `docs/superpowers/specs/2026-08-11-objectifs-roadmap.md`.

---

## 📊 Vue d'ensemble

| Phase | Objectif | Statut | Tests |
|-------|----------|--------|-------|
| **1. Assainissement** | Code mort, bugs, dette technique | ✅ **Terminé** | 119→187 |
| **2. SMC v12** | Reproduction fidèle du Pine v12 | ✅ **Terminé** (backend + UI) | 187 |
| **Data Pipeline** | Ingestion continue des bougies | 🔄 **En cours** (Bybit ✅, IG ❌) | — |
| **Sentiment** | Composite multi-asset + post-filtre | ✅ **Phases 1-3** (technique + F&G + VIX + filtre) | 187 |
| **TV Charting Library** | Affichage Pine natif | ⏳ **En attente** (formulaire soumis) | — |
| **3. Prompts IA** | Amélioration des prompts | ⏸ **Différé** (après stratégies définies) | — |
| **4. Revue de presse** | Digest IA ciblé | ⏸ **À brainstormer** | — |
| **5. Fuseaux horaires** | Paris partout | ✅ **Terminé** | — |
| **6. Alertes** | Sélection + type/contenu | ⏸ Mail supprimé. Telegram existe. À améliorer. | — |
| **7. Extension actifs** | Forex + crypto + actions | 🔄 Crypto + métaux ✅. Forex ❌ (IG cassé). Actions ❌ | — |
| **8. Journal + Apprentissage** | Trades → IA apprend | ⏸ **À faire** | — |
| **9. Constructeur stratégies** | Ajouter de nouvelles stratégies | ⏸ **À faire** (phase finale) | — |

---

## ✅ Phase 1 — Assainissement & bases solides (TERMINÉ)

| Sous-plan | Contenu | Commits | Statut |
|-----------|---------|---------|--------|
| 1.1 | Code mort + navigation cassée | 7 | ✅ Fusionné |
| 1.2 | 3 bugs critiques (double-start, fuite mémoire, params SMC) | 3 | ✅ Fusionné |
| 1.3 | Suppression backtest (D1) | 3 | ✅ Fusionné |
| 1.4 | Mutualisation HTTP/Ollama (4 bypass sémaphore corrigés) | 4 | ✅ Fusionné |
| 1.5 | Frontend HTTP unifié + ts-rs drift corrigé | 5 | ✅ Fusionné |
| 1.6a-d | Démonolithage : extraction crates `notifications`, `llm`, `news`, `brokers→data` | 13 | ✅ Fusionné |
| Quick wins | Prompts sidebar, Rockets analyse data fix, BOS bornés | 3 | ✅ Fusionné |
| D0 | Hook Vibe 300→600 lignes | 2 | ✅ Fusionné |

**Bilan** : 12 crates dans le workspace (avant : 9 + monolithe api). 187 tests (avant : 131). Zéro bug critique connu.

---

## ✅ Phase 2 — Reproduction SMC v12 (TERMINÉ)

| Sous-plan | Contenu | Fichiers v12 | Statut |
|-----------|---------|-------------|--------|
| 2.0 | Foundation (ATR, pivots, structure, BOS) | 8 | ✅ |
| 2.1 | MSS/CHOCH + Liquidités + Sweep | +3 | ✅ |
| 2.2 | Zones (FVG, OB, Breaker, Propulsion, Imbalance) | +5 | ✅ |
| 2.3 | Contexte (PD, OTE, KZ, NDOG, MTF H1/H4/W1/MN, Zone-cœur) | +6 | ✅ |
| 2.5-2.7 | Scoring v11 (16 composantes) + BSZones + Signaux + Lifecycle | +6 | ✅ |
| 2.8 | Rendu visuel (API replay + overlay canvas + bascules ON/OFF) | +3 backend, +3 frontend | ✅ |

**Bilan** : 28 fichiers Rust dans `smc::v12`. Les 20+ indicateurs Pine v12 sont reproduits. Overlay canvas avec toggle par indicateur. Endpoint `/api/smc/v12/analyse`.

**Vérification parité TradingView** : ⏳ En attente de validation visuelle (ou TV Charting Library pour résoudre définitivement).

---

## 🔄 Data Pipeline (EN COURS — priorité #1)

### Ce qui marche

| Source | Actifs | Mécanisme | Statut |
|--------|--------|-----------|--------|
| **Bybit WS** | 10 crypto + XAUUSD + XAGUSD | Worker persistant 24/7, 60 topics (12 actifs × 5 TF) | ✅ **Actif** — bougies fraîches en DB (source `bybit_ws`) |
| **Yahoo Finance** | S&P500, DAX, Or, VIX… | Fetch live non officiel (fragile) | ⚠ Fragile |
| **Binance REST** | Crypto (Rockets scan) | API publique (géo-bloqué France) | ⚠ 7 endpoints à migrer vers Bybit |

### Ce qui ne marche pas

| Source | Actifs | Problème | Statut |
|--------|--------|----------|--------|
| **IG Markets** | 12 forex + 7 indices | Compte inactif (`client-inactive-must-reapply`) | ❌ L'utilisateur doit réactiver son compte sur IG.com |
| **MT5** | Historique | Import CSV manuel seulement | ❌ Pas temps réel |

### Décisions prises

| # | Décision | Statut |
|---|----------|--------|
| D2 | CPG abandonné. Stratégie gratuite (Bybit + réparer IG) | ✅ Tranché |
| — | Mail alerts supprimés (Telegram uniquement) | ✅ |
| — | Circuit breaker IG (cooldown 5→30 min) | ✅ Implémenté |
| — | Worker Bybit WS persistant | ✅ Implémenté |
| — | Spike NautilusTrader | ✅ Validé (pipeline marche, en pause) |

### Reste à faire (priorité)

1. ⏳ **Réactiver le compte IG** (côté utilisateur) → forex + indices.
2. ⏳ **Finir migration Binance→Bybit** (7 endpoints restants pour Rockets scan).
3. ⏳ **D1 backfill** pour les 10 assets du sentiment (actuellement stale depuis mars/avril pour la plupart).

---

## ✅ Fuseaux horaires (TERMINÉ)

- `chrono-tz` ajouté, helper unifié `common::time::paris_from_unix()`.
- DST manuel supprimé (liquidites_tz.rs + scheduler.rs).
- 4 bugs corrigés (heatmap roulement jour, créneaux label Paris, weekday, MarketClocks DST).
- ~15 formatters frontend passés en `Europe/Paris` explicite (`utils/date.ts`).
- Kill Zones ICT conservées en UTC (convention TradingView).

---

## ✅ Sentiment Composite (PHASES 1-3 TERMINÉES)

| Phase | Contenu | Statut |
|-------|---------|--------|
| 1 | Sentiment technique (RSI14 D1 + breadth MA20) | ✅ |
| 2 | F&G reconnecté (crypto) + VIX inversé (forex/indices/métaux) | ✅ |
| 3 | Post-filtre directionnel (aligné +15% / opposé -20% / extrême skip) | ✅ |
| Frontend | Jauge circulaire 0-100 + 4 mini-jauges par classe | ✅ |
| 4 | FinBERT (news directionnel, ~85% précision) | ⏸ Différé |
| 5 | Synthèse Ollama daily (qwen2.5:14b) | ⏸ Différé |

**Note** : le sentiment est actuellement faux car les D1 sont stale (sauf XAUUSD). Il deviendra fiable quand le Bybit WS aura backfillé les D1 (crypto + métaux) et que IG sera réparé (forex + indices).

---

## ⏳ TradingView Charting Library (EN ATTENTE)

- Page démo GitHub Pages : ✅ `https://rono40230.github.io/NativeTrading/`
- Formulaire TV soumis : ✅ 2026-08-13
- Approvation attendue : 1-3 jours ouvrés
- Si approuvé → remplacement lightweight-charts par widget TV + Pine v12 natif (100% parité)
- Si refusé → continuer avec overlays canvas actuels

---

## ⏸ Phases différées

| Phase | Pourquoi différée | Débloquée quand ? |
|-------|-------------------|-------------------|
| **3. Prompts IA** | Améliorer après stratégies définies | Fin SMC v12 + parité TV vérifiée |
| **4. Revue de presse** | Ciblage à brainstormer (D4) | L'utilisateur est prêt à en discuter |
| **6. Alertes (type/contenu)** | D5 à définir | Après data pipeline fiable |
| **7. Journal + Apprentissage** | Dépend des signaux fiables | Après data pipeline + parité SMC |
| **9. Constructeur stratégies** | Phase finale | Tout le reste stabilisé |

---

## 📈 Métriques du projet

| Métrique | Avant session (2026-08-11) | Maintenant (2026-08-13) |
|----------|--------------------------|------------------------|
| Crates workspace | 9 (monolithe api) | 12 (notifications, llm, news extraits) |
| Fichiers smc::v12 | 0 | 28 |
| Tests backend | 131 | 187 |
| Backtest | Cassé (3 implémentations divergentes) | Supprimé (D1) |
| Data ingestion | Frontend-dépendant (stale) | **Bybit WS persistant 24/7** (crypto + métaux) |
| Sentiment | Tableau de prix Yahoo déguisé | Composite multi-asset + post-filtre directionnel |
| Fuseaux | UTC/Paris mélangés | Unifié Europe/Paris |
| Bug circuit breaker IG | Hammering 2880/jour | Cooldown 5→30 min |
| Dette backtest | 3 implémentations divergentes | Supprimée proprement |

---

## 🎯 Priorités immédiates

1. **Utilisateur** : réactiver compte IG sur ig.com
2. **Code** : finir migration Binance→Bybit (7 endpoints Rockets)
3. **Data** : D1 backfill pour les 10 assets du sentiment
4. **Attente** : réponse TradingView Charting Library (1-3 jours)
5. **Discussion** : revue de presse (ciblage + workflow)
