# TODO — Restant à faire
Date: 29 mars 2026

---

## Validations manuelles en app

- [ ] Lancer plusieurs analyses LLM en rafale → pas de freeze/crash
- [ ] Vérifier dégradation propre si Ollama lent/injoignable

---

## Backtest — Fonctionnalités manquantes

- [x] Break-even (`be_atr_mult` dans `BacktestEngine`, câblé API, fixé à 0 en dur — retiré de l'UI) ✅ 29/03/2026
- [x] TP partiels Straddle : ⅓ à TP1 (SL→BE), ⅓ à TP2 (SL→TP1), trailing ATR actif après TP2 ✅ 29/03/2026
- [x] Straddle hybride : jambe survivante bascule en SMC directionnel (pyramidalisation) ✅ 29/03/2026
- [x] Toggle vente partielle (⅓ ou lot entier) en UI ✅ 29/03/2026

---

## UI backtest — Pilotage live

- [x] Debounce + relance auto du backtest à chaque modification des params ✅ 29/03/2026
- [x] Affichage immédiat des résultats recalculés ✅ 29/03/2026

---

## Tests de sortie Phase 5

- [x] Changer ATRPeriod / TP ratio → résultats backtest changent de façon cohérente (validé en app)
- [ ] Vérifier persistance des paramètres après reload UI
- [ ] Comparaison Rust vs MT5 sur cas témoin (hors scope phase actuelle)
- [x] `cargo test --workspace` ✅ 29/03/2026

---

## Alarme signal — Vérifications app

- [x] Déclencher un signal test → modale apparaît sur toutes les pages ✅ 29/03/2026
- [x] Navigation N/N avec plusieurs signaux simultanés ✅ 29/03/2026
- [x] Fermeture uniquement par X (pas de fermeture accidentelle) ✅ 29/03/2026
- [x] Son Tauri joué à chaque nouveau signal ✅ 29/03/2026
- [ ] Bouton 🔍 tableau SMC → pré-remplit correctement `/smc/analyser`
oui