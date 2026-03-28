# TODO — Restant à faire
Date: 28 mars 2026

---

## Phase 4 — Architecture/Nettoyage

### 4.2 Supprimer code ML mort (RandomForest)
Zone: `backend/crates/ml`
- [ ] Verifier et supprimer références restantes au RandomForest
- [ ] Vérifier que les types/API restent propres après suppression
Critère: backend ML sans références mortes, `cargo clippy` vert

### 4.3 Extraction logique métier hors couche API
Zone: `backend/crates/api/src`, `backtest/src`, `strategies/src`
- [ ] Déplacer les blocs métier encore dans les handlers API vers les crates dédiées
- [ ] Garder handlers API minces: parse → appel service → réponse
Critère: couche API centrée transport HTTP uniquement

### 4.4 WebSocket signaux — vérification robustesse
- [ ] Tester stabilité en usage prolongé (composable `useSignalEngine`)

### Tests de sortie Phase 4
- [ ] `bash scripts/check-file-size.sh` → 0 fichier >300 lignes
- [ ] `cd backend && cargo clippy --workspace -- -D warnings` → 0 warnings
- [ ] `cd backend && cargo test --workspace`
- [ ] `cd frontend && npm run build`
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
