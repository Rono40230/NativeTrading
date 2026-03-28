# TODO — Restant à faire
Date: 28 mars 2026

---

## Phase 4 — Architecture/Nettoyage ✅ TERMINÉ

### 4.2 Supprimer code ML mort (RandomForest) ✅ TERMINÉ (commit 5d4469e)
### 4.3 Extraction logique métier hors couche API ✅ TERMINÉ (commit 5d4469e)
### 4.4 WebSocket signaux ✅ VALIDÉ en app
- [x] Signal Engine démarré, `/api/signal-engine/status` HTTP 200
- [x] Rockets scan actif (268 candidats) — logique post-extraction fonctionnelle

### Tests de sortie Phase 4 ✅ TERMINÉ
- [x] `bash scripts/check-file-size.sh` → 0 fichier >300 lignes
- [x] `cd backend && cargo clippy --workspace -- -D warnings` → 0 warnings
- [x] `cd backend && cargo test --workspace` → 59/59 OK
- [x] App Tauri ouverte, API répond, Rockets scan 268 candidats

---

## Phase 3 — Validations manuelles en app (3.4 + 3.5)

### Ollama rate limiting (impl. faite, valider en app)
- [ ] Lancer plusieurs analyses LLM en rafale → pas de freeze/crash
- [ ] Vérifier dégradation propre si Ollama lent/injoignable

### Surveillance accuracy_val ✅ VALIDÉ en app
- [x] Log démarrage: `"Surveillance ML activée (check toutes les 6h, seuil 52%)"` ✅
- [ ] Forcer accuracy_val bas → warning UI actif (test optionnel)

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
