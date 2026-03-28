# TODO — Restant à faire
Date: 28 mars 2026

---

## Phase 4 — Architecture/Nettoyage

### 4.2 Supprimer code ML mort (RandomForest) ✅ TERMINÉ (commit 5d4469e)
Zone: `backend/crates/ml`
- [x] Supprimer log "RF+LSTM" → "XGBoost+LSTM" dans ml/src/lib.rs
- [x] Supprimer commentaire RandomForest dans ml/src/features.rs
Critère: backend ML sans références mortes, `cargo clippy` vert

### 4.3 Extraction logique métier hors couche API ✅ TERMINÉ (commit 5d4469e)
Zone: `backend/crates/api/src`, `backtest/src`, `strategies/src`
- [x] `rockets_indicateurs.rs` déplacé api/src → strategies/src
- [x] `straddle_precision.rs` déplacé api/src → strategies/src
- [x] `strategies/Cargo.toml`: ajout dépendance `db`
- [x] Tous les imports mis à jour dans les fichiers api consommateurs
Critère: couche API centrée transport HTTP uniquement

### 4.4 WebSocket signaux — vérification robustesse
- [ ] Tester stabilité en usage prolongé (composable `useSignalEngine`)

### Tests de sortie Phase 4 ✅ TERMINÉ (commit 5d4469e)
- [x] `bash scripts/check-file-size.sh` → 0 fichier >300 lignes
- [x] `cd backend && cargo clippy --workspace -- -D warnings` → 0 warnings
- [x] `cd backend && cargo test --workspace` → 59/59 OK
- [ ] `cd frontend && npm run build` (vérifier app Tauri)
- [ ] Vérifier signaux temps réel reçus en UI via WebSocket

---

## Phase 3 — Validations manuelles en app (3.4 + 3.5)

### Ollama rate limiting (impl. faite, valider en app)
- [ ] Lancer plusieurs analyses LLM en rafale → pas de freeze/crash
- [ ] Vérifier dégradation propre si Ollama lent/injoignable

### Surveillance accuracy_val (impl. faite, valider en app)
- [ ] Forcer accuracy_val bas → warning UI actif
- [ ] Vérifier déclenchement worker de ré-entraînement (simulation seuil <52%)
- [ ] Vérifier qu'en régime normal le worker n'entraîne pas inutilement

---

## Phase 5 — Paramétrage MQ5 + Backtest pilotable

### 5.1 Paramètres Straddle (BiDiParams)
- [ ] Struct params dédiée
- [ ] GET/PUT API params Straddle
- [ ] Persistance DB
- [ ] Backtest Straddle piloté par ces paramètres

### 5.2 Paramètres SMC (UniParams)
- [ ] Struct params dédiée
- [ ] GET/PUT API params SMC
- [ ] Persistance DB
- [ ] Backtest SMC piloté par ces paramètres

### 5.3 Fonctionnalités backtest manquantes
- [ ] Break-even
- [ ] Trailing stop
- [ ] Expiration
- [ ] TP partiels

### 5.4 UI de pilotage backtest
- [ ] Formulaires paramètres dans vues Straddle/SMC
- [ ] Debounce + relance auto du backtest à chaque modification
- [ ] Affichage immédiat des résultats recalculés

### Tests de sortie Phase 5
- [ ] Changer ATRPeriod / TP ratio → résultats backtest changent de façon cohérente
- [ ] Vérifier persistance des paramètres après reload UI
- [ ] Comparaison Rust vs MT5 sur cas témoin (écart acceptable défini)
- [ ] `cargo test --workspace`
