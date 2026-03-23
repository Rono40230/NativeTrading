# Plan de travail — SMC Directionnel + Straddle

> Objectif : aligner SMC et Straddle sur le niveau de maturité de Rockets
> (filtrage LLM qualité, historique complet, analyse des performances)

---

## Vue d'ensemble des 3 stratégies

| Stratégie | Assets | Mode | Rôle du LLM |
|---|---|---|---|
| **Rockets** | 200+ cryptos Binance | Scanner auto (scan 5 min) | Filtre pré-save (conviction ≥65) + analyse périodique |
| **SMC Directionnel** | Forex, XAUUSD, XAGUSD, BTC, ETH | Moteur auto (Signal Engine 5 min) | Filtre pré-save + enrichissement TP/SL |
| **Straddle** | Forex, XAUUSD, XAGUSD, BTC, ETH | **Manuel / Recherche** | Identifier des créneaux de forte volatilité récurrente à valider en backtesting |

> ⚠️ **Le marché crypto (hors BTC/ETH) n'est PAS concerné par le Straddle.**
> ⚠️ **Le Straddle ne génère PAS de signaux live automatiques.**
> Le LLM Straddle produit des **créneaux horaires à tester en backtest**, pas des ordres.

---

## Chantier 1 — SMC Directionnel : filtrage qualité + traçabilité LLM

### 1.1 — Migration DB

Ajouter dans la table `signaux` les colonnes de traçabilité LLM (migration 0015) :

```sql
ALTER TABLE signaux ADD COLUMN llm_valide      INTEGER;  -- 1=validé | 0=rejeté | NULL=LLM indispo
ALTER TABLE signaux ADD COLUMN llm_conviction  INTEGER;  -- 0–100
ALTER TABLE signaux ADD COLUMN llm_raison      TEXT;     -- explication courte LLM
ALTER TABLE signaux ADD COLUMN llm_sl_suggere  REAL;     -- SL ajusté par LLM (optionnel)
ALTER TABLE signaux ADD COLUMN llm_tp1_suggere REAL;     -- TP1 ajusté par LLM (optionnel)
```

### 1.2 — Filtre pré-sauvegarde (comme `rockets_filtre.rs`)

Créer `backend/crates/api/src/ollama/smc_filtre.rs` :

- Identique à `rockets_filtre.rs` dans sa structure
- Données envoyées : asset, timeframe, direction, score SMC, RSI, ATR ratio, kill zone active, sweep détecté, historique 10 derniers signaux sur cet asset
- Seuil : **conviction ≥ 65** pour sauvegarde
- Philosophie : **qualité > quantité — en cas de doute → rejeter**
- Le filtre bloque la sauvegarde dans `boucle_detection()` du Signal Engine

### 1.3 — Mise à jour du prompt SMC

Refondre `PROMPT_SIGNAL_SMC` dans `ollama/prompts.rs` :

- Ajouter la philosophie "qualité > quantité" (comme Rockets)
- Barème conviction aligné : < 65 = `valide=false` IMPÉRATIF
- Ajouter critères d'invalidation stricts :
  - Kill Zone non active → invalider
  - Sweep non confirmé → dégrader
  - RSI en zone extrême (>85 ou <15) → invalider
  - ATR ratio < 0.8 (compression, pas de momentum) → invalider
  - Contexte historique : winrate < 40% sur cet asset+phase → éviter
- Recommander ajustements SL/TP1 si l'historique montre des niveaux récurrents

### 1.4 — Analyse LLM périodique des performances SMC

Créer `backend/crates/api/src/ollama/smc_analyse.rs` (calqué sur `rockets_analyse.rs`) :

- Déclenchement : 1× par semaine (ou sur demande)
- Données analysées : taux de succès par asset, par timeframe, par Kill Zone, par phase de marché
- Résultat stocké en DB (nouvelle table `smc_analyses_llm`)
- L'analyse répond à : "quels assets/timeframes/KZ ont le meilleur winrate ? lesquels éviter ?"

### 1.5 — Frontend : compléter la modale HistoryView

- La modale "Analyse SMC Directionnel" dans `HistoryView.vue` est actuellement un **placeholder vide**
- La compléter avec : conviction LLM, raison, SL/TP suggérés, lien vers l'analyse périodique

---

## Chantier 2 — Straddle : redesign en outil de recherche de créneaux

### Concept

Le Straddle n'est **pas** un générateur de signaux live.  
C'est un **outil analytique** : le LLM analyse les données historiques de volatilité et identifie des **patterns récurrents** (jour de la semaine × heure × asset) qui méritent d'être testés en backtest avec une stratégie bidirectionnelle.

### Flux cible

```
Historique OHLCV (2 ans) + Calendrier économique
          ↓
    Analyse LLM (manuelle, sur demande)
          ↓
    Créneaux identifiés (ex: "Mardi 14h–16h UTC, XAUUSD, ATR moyen +90 pips")
          ↓
    Sauvegarde en DB (table straddle_creneaux)
          ↓
    Backtest sur ces créneaux → validation statistique
          ↓
    Résultats affichés dans une vue dédiée
```

### 2.1 — Nettoyage du code existant

- `strategies/src/straddle.rs` génère actuellement des signaux live ATR×1.5 → à **déconnecter du Signal Engine**
- Les endpoints `POST /api/ia/signal/straddle` et `straddle_handlers.rs` → à réorienter vers l'analyse de créneaux
- Le code ATR peut être conservé comme **indicateur de calcul** (pas comme déclencheur de signal)

### 2.2 — Nouvelle table DB `straddle_creneaux` (migration 0016)

```sql
CREATE TABLE straddle_creneaux (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    asset       TEXT    NOT NULL,       -- XAUUSD, EURUSD, BTCUSDT, ETHUSDT...
    jour_semaine INTEGER,               -- 0=Lundi ... 6=Dimanche (NULL=tous)
    heure_debut TEXT    NOT NULL,       -- "14:00" UTC
    heure_fin   TEXT    NOT NULL,       -- "16:00" UTC
    atr_moyen   REAL,                   -- amplitude moyenne détectée (en pips ou %)
    frequence   REAL,                   -- % des occurrences où le seuil est atteint
    llm_raison  TEXT,                   -- explication du LLM
    llm_conviction INTEGER,            -- 0–100
    statut      TEXT DEFAULT 'a_tester', -- 'a_tester' | 'valide' | 'invalide'
    cree_le     TEXT DEFAULT (datetime('now')),
    backtest_winrate REAL,             -- rempli après backtest
    backtest_profit_factor REAL
);
```

### 2.3 — Prompt Straddle (refonte complète)

Créer `ollama/prompts.rs` > `PROMPT_ANALYSE_STRADDLE` :

Données fournies au LLM :
- Historique OHLCV 1h sur 6–24 mois pour l'asset (amplitude bougie par bougie)
- Calendrier économique de la période (annonces HIGH)
- ATR moyen par tranche horaire et par jour de la semaine
- Résultats backtests des créneaux déjà testés (apprentissage)

Questions posées au LLM :
1. Quels créneaux jour×heure montrent une **récurrence de mouvement supérieur à 1.5×ATR moyen** ?
2. Ces créneaux coïncident-ils avec des annonces économiques régulières ?
3. Priorité de test (conviction 0–100 par créneau)
4. SL/TP suggérés (basés sur l'amplitude historique)

Format de réponse : JSON array de créneaux

### 2.4 — Handler et planification

- Endpoint `POST /api/straddle/analyser` : déclenche l'analyse LLM (manuelle, sur demande)
- Endpoint `GET /api/straddle/creneaux` : liste les créneaux identifiés
- Endpoint `PATCH /api/straddle/creneaux/:id` : mettre à jour statut + résultats backtest
- **Pas de scanner automatique** — toujours sur demande

### 2.5 — Vue frontend dédiée Straddle

Nouvelle vue `StraddleView.vue` :

- **Panneau de lancement** : choisir asset + période d'analyse → bouton "Analyser" → appel LLM
- **Tableau des créneaux** : asset, créneau horaire, ATR moyen, fréquence, conviction LLM, statut
- **Intégration backtest** : bouton "Tester ce créneau" → lance backtest sur la période concernée
- **Résultats** : winrate, profit factor, affichage dans le tableau

---

## Chantier 3 — Commun : qualité des prompts

### 3.1 — Philosophie unifiée pour tous les LLMs

Dans tous les prompts (SMC, Straddle, Rockets analyse), ajouter :

```
PHILOSOPHIE : QUALITÉ > QUANTITÉ
Il vaut MIEUX passer 0 signal que valider 1 mauvais signal.
En cas de doute → conviction < 65 → valide=false.
```

### 3.2 — Harmoniser les barèmes

| Score | Signification | Action |
|---|---|---|
| 80–100 | Setup excellent | `valide=true` |
| 65–79 | Bon setup | `valide=true` |
| < 65 | Insuffisant | `valide=false` — **IMPÉRATIF** |

---

## Ordre d'exécution recommandé

```
Sprint 1 (SMC filtre qualité)
  ├── Migration 0015 (colonnes LLM sur signaux)
  ├── smc_filtre.rs (filtre pré-save conviction ≥65)
  ├── Prompt SMC refondu
  └── Modale HistoryView SMC complétée

Sprint 2 (Straddle outil de recherche)
  ├── Migration 0016 (table straddle_creneaux)
  ├── Prompt Straddle refondu
  ├── Handler + endpoints Straddle
  └── Vue StraddleView.vue

Sprint 3 (Analyse périodique SMC)
  ├── Table smc_analyses_llm
  ├── smc_analyse.rs (analyse hebdo)
  └── UI : affichage synthèse LLM dans HistoryView
```

---

---

## Décisions d'architecture (arrêtées)

### `straddle.rs` actuel (signaux ATR×1.5)
**→ Conserver comme simulation**, déconnecté du Signal Engine live.  
Sert de stratégie passée au `BacktestEngine` lors du test des créneaux.

### Assets Straddle
**→ Configurables depuis l'UI** (section "Paramètres", comme les assets actuels).  
Assets par défaut : XAUUSD, XAGUSD, EURUSD, GBPUSD, BTCUSDT, ETHUSDT.  
La liste est modifiable par l'utilisateur.

### Période d'analyse historique
**→ Modulable** — choix dans l'UI parmi : 3 mois / 6 mois / 1 an / 2 ans.  
Permet de comparer les créneaux sur différentes profondeurs d'historique.

### Moteur de backtest
**→ Le `BacktestEngine` existant est réutilisé tel quel.** Il supporte déjà `Direction::Both`
(nécessaire pour le Straddle bidirectionnel) et calcule winrate, Sharpe, drawdown, profit_factor.  
La seule addition nécessaire : **un filtre de bougies par créneau** (jour × heure UTC) qui
extrait uniquement les bougies correspondant au créneau cible avant de les passer à `BacktestEngine::run()`.

```
BacktestEngine::run(
    filtrer_par_creneau(&toutes_bougies, jour=Mardi, heure_debut="14:00", heure_fin="16:00"),
    &StraddleStrategy { ... }
)
→ BacktestResults { win_rate, sharpe_ratio, profit_factor, ... }
```
