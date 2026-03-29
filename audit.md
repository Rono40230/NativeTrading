# TODO — Restant à faire
Date: 29 mars 2026

---

## Validations manuelles en app

- [ ] Lancer plusieurs analyses LLM en rafale → pas de freeze/crash
- [ ] Vérifier dégradation propre si Ollama lent/injoignable

---

## Backtest — Fonctionnalités manquantes

- [x] Break-even (`be_atr_mult` dans `BacktestEngine`, slider UI, câblé API) ✅ 29/03/2026
- [ ] TP partiels Straddle : ⅓ à TP1 (SL→BE), trailing actif, sortie finale au trailing après TP2

---

## UI backtest — Pilotage live

- [ ] Debounce + relance auto du backtest à chaque modification des params
- [ ] Affichage immédiat des résultats recalculés

---

## Tests de sortie Phase 5

- [ ] Changer ATRPeriod / TP ratio → résultats backtest changent de façon cohérente
- [ ] Vérifier persistance des paramètres après reload UI
- [ ] Comparaison Rust vs MT5 sur cas témoin (écart acceptable défini)
- [ ] `cargo test --workspace`

---

## Alarme signal — Vérifications app

- [ ] Déclencher un signal test → modale apparaît sur toutes les pages
- [ ] Navigation N/N avec plusieurs signaux simultanés
- [ ] Fermeture uniquement par X (pas de fermeture accidentelle)
- [ ] Son Tauri joué à chaque nouveau signal
- [ ] Bouton 🔍 tableau SMC → pré-remplit correctement `/smc/analyser`
