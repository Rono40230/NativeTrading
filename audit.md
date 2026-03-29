# TODO — Restant à faire
Date: 29 mars 2026

---

## 🔍 AUDIT — 29 mars 2026

### ✅ Rust fmt
- `cargo fmt --all --check` → diffs corrigés automatiquement (ollama_handlers, main.rs, signal_engine_analyse, etc.)

### ✅ Rust Clippy
- 1 erreur corrigée : argument nommé `seuil_score` jamais utilisé dans le template de prompt → `{seuil_score:.0}` inséré dans la ligne de critère
- `cargo clippy --workspace -- -D warnings` → **0 warning** ✅

### ✅ Security (cargo-audit)
- 397 crates scannées — aucune vulnérabilité critique identifiée

### ✅ Rust : zéro unwrap/panic en production
- 23 occurrences `unwrap()`/`expect()` → toutes dans des blocs `#[cfg(test)]` ✅

### ✅ TypeScript : zéro erreur
- VS Code diagnostics → **0 erreur** sur tout `frontend/src/` ✅

### ✅ Frontend : zéro console.log / debugger
- Aucun `console.log`, `debugger`, `alert()` dans `*.ts` et `*.vue` ✅

### ✅ Tests Rust
- `cargo test --workspace --no-run` → compilation OK, aucun échec de build

### ❌ Taille fichiers — À refactoriser
Fichiers dépassant 300 lignes (limite dure) :
- `backend/crates/backtest/src/lib.rs` — **316 lignes**
- `backend/crates/api/src/main.rs` — **312 lignes**
- `backend/crates/ml/src/lib.rs` — **302 lignes**
- `frontend/src/services/api.service.ts` — **319 lignes**
- `frontend/src/views/PnLView.vue` — **313 lignes**

### Verdict global : 🟡 ATTENTION
**Rust codebase** : ✅ prêt au commit  
**Frontend** : ✅ prêt au commit  
**Bloquants** : aucun — mais 5 fichiers dépassent la limite 300 lignes (refactoring recommandé avant prochaine feature)

---

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

### 5.1 Paramètres Straddle (BiDiParams) ✅ TERMINÉ (commit 57595b4)
- [x] Struct params dédié
- [x] GET/PUT API params Straddle
- [x] Persistance DB
- [x] Backtest Straddle piloté par ces paramètres

### 5.2 Paramètres SMC (UniParams) ✅ TERMINÉ (commit 57595b4)
- [x] Struct params dédiée
- [x] GET/PUT API params SMC
- [x] Persistance DB
- [x] Backtest SMC piloté par ces paramètres

### 5.3 Fonctionnalités backtest manquantes
- [ ] Break-even
- [x] Trailing stop ✅ TERMINÉ (commit 57595b4)
- [ ] Expiration
- [ ] TP partiels

### 5.4 UI de pilotage backtest
- [x] Formulaires params en modales dans P&L (Straddle) et Analyse IA SMC ✅ TERMINÉ (29 mars)
- [ ] Debounce + relance auto du backtest à chaque modification
- [ ] Affichage immédiat des résultats recalculés

### Tests de sortie Phase 5
- [ ] Changer ATRPeriod / TP ratio → résultats backtest changent de façon cohérente
- [ ] Vérifier persistance des paramètres après reload UI
- [ ] Comparaison Rust vs MT5 sur cas témoin (écart acceptable défini)
- [ ] `cargo test --workspace`
