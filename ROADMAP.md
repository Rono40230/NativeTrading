# 🗺️ ROADMAP — Native Trading AI
> Dernière mise à jour : 16 avril 2026

## ✅ BILAN

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
| **P13** | **Extension Straddle + SMC** | **⚠️ PARTIEL** |

---

## ⚠️ P13 — Extension Straddle + SMC (état partiel)

> **État : ⚠️ PARTIEL** — Straddle terminé. SMC non commencé.

### Straddle ✅ TERMINÉ

| Composant | Fichier | Statut |
|---|---|---|
| Migration snapshot | `0053_straddle_features_snapshot.sql` | ✅ |
| Module DB | `db/src/straddle_features.rs` | ✅ |
| Trainer ML | `ml/src/straddle_trainer.rs` (`XgbStraddle`) | ✅ |
| Intégration pipeline | `ml/src/lib.rs` (`XgbStraddle` dans `PipelineML`) | ✅ |
| Appel snapshot | `api/src/straddle_signal_handler.rs` (l. 293-300) | ✅ |
| Fine-tuning job | `api/src/ml_retrain_job.rs` (`executer_fine_tuning_straddle`) | ✅ |
| Modèle sur disque | `data/modele_xgboost_straddle.json` | ❌ pas encore généré (manque de données) |

### SMC ❌ NON COMMENCÉ

| Composant | Fichier | Statut |
|---|---|---|
| Migration snapshot | `0055_smc_features_snapshot.sql` | ❌ |
| Module DB | `db/src/smc_features.rs` | ❌ |
| Features SMC étendues | `smc/src/features.rs` (7 features SMC en plus) | ❌ |
| Trainer ML | `ml/src/smc_trainer.rs` (`XgbSmc`) | ❌ |
| Intégration pipeline | `ml/src/lib.rs` (`XgbSmc`) | ❌ |
| Appel snapshot | `api/src/smc_boucle.rs` | ❌ |
| Fine-tuning job | `api/src/ml_retrain_job.rs` (`executer_fine_tuning_smc`) | ❌ |

---

## 🔴 P13 — Extension SMC (apprentissage indépendant)

> **État : ❌ À FAIRE** — Straddle ✅ terminé. SMC non commencé.

**Règle absolue :** chaque stratégie a ses propres tables, son propre modèle, son propre prompt LLM. Aucune donnée partagée entre stratégies.

### Contexte architecture (rappel)

```
Niveau 1 — Pré-entraînement général (conservé, identique pour les 3 stratégies)
  Bougies OHLCV historiques → XGBoost/LSTM → connaissance directionnelle du marché

Niveau 2 — Fine-tuning stratégie (fait pour Rockets ✅ et Straddle ✅, reste SMC)
  Signal émis → snapshot features → trade clôturé → label réel (TP/SL)
  → {strategie}_features_snapshot JOIN signaux
  → XGBoost fine-tuné par stratégie
```

### Référence par stratégie

| Composant | Rockets ✅ | Straddle ✅ | SMC ❌ |
|---|---|---|---|
| Snapshot features | `rockets_features_snapshot` | `straddle_features_snapshot` | `smc_features_snapshot` |
| Module DB | `db/src/rockets_features.rs` | `db/src/straddle_features.rs` | `db/src/smc_features.rs` |
| Trainer ML | `ml/src/rockets_trainer.rs` | `ml/src/straddle_trainer.rs` | `ml/src/smc_trainer.rs` |
| Fine-tuning | `executer_fine_tuning_rockets` | `executer_fine_tuning_straddle` | `executer_fine_tuning_smc` |
| Modèle | `modele_xgboost_rockets.json` | `modele_xgboost_straddle.json` (⚠️ pas encore généré — manque données) | `modele_xgboost_smc.json` |

### SMC — plan d'implémentation

**1. Migration SQL** `0055_smc_features_snapshot.sql` :
```sql
CREATE TABLE smc_features_snapshot (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL UNIQUE REFERENCES signaux(id),
    ticker        TEXT NOT NULL,
    cree_le       TEXT NOT NULL DEFAULT (datetime('now')),
    features_json TEXT NOT NULL
);
```

**2. `backend/crates/db/src/smc_features.rs`** — pattern identique à `straddle_features.rs` :
- `inserer_snapshot(pool, signal_id, ticker, features)`
- `lire_snapshots_avec_labels(pool)`

**3. Features SMC étendues** dans `smc/src/` ou `ml/src/features.rs` :
- Vecteur OHLCV standard + 7 features SMC : Tendance, Order Block (×2), Imbalance, IFVG (×2), Fibonacci

**4. `backend/crates/ml/src/smc_trainer.rs`** — `XgbSmc`, chemin `data/modele_xgboost_smc.json`, guard min 50 samples.

**5. `backend/crates/ml/src/lib.rs`** — ajouter `XgbSmc` dans `PipelineML`.

**6. `backend/crates/api/src/smc_boucle.rs`** — appel snapshot après émission signal.

**7. `backend/crates/api/src/ml_retrain_job.rs`** — ajouter `executer_fine_tuning_smc()`.

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
cargo clippy --workspace -- -D warnings
cargo test --workspace
cd frontend && npm run type-check
```

Alertes bloquantes :
- Fichier ≥ 250 lignes → split immédiat
- `unwrap()` / `console.log()` → bloquant
- Calcul métier côté Vue → interdit

### Fichiers à surveiller (proches limite 300 lignes)

| Fichier | Lignes |
|---|---|
| `api/src/straddle_signal_handler.rs` | 303 ⚠️ |
| `api/src/routes.rs` | 285 |
| `api/src/news_scoring.rs` | 278 |
| `api/src/straddle_boucle.rs` | 279 |
| `frontend/src/views/HistoryView.vue` | 294 |
| `frontend/src/composables/useSmcCanvas.ts` | 297 |
| `frontend/src/composables/useChartIndicators.ts` | 271 |

