# Audit nickel - Checklist actionnable

Date: 28 mars 2026
Objectif: obtenir un audit 100% vert (taille, build, tests, hygiene) sans regression.

## 1) Bloquants a corriger en premier

### 1.1 Limite dure >300 lignes
A corriger immediatement (regle projet):
- [ ] backend/crates/ml/src/lstm/mod.rs (321)
- [ ] backend/crates/ml/src/lib.rs (301)

Plan de split recommande:
- [ ] Extraire le clipping/verification dans `backend/crates/ml/src/lstm/robustesse.rs`
- [ ] Extraire la boucle d'entrainement dans `backend/crates/ml/src/lstm/entrainement.rs`
- [ ] Garder `mod.rs` comme facade (struct + wiring)
- [ ] Extraire le code pipeline non coeur vers `backend/crates/ml/src/pipeline_runtime.rs` ou equivalent

Critere de fin:
- [ ] Aucun fichier backend >300 lignes

### 1.2 Duplicats frontend `.vue.js` hors norme
Ces fichiers depassent 300 et polluent l'audit:
- [ ] frontend/src/components/common/DashboardPrixStrip.vue.js (362)
- [ ] frontend/src/components/common/MarketClocks.vue.js (345)
- [ ] frontend/src/components/common/IndicatorModal.vue.js (852)
- [ ] frontend/src/components/common/ChartPrixStats.vue.js (310)
- [ ] frontend/src/components/common/ChartImportPanel.vue.js (370)
- [ ] frontend/src/components/common/SignalModal.vue.js (394)
- [ ] frontend/src/views/PnLView.vue.js (383)
- [ ] frontend/src/views/SMCAnalyzerView.vue.js (383)
- [ ] frontend/src/views/ChartsView.vue.js (401)
- [ ] frontend/src/views/HistoryView.vue.js (376)
- [ ] frontend/src/views/SMCCoachView.vue.js (305)
- [ ] frontend/src/views/SettingsView.vue.js (399)
- [ ] frontend/src/views/HeatmapView.vue.js (340)

Plan recommande (Phase 4.1):
- [ ] Verifier qu'aucun import runtime ne cible `*.vue.js`
- [ ] Supprimer tous les `*.vue.js` dans `frontend/src/` (artifacts de transpilation)
- [ ] Renforcer exclusions pour eviter leur regeneration dans `src/`

Critere de fin:
- [ ] Plus aucun `frontend/src/**/*.vue.js`
- [ ] Build frontend OK apres suppression

## 2) Hygiene compilation (non bloquant mais requis pour audit propre)

Warnings actuels a nettoyer:
- [ ] backend/crates/ml/src/lstm/couche.rs:179 -> `biais_mut` jamais utilise
- [ ] backend/crates/api/src/rockets_scan.rs:6 -> import `RocketsConfig` inutilise
- [ ] backend/crates/api/src/straddle_signal_handler.rs:4 -> import `serde::Deserialize` inutilise
- [ ] backend/crates/api/src/straddle_signal_handler.rs:9 -> import `OllamaMsg` inutilise

Critere de fin:
- [ ] `cargo clippy --workspace -- -D warnings` passe
- [ ] Plus de warnings `unused_*` sur le backend

## 3) Validation fonctionnelle des changements Phase 3.4 et 3.5

### 3.4 Ollama rate limiting
- [ ] Verifier que tous les appels passent bien par `appeler_ollama`
- [ ] Verifier semaphore global limite a 2 appels concurrents
- [ ] Verifier timeout global 60s (degradation propre)

Tests manuels:
- [ ] Lancer plusieurs analyses LLM en rafale depuis l'app
- [ ] Confirmer absence de freeze/crash
- [ ] Confirmer erreurs propres si Ollama lent/injoignable

### 3.5 Surveillance accuracy_val
- [ ] Verifier log de demarrage: surveillance 6h active, seuil 52%
- [ ] Verifier methode DB `accuracy_val_recente(3)`
- [ ] Verifier qu'un retrain auto est declenche si moyenne <52%

Tests manuels:
- [ ] Injecter/forcer un historique faible puis verifier le retrain auto
- [ ] Verifier qu'en regime normal le worker n'entraine pas inutilement

## 4) Commandes d'audit final (ordre conseille)

- [ ] `bash scripts/check-file-size.sh`
- [ ] `cd backend && cargo fmt --all --check`
- [ ] `cd backend && cargo clippy --workspace -- -D warnings`
- [ ] `cd backend && cargo test --workspace`
- [ ] `bash scripts/test.sh`

Option frontend (si scope Phase 4.1):
- [ ] `cd frontend && npm run build`
- [ ] `cd frontend && npm run test` (si suite dispo)

## 5) Definition de "audit nickel"

Audit considere nickel quand:
- [ ] 0 fichier >300 lignes
- [ ] 0 warning clippy/compiler sur le scope modifie
- [ ] tests backend verts
- [ ] scripts projet verts
- [ ] verification manuelle app Tauri faite (ML monitoring + LLM + backtest)

## 6) Ordre d'execution recommande (rapide)

1. Corriger les 2 fichiers backend >300
2. Supprimer les duplicats `*.vue.js` dans `frontend/src`
3. Nettoyer les imports/methodes inutilises
4. Lancer la batterie de commandes section 4
5. Tester dans l'app Tauri et valider les regressions
