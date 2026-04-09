# 🗺️ ROADMAP — Native Trading AI
> Dernière mise à jour : 8 avril 2026

## Vision finale
Un système de trading algorithmique local, cohérent et professionnel :
- 3 stratégies (Rockets, SMC, Straddle) avec suivi live identique et paramétrable
- Backtest unifié produisant exactement les mêmes calculs que le live
- ML hybride (LSTM + XGBoost) utilisé par les 3 stratégies
- Gestion du risque centralisée et configurable par asset
- UI native (Tauri) sans ambiguïté de navigation

---

## 📊 ÉTAT ACTUEL — BILAN

### ✅ Ce qui fonctionne
- Worker live Rockets (`rockets_suivi.rs`) — vente partielle TP1/TP2, option 1/2 configurable
- Worker live SMC (`smc_feedback_job.rs`) — trailing stop après TP2, Kill Zone désactivable
- Worker live Straddle (`straddle_feedback_job.rs`) — jambe survivante → trailing, suivi indépendant
- Backtest Straddle unifié avec `vente_partielle: bool` configurable, R:R aligné
- Risk management global (2% max/trade, 3 positions max, 20% drawdown)
- Pipeline ML hybride (LSTM 60% + XGBoost 40%) — inférence <200ms
- Prix temps réel : WebSocket Binance + fallback HTTP + Yahoo Finance (non-crypto)
- Params SMC/Straddle configurables en DB (`smc_params`, `straddle_params`, `asset_params`)
- Table `asset_params` + sizing par asset + `pip_updater` job (Phase 2 ✅)
- `AssetParamsPanel.vue` — tableau momentum éditable en frontend
- Kill Zone désactivable en DB + toggle frontend (Phase 3.1 ✅)
- Endpoint `GET /api/smc/score-debug` — diagnostic JSON détaillé (Phase 3.2 ✅)
- ML confirme direction SMC → bonus +15/+20 pts dans scoring (Phase 3.3 ✅)
- Gate ML branché dans Rockets scan (stub, actif mais ne rejette pas encore)

### 🟡 Ce qui est partiellement implémenté
- Gate ML Rockets : pipeline passé, stub `ml_rejette_rocket()` retourne `false` → TODO bougies DB
- Straddle : non connecté au ML (ni bonus, ni gate)
- ML ne collecte pas les outcomes des trades → pas encore de feedback loop

### 🔴 Ce qui manque
- Pages backtest ambiguës (SmcBacktestsView → pointe vers StraddleView)
- TP3 / trailing stop non testé/validé en backtest "heure précise"
- ML feedback loop : collecte résultats → réentraînement → suggestions paramètres (Phase 8)

---

## 🏗️ PHASES

Les phases sont ordonnées par impact sur la cohérence du système.
Chaque phase doit passer l'audit `.vibe/bin/audit.sh` avant commit.

---

## PHASE 1 — Suivi live unifié et configurable
**Priorité : CRITIQUE — sans ça, les 3 stratégies se comportent différemment**

### 1.1 Option 1 / Option 2 configurable par stratégie ✅
**Objectif** : Chaque stratégie peut basculer entre vente partielle (Option 1) et lot entier (Option 2).

**Backend** :
- [x] Colonne `vente_partielle BOOLEAN DEFAULT TRUE` dans `smc_params`, `straddle_params`, `rockets_config`
- [x] `rockets_suivi.rs` : option 1/2 branchée (vente ⅓ + SL Break-Even / lot entier)
- [x] `smc_feedback_job.rs` : même traitement
- [x] `straddle_feedback_job.rs` : même traitement sur chaque jambe

**Frontend** :
- [x] `StrategiesParamsPanel.vue` : toggles Option 1/2 par stratégie
- [x] API PATCH `/api/strategies/params/{strategy}` → mise à jour flag

**Tests** :
- [ ] Test unitaire Rust : `test_suivi_option1_tp1_deplace_sl`
- [ ] Test unitaire Rust : `test_suivi_option2_pas_vente_partielle`

---

### 1.2 Alignement SL/TP — Règle universelle R:R ✅
**Objectif** : Les 3 stratégies respectent la même formule :

```
SL   = Entrée ± (ATR × sl_mult)         → perte = 1R
TP1  = Entrée ± (ATR × (sl_mult + 1R))  → gain = +1R
TP2  = Entrée ± (ATR × (sl_mult + 2R))  → gain = +2R
TP3  = Stop suiveur ATR × trailing_mult
```

- [x] Multiplicateurs Rockets audités et alignés dans `rockets_indicateurs.rs`
- [x] SMC aligné : `atr_tp1 = sl_mult + 1.0`
- [x] Straddle aligné : `tp1 = sl_mult + 1.0`
- [x] Params DB reflètent les nouvelles valeurs

---

### 1.3 Straddle — Suivi indépendant par jambe ✅
**Objectif** : Chaque jambe (LONG + SHORT) a son SL propre qui progresse indépendamment.

- [x] `sl_courant_long` et `sl_courant_short` stockés séparément en DB
- [x] Progression Break-Even / SL→TP1 indépendante sur chaque jambe
- [x] Si jambe 1 ferme → jambe 2 bascule en SMC pyramidal (vérifié)
- [x] Les 2 jambes affichées dans `StraddleSignauxView.vue`

---

## PHASE 2 — Paramètres par asset (tableau momentum) ✅
**Priorité : HAUTE — les multiplicateurs globaux ne sont pas adaptés à chaque marché**

### 2.1 Table `asset_params` en DB ✅
**Objectif** : Chaque asset a ses propres paramètres de sizing et risque.

- [x] Migration SQLx + seed pour BTC, ETH, XAUUSD, EURUSD, GBPUSD
- [x] API GET/PUT `/api/assets/params` opérationnelle
- [x] `asset_params` intégré dans `calculer_taille_position()` de `risk/src/lib.rs`
- [x] Job `pip_updater` — rafraîchit `valeur_pips` depuis prix Binance/Yahoo

### 2.2 Frontend — Tableau momentum ✅
- [x] `AssetParamsPanel.vue` — tableau éditable par asset (Capital, Valeur pips, SL pips, Risque %, Lot calculé)
- [x] `useAssetParamsStore` — store Pinia dédié
- [x] Valeur pips en lecture seule (mise à jour auto par le backend)

---

## PHASE 3 — Correction génération signal SMC ✅
**Priorité : HAUTE — SMC produit 0 signal actuellement**

### 3.1 Kill Zone désactivable ✅
- [x] Migration `0038_smc_kill_zone_flag.sql` — colonne `kill_zone_filtre INTEGER DEFAULT 1`
- [x] `SmcParams.kill_zone_filtre: bool` — lecture/sauvegarde en DB
- [x] `SmcDirectionalStrategy::analyze()` — conditionnel `if self.params.kill_zone_filtre && !est_en_kill_zone(...)`
- [x] Log `debug` quand signal bloqué par Kill Zone
- [x] Toggle "Kill Zone ICT" dans `StrategiesParamsPanel.vue` (London 07h-10h · NY 13h30-16h30 UTC)

### 3.2 Endpoint diagnostic score ✅
- [x] `GET /api/smc/score-debug?asset=EURUSD&timeframe=M15` → JSON complet :
  - score par composant (tendance, order_block, imbalance, ifvg, fib, ml)
  - kill_zone status, sweep détecté, bloqueurs[], signal_emis
- [x] Route enregistrée dans `routes.rs`

### 3.3 Intégration ML dans SMC et Rockets ✅ (partiel)
- [x] SMC : prédiction ML comme 6e critère — `direction_ml == direction_smc` → **+15 pts** ; `confiance > 0.7` → **+20 pts**
- [x] Rockets : `pipeline_ml` passé au worker scan, gate `ml_rejette_rocket()` branché
- [⚠️] Gate Rockets : stub actif mais retourne toujours `false` — TODO bougies DB dans ce contexte
- [ ] Straddle : non encore connecté au ML
- [ ] Documenter dans les prompts Ollama correspondants

---

## PHASE 4 — Unification Backtest
**Priorité : MOYENNE — impact sur fiabilité des résultats de simulation**

### 4.1 Valider TP3 dans backtest "heure précise"
- [ ] Lire `straddle_slot_backtest_fenetre.rs` — vérifier si `simuler_sortie_pyramidal()` est appelé
- [ ] Si TP3 absent : brancher `trailing_atr_mult` idem `straddle_slot_backtest.rs`
- [ ] Tester sur 30 jours BTC M5 → vérifier que P&L change cohéremment avec TP3

### 4.2 Unifier les calculs (heure précise = créneau horaire)
- [ ] Extraire `BacktestEngine` en struct partagée dans `backtest/src/engine.rs`
- [ ] Les 2 modes (`fenetre` et `creneau`) utilisent le même engine, seul le filtre temporel diffère
- [ ] Vérifier résultats identiques sur même créneau/même jour → delta = 0

### 4.3 Clarifier la navigation frontend
- [ ] `SmcBacktestsView.vue` → renommer en `BacktestView.vue` (backtest unifié Straddle)
- [ ] Supprimer `StraddleView.vue` si doublon avec backtest
- [ ] 2 onglets dans la même vue : [Heure précise] | [Créneau horaire]
- [ ] Vérifier les routes `router/` pour supprimer les entrées orphelines

---

## PHASE 5 — Qualité et robustesse
**Priorité : CONTINUE — à maintenir tout au long des autres phases**

### 5.1 Couverture tests
- [ ] Risk management : tests >80% couverture (drawdown, position sizing, exposition)
- [ ] ML pipeline : test inférence <200ms sur fixtures de 60 bougies
- [ ] `calculer_verdict_rocket()` : test sur scénario SL/TP1/TP2/TP3/expiration
- [ ] `smc_feedback_job.rs` : mêmes 5 scénarios

### 5.2 Observabilité
- [ ] Tous les workers logguent leur durée d'exécution (`tracing::debug!("Worker: {:?}", dur)`)
- [ ] Endpoint GET `/api/health` → statut workers + dernier tick + nb signaux actifs
- [ ] Widget santé système dans Dashboard (fond vert/orange/rouge selon état workers)

### 5.3 Nettoyage dette technique
- [ ] Vérifier et supprimer `App.jsx` (fichier React à la racine — doublon ?)
- [ ] Vérifier `frontend/src/main.js` vs `main.ts` (doublon ?)
- [ ] Passer `rockets_config` au même pattern que `smc_params` (struct + DB centralisée)
- [ ] Vérifier taille de tous les fichiers backend — split si >250 lignes

---

## PHASE 6 — Signaux SMC sur graphique
**Priorité : BASSE — amélioration UX**

### 6.1 Affichage signaux SMC sur TradingView
Reproduire ce que fait "OB Ultimate" : marquer les Order Blocks + signaux d'entrée sur le chart.

- [ ] Backend : endpoint GET `/api/smc/signals/chart?asset=BTCUSDT&tf=M5&n=100`
  → retourne liste de `{ timestamp, type: "OB_LONG"|"OB_SHORT"|"ENTRY_LONG"|..., niveau, label }`
- [ ] Frontend `SmcView.vue` : superposer ces markers sur TradingView Lightweight Charts
  - `series.setMarkers(markers)` avec shapes ▲/▼ colorés
  - Légende interactive (clic → détail OB/Imbalance/IFVG)

### 6.2 Page "Analyser un setup" — clarification
Renommer en "Setup Analyzer" et documenter sa finalité :
> Permet à l'utilisateur de charger manuellement un screenshot de chart et de demander à l'IA (Ollama Vision) une analyse SMC du setup.
- [ ] Vérifier que `vision.rs` est connecté et fonctionnel
- [ ] Ajouter placeholder "Glisser-déposer un screenshot ici" si image absente

---

## PHASE 7 — Prix temps réel métaux/forex via Alpaca
**Priorité : BASSE — infrastructure à explorer**

### Contexte
Straddle (XAUUSD, XAGUSD) et SMC (forex, indices) n'ont pas de flux prix temps réel.
Actuellement : worker backend toutes les 5 min via MT5. Conséquence : le tableau
"En cours" peut avoir jusqu'à 5 min de retard pour afficher un TP/SL franchi.

Binance ne propose pas XAUUSD ni XAGUSD (uniquement PAXG/XAUT, tokens crypto adossés à
l'or — corrélés mais pas identiques) et aucun indice boursier.

### Solution candidate : Alpaca Markets
Alpaca propose une API REST + WebSocket gratuite couvrant :
- Métaux : XAUUSD, XAGUSD (via leur feed "crypto" étendu ou "forex")
- Forex majeurs : EURUSD, GBPUSD, USDJPY, etc.
- Indices US : SPX, NDX, DJI (données différées ou live selon abonnement)

### Tâches à explorer
- [ ] Vérifier la disponibilité exacte de XAUUSD/XAGUSD sur l'API free tier Alpaca
- [ ] Comparer latence/précision vs données MT5 sur un même créneau
- [ ] Backend : créer `AlpacaProvider` dans `crates/data/` implémentant le trait `DataProvider`
  - WebSocket `wss://stream.data.alpaca.markets/v1beta3/forex` (ou équivalent métaux)
  - Authentification par clé API (stocker dans `.env`, jamais en dur)
- [ ] Intégrer les prix Alpaca dans `prixStore` (même pattern que Binance)
- [ ] Déclencher `sync_signaux()` sur franchissement TP/SL (même pattern que Rockets)
- [ ] Évaluer si l'abonnement payant est nécessaire pour les données live

### Risques identifiés
- Alpaca peut modifier ses conditions d'accès aux données forex/métaux
- Latence Alpaca ≠ latence MT5 → possible désynchronisation signal/verdict
- Ne remplace pas MT5 pour l'exécution réelle des ordres

---

## PHASE 8 — ML Feedback Loop : apprentissage des 3 stratégies
**Priorité : HAUTE — c'est le cœur du système IA : le ML se nourrit des résultats réels pour s'améliorer et proposer des réajustements de paramètres**

### Vision
Chaque trade clôturé devient une donnée d'entraînement. Le ML observe ce qui a fonctionné ou échoué sur chaque stratégie, corrèle les features rêelles avec les outcomes, et propose des ajustements de paramètres. L'utilisateur valide ou refuse avant application.

### 8.1 — Table `ml_training_samples` : collecte des outcomes
**Objectif** : Enregistrer, pour chaque signal émis, les features ML utilisées + le résultat final du trade.

```sql
CREATE TABLE ml_training_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_signal TEXT NOT NULL,       -- ISO8601 heure d'entrée
    strategie TEXT NOT NULL,              -- "ROCKETS" | "SMC" | "STRADDLE"
    asset TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    direction TEXT NOT NULL,              -- "LONG" | "SHORT"
    features_json TEXT NOT NULL,          -- snapshot des 50+ features au moment du signal
    prediction_ml REAL,                   -- confiance ML au moment du signal (0.0-1.0)
    direction_ml TEXT,                    -- direction prédite par le ML
    outcome TEXT,                         -- "TP1" | "TP2" | "TP3" | "SL" | "EXPIRE"
    rr_realise REAL,                      -- R:R effectivement atteint (ex: 1.8)
    duree_minutes INTEGER,                -- durée de vie du trade en minutes
    pnl_usd REAL,                         -- P&L en USD (positif = gain)
    cree_le TEXT NOT NULL DEFAULT (datetime('now'))
);
```

- [x] Migration SQLx `0040_ml_training_samples.sql`
- [x] `db/src/ml_samples.rs` — `sauvegarder_sample()` + `compter_nouveaux_samples()`
- [x] Déclencher `sauvegarder_sample()` dans chaque worker feedback **à la clôture** :
  - `rockets_suivi.rs` — à SL/TP1/TP2/TP3/invalide
  - `smc_feedback_job.rs` — idem, direction-aware
  - `straddle_feedback_job.rs` — par verdict global

### 8.2 — Feedback par stratégie : analyse de performance

- [x] `ml/src/feedback_analyser.rs` — structs `AnalyseGlobale`, `SmcAnalyse`, `TrancheStat`
- [x] Minimum 30 samples par stratégie avant activation des suggestions
- [x] API `GET /api/ml/feedback/stats` → JSON stats complètes (win rate, R:R, corrélation ML)

### 8.3 — Suggestions de paramètres automatiques

- [x] `ml/src/params_suggester.rs` — `generer_suggestions(analyse, params_actuels)` → 3 règles (score_min, kill_zone, atr_sl)
- [x] API `GET /api/ml/suggestions` → liste de `SuggestionParams` triée par gain estimé
- [x] API `POST /api/ml/suggestions/appliquer` → met à jour la table de params correspondante
- [x] Historique des suggestions appliquées en DB (`ml_suggestions_log`, migration 0039)

### 8.4 — Réentraînement incrémental

- [x] `api/src/ml_retrain_handler.rs` — `RetainState` + handlers POST/GET
  - Rollback automatique si accuracy < baseline − 2% (`std::fs::copy` backup → restore)
- [x] API `POST /api/ml/retrain` → déclenche réentraînement async, retourne `{ job_id, status }`
- [x] API `GET /api/ml/retrain/status/{job_id}` + `GET /api/ml/retrain/last`
- [x] Déclenchement automatique si `nb_nouveaux_samples >= 100` dans les 24h (scheduler)

### 8.5 — Frontend : panneau "ML Insights"

- [x] `MlInsightsView.vue` — 3 onglets : Performance | Suggestions | Réentraînement
- [x] `useMlInsightsStore` — store Pinia : stats, suggestions, statut retraining, polling 3s
- [x] `api.ml_insights.ts` — service HTTP complet
- [x] Route `/ml-insights` dans `router/index.ts`
- [x] Entrée dans la sidebar (🧠 ML Insights)

### 8.6 — Connection Straddle au ML (complète)

- [x] `straddle_ml_gate.rs` — gate isolé : confiance > 0.75 → skip Straddle ; 0.45–0.55 → bonus contexte Ollama
- [x] `straddle_boucle.rs` — gate appelé avant Ollama, signature mise à jour
- [x] Samples Straddle enregistrés dans `ml_training_samples` à la clôture

---

## 📅 ORDRE D'EXÉCUTION RECOMMANDÉ

```
✅ Semaine 1  → Phase 1 (Option 1/2 + R:R + jambes Straddle)
✅ Semaine 2  → Phase 2 (asset_params + tableau momentum)
✅ Semaine 3  → Phase 3 (Kill Zone + score-debug + ML dans SMC/Rockets)
✅ Semaine 4  → Phase 8 (ML feedback loop complet — collecte + rétro + suggestions)
   Semaine 5  → Phase 3 suite (gate Rockets réel + Straddle ML)
   Semaine 6  → Phase 4 (unification backtest)
   Semaine 7  → Phase 5 (tests + observabilité + nettoyage)
   Semaine 8+ → Phase 6 (signaux chart SMC)
Phase 7      → Prix temps réel métaux/forex via Alpaca (à planifier après Phase 6)
```

---

## � RÈGLES ABSOLUES (non négociables)

### Règle 1 — Zéro régression
Avant toute implémentation, analyser les conséquences sur le code existant :
- Lister les fichiers impactés (directs et indirects)
- Identifier les appels entrants vers les fonctions modifiées
- Vérifier que les signatures d'API REST ne changent pas sans migration
- Lancer `cargo build --workspace` + `cargo test --workspace` après chaque modification
- Si un test existant échoue → corriger AVANT de continuer, jamais ignorer

### Règle 2 — Explication avant action
Pour chaque tâche non triviale, fournir AVANT de toucher au code :
1. **Ce qui va être modifié** : liste des fichiers + fonctions concernées
2. **Pourquoi** : justification métier ou technique
3. **Risques identifiés** : ce qui pourrait casser
4. **Plan de rollback** : comment revenir si ça échoue
→ L'utilisateur valide (explicitement ou par silence >30s) avant l'implémentation.

---

## �🔁 RÈGLES DE WORKFLOW (rappel)

Chaque item coché = audit obligatoire avant commit :
```bash
./.vibe/bin/audit.sh        # Clippy + tests + taille fichiers + zero-unwrap
cargo test --workspace      # Tous les tests backend
cd frontend && npm run test # Tests Vue
```

Signal d'alerte :
- Fichier ≥ 250 lignes → split immédiat
- `unwrap()` / `console.log()` → bloquant
- Calcul métier côté Vue → interdit (tout passe par le backend)
