# 🗺️ ROADMAP — Native Trading AI
> Dernière mise à jour : 16 avril 2026

## ✅ BILAN — Tout est terminé sauf P13

| Priorité | Intitulé | Statut |
|---|---|---|
| P1 | Walk-forward rollback ML | ✅ FAIT |
| P2 | Snapshot features Rockets | ✅ FAIT |
| P3 | Ré-entraînement sur trades clôturés | ✅ FAIT |
| P4 | Feature importance | ✅ FAIT |
| P5 | Seuil de confiance ML dynamique | ✅ FAIT |
| P6 | Feedback loop trader | ✅ FAIT |
| P7 | Capital simulé + position sizing | ✅ FAIT |
| P8 | Dashboard performances LLM | ✅ FAIT |
| P9 | Enrichissement automatique feedback | ✅ FAIT |
| P10 | Détection patterns d'échec | ✅ FAIT |
| P11 | UI tableau suivi signaux pris | ✅ FAIT |
| P12 | Calibration automatique paramètres | ✅ FAIT |
| **P13** | **Extension Straddle + SMC** | **❌ À FAIRE** |

---

## 🔴 P13 — Extension à Straddle et SMC (apprentissages indépendants)

> **État : ❌ À FAIRE** — Aucun snapshot features pour Straddle/SMC, aucun fine-tuning, aucun modèle séparé. Seul `modele_xgboost.json` généraliste existe.

**Règle absolue :** chaque stratégie a ses propres tables, son propre modèle, son propre prompt LLM. Aucune donnée partagée entre stratégies.

### Contexte architecture (rappel)

```
Niveau 1 — Pré-entraînement général (conservé, identique pour les 3 stratégies)
  Bougies OHLCV historiques → XGBoost/LSTM → connaissance directionnelle du marché

Niveau 2 — Fine-tuning stratégie (fait pour Rockets, à faire pour Straddle + SMC)
  Signal émis → snapshot features → trade clôturé → label réel (TP/SL)
  → {strategie}_features_snapshot JOIN {strategie}_signaux
  → XGBoost fine-tuné par stratégie
```

### Ce qui existe pour Rockets (référence à répliquer)

| Composant | Rockets ✅ | Straddle ❌ | SMC ❌ |
|---|---|---|---|
| Snapshot features | `rockets_features_snapshot` | `straddle_features_snapshot` | `smc_features_snapshot` |
| Feedback | `rockets_feedback` | `straddle_feedback` | `smc_feedback` |
| Modèle XGB | `modele_xgboost_rockets.json` | `modele_xgboost_straddle.json` | `modele_xgboost_smc.json` |
| Trainer ML | `ml/src/rockets_trainer.rs` | `ml/src/straddle_trainer.rs` | `ml/src/smc_trainer.rs` |
| Features | `extraire_features()` (OHLCV) | idem (OHLCV) | `extraire_features_smc()` (+ 5 SMC) |

---

### Straddle — ce qui manque

**1. Migration SQL** — nouvelle table :
```sql
CREATE TABLE straddle_features_snapshot (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL UNIQUE REFERENCES straddle_signaux(id),
    ticker        TEXT NOT NULL,
    cree_le       TEXT NOT NULL DEFAULT (datetime('now')),
    features_json TEXT NOT NULL
);
```

**2. `backend/crates/db/src/straddle_features.rs`** — copie du pattern `rockets_features.rs` :
- `inserer_snapshot(pool, signal_id, ticker, features)`
- `lire_snapshots_avec_labels(pool)`

**3. `backend/crates/ml/src/straddle_trainer.rs`** — copie de `rockets_trainer.rs` :
- `XgbStraddle`, chemin `data/modele_xgboost_straddle.json`
- `entrainer_sur_trades_clotures()`, guard min 50 samples

**4. `backend/crates/api/src/straddle_signal_handler.rs`** — appel snapshot après sauvegarde signal.

**5. `backend/crates/api/src/ml_retrain_job.rs`** — ajouter `executer_fine_tuning_straddle()` appelé par le scheduler quotidien.

---

### SMC — ce qui manque (plus complexe)

**1. `backend/crates/smc/src/features.rs`** — `extraire_features_smc()` avec les indicateurs SMC supplémentaires :
- Tendance (HH/HL/LH/LL) → valeur normalisée
- Order Block (présence + distance) → 2 features
- Imbalance (gap pips) → 1 feature
- IFVG (présence + direction) → 2 features
- Fibonacci (niveau le plus proche : 38.2/50/61.8) → 1 feature

Le vecteur SMC = vecteur OHLCV standard + ces 7 features supplémentaires.

**2. Migration SQL** :
```sql
CREATE TABLE smc_features_snapshot (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL UNIQUE REFERENCES smc_signaux(id),
    ticker        TEXT NOT NULL,
    cree_le       TEXT NOT NULL DEFAULT (datetime('now')),
    features_json TEXT NOT NULL
);
```

**3. `backend/crates/db/src/smc_features.rs`** — même pattern, FK vers `smc_signaux`.

**4. `backend/crates/ml/src/smc_trainer.rs`** — `XgbSmc`, chemin `data/modele_xgboost_smc.json`.

**5. `backend/crates/api/src/smc_boucle.rs`** — appel snapshot après émission signal.

**6. `backend/crates/api/src/ml_retrain_job.rs`** — ajouter `executer_fine_tuning_smc()`.

---

### Ordre d'implémentation recommandé

1. **Straddle** en premier (réplication quasi-directe de Rockets, features identiques)
2. **SMC** ensuite (nécessite `extraire_features_smc()` dans le crate `smc` d'abord)

---

### Ce qui est hors scope

| Élément | Pourquoi exclu |
|---|---|
| Exécution automatique des trades | L'app est signal-only |
| Connexion API Binance/MT5 pour le solde | Pas de trading réel |
| Fine-tuning du modèle LLM | Trop lourd, hors scope actuel |
| Apprentissage croisé entre stratégies | Contamination des données — interdit |

---

## 📏 RÈGLES ABSOLUES (rappel workflow)

### Règle 1 — Zéro régression
- Lister les fichiers impactés avant tout changement
- `cargo build --workspace` + `cargo test --workspace` après chaque modification
- Test qui échoue → corriger AVANT de continuer

### Règle 2 — Explication avant action
Fournir avant de toucher au code :
1. **Ce qui va être modifié** (fichiers + fonctions)
2. **Pourquoi** (justification métier/technique)
3. **Risques identifiés**
4. **Plan de rollback**

### Règle 3 — Workflow audit avant commit
```bash
./.vibe/bin/audit.sh        # Clippy + tests + taille fichiers + zero-unwrap
cargo test --workspace
cd frontend && npm run test
```

Alertes bloquantes :
- Fichier ≥ 250 lignes → split immédiat
- `unwrap()` / `console.log()` → bloquant
- Calcul métier côté Vue → interdit
