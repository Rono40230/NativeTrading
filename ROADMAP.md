# 🗺️ ROADMAP — Native Trading AI
> Dernière mise à jour : 7 avril 2026

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
- Worker live Rockets (`rockets_suivi.rs`) — vente partielle TP1/TP2 implémentée
- Worker live SMC (`smc_feedback_job.rs`) — trailing stop après TP2
- Worker live Straddle (`straddle_feedback_job.rs`) — jambe survivante → trailing
- Backtest Straddle unifié avec `vente_partielle: bool` configurable
- Risk management global (2% max/trade, 3 positions max, 20% drawdown)
- Pipeline ML hybride (LSTM 60% + XGBoost 40%) — inférence <200ms
- Prix temps réel : WebSocket Binance + fallback HTTP + Yahoo Finance (non-crypto)
- Params SMC/Straddle configurables en DB (`smc_params`, `straddle_params`)

### 🔴 Ce qui manque ou est incohérent
- Option 1 vs Option 2 non exposée en config frontend pour Rockets/SMC
- Aucun paramètre différencié par asset (BTC utilise les mêmes multiplieurs qu'EURUSD)
- SMC génère peu/aucun signal à cause du check Kill Zone trop restrictif
- ML non utilisé en génération de signal Rockets et SMC (seulement Straddle)
- Pages backtest ambiguës (SmcBacktestsView → pointe vers StraddleView)
- Tableau momentum (Lot, SL pips, Risque %, Valeur pips) absent du frontend
- TP3 / trailing stop non testé/validé en backtest "heure précise"

---

## 🏗️ PHASES

Les phases sont ordonnées par impact sur la cohérence du système.
Chaque phase doit passer l'audit `.vibe/bin/audit.sh` avant commit.

---

## PHASE 1 — Suivi live unifié et configurable
**Priorité : CRITIQUE — sans ça, les 3 stratégies se comportent différemment**

### 1.1 Option 1 / Option 2 configurable par stratégie
**Objectif** : Chaque stratégie peut basculer entre vente partielle (Option 1) et lot entier (Option 2).

**Backend** :
- [ ] Ajouter colonne `vente_partielle BOOLEAN DEFAULT TRUE` dans `smc_params` et `rockets_config`
  - Straddle : déjà dans `BacktestEngine` mais pas dans `straddle_params` en DB → ajouter
- [ ] `rockets_suivi.rs` : lire flag depuis DB, brancher TP1/TP2 selon option
  - Option 1 : vente ⅓ + SL → Break-Even (TP1) / SL → TP1 (TP2)
  - Option 2 : pas de vente + même progression SL
- [ ] `smc_feedback_job.rs` : même traitement
- [ ] `straddle_feedback_job.rs` : même traitement sur chaque jambe

**Frontend** :
- [ ] `SettingsView.vue` ou `StrategiesParamsPanel.vue` : ajouter toggle Option 1/2 par stratégie
- [ ] API PATCH `/api/strategies/params/{strategy}` → mettre à jour flag

**Tests** :
- [ ] Test unitaire Rust : `test_suivi_option1_tp1_deplace_sl`
- [ ] Test unitaire Rust : `test_suivi_option2_pas_vente_partielle`

---

### 1.2 Alignement SL/TP — Règle universelle R:R
**Objectif** : Les 3 stratégies respectent la même formule :

```
SL   = Entrée ± (ATR × sl_mult)         → perte = 1R
TP1  = Entrée ± (ATR × (sl_mult + 1R))  → gain = +1R
TP2  = Entrée ± (ATR × (sl_mult + 2R))  → gain = +2R
TP3  = Stop suiveur ATR × trailing_mult
```

**Audit actuel** :
| Stratégie | SL mult | TP1 mult | R:R TP1 | TP2 mult | R:R TP2 | Conforme |
|-----------|---------|----------|---------|----------|---------|---------|
| Rockets   | ?       | ?        | ?       | ?        | ?       | ⚠️ vérifier |
| SMC       | 1.0     | 1.5      | 1.5:1   | 3.0      | 3:1     | ⚠️ R:R TP1 < 2:1 |
| Straddle  | 0.5     | 2.0      | 4:1     | 3.5      | 7:1     | ⚠️ non symétrique |

- [ ] Auditer les multiplicateurs Rockets dans `rockets_indicateurs.rs`
- [ ] Aligner SMC : `atr_tp1 = sl_mult + 1.0` (ex: sl=1.0 → tp1=2.0, tp2=3.0)
- [ ] Aligner Straddle : `tp1 = sl_mult + 1.0` (ex: sl=0.5 → tp1=1.5, tp2=2.5)
- [ ] Vérifier que les params DB reflètent les nouvelles valeurs

---

### 1.3 Straddle — Suivi indépendant par jambe
**Objectif** : Chaque jambe (LONG + SHORT) a son SL propre qui progresse indépendamment.

Actuellement `straddle_feedback_job.rs` gère la survie mais pas la progression SL par jambe.

- [ ] Stocker `sl_courant_long` et `sl_courant_short` séparément en DB
- [ ] Appliquer progression Break-Even / SL→TP1 indépendamment sur chaque jambe
- [ ] Si jambe 1 ferme → jambe 2 bascule en SMC pyramidal (déjà implémenté, vérifier)
- [ ] Afficher les 2 jambes dans `StraddleSignauxView.vue`

---

## PHASE 2 — Paramètres par asset (tableau momentum)
**Priorité : HAUTE — les multiplicateurs globaux ne sont pas adaptés à chaque marché**

### 2.1 Table `asset_params` en DB
**Objectif** : Chaque asset a ses propres paramètres de sizing et risque.

```sql
CREATE TABLE asset_params (
    asset TEXT PRIMARY KEY,         -- "BTCUSDT", "XAUUSD", "EURUSD"
    valeur_pips REAL NOT NULL,      -- USD par pip (ex: 10.0 pour forex standard)
    sl_pips REAL NOT NULL,          -- SL en pips par défaut
    risque_pct REAL NOT NULL,       -- % capital risqué (1.0 à 3.0)
    lot_min REAL NOT NULL DEFAULT 0.01,
    lot_max REAL NOT NULL DEFAULT 10.0
);
-- lot = (capital × risque_pct) / (sl_pips × valeur_pips)
```

- [ ] Migration SQLx + seed pour BTC, ETH, XAUUSD, EURUSD, GBPUSD
- [ ] API GET/PUT `/api/assets/params` (lecture + mise à jour live)
- [ ] Intégrer `asset_params` dans `calculer_taille_position()` de `risk/src/lib.rs`
- [ ] Valeur pips à rafraîchir depuis prix actuel (Binance / Yahoo)

### 2.2 Frontend — Tableau momentum
- [ ] Nouvelle section dans `SettingsView.vue` : tableau éditable par asset
  - Colonnes : Asset | Capital | Valeur pips | SL pips | Risque % | Investi | Lot calculé
  - Ligne "Investi" et "Lot" = calculées automatiquement (readonly)
  - Bouton "Sauvegarder" → PATCH `/api/assets/params`
- [ ] `useAssetParamsStore` — store Pinia dédié

---

## PHASE 3 — Correction génération signal SMC
**Priorité : HAUTE — SMC produit 0 signal actuellement**

### 3.1 Diagnostic Kill Zone
Le check `kill_zone::est_en_kill_zone(last_ts)` bloque les signaux hors London/NY.

- [ ] Ajouter paramètre `kill_zone_active: bool` dans `smc_params`
- [ ] Permettre désactivation temporaire pour tests
- [ ] Logger clairement quand un signal est bloqué par Kill Zone (niveau `debug`)
- [ ] Tester génération manuelle via endpoint `/api/smc/test-signal` (dev only)

### 3.2 Calibrage score minimum
- [ ] Vérifier distribution des scores historiques (top 10 dernières bougies)
- [ ] Si score max < 70 → baisser seuil temporairement à 50 pour diagnostic
- [ ] Ajouter endpoint GET `/api/smc/score-debug?asset=BTCUSDT` → renvoie score actuel + détail

### 3.3 Intégration ML dans SMC
Le ML n'est actuellement utilisé qu'en Straddle (indécision). Rockets et SMC l'ignorent.

- [ ] SMC : ajouter prédiction ML (LSTM+XGB) comme 6e critère de scoring (+15 pts max)
  - `direction_ml == direction_smc` → +15 pts
  - `confiance_ml > 0.7` → +5 pts bonus
- [ ] Rockets : utiliser ML pour confirmer/rejeter signal avant émission
  - ML confiant dans direction opposée → ignorer signal Rockets
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

## 📅 ORDRE D'EXÉCUTION RECOMMANDÉ

```
Semaine 1  → Phase 1.1 + 1.2  (Option 1/2 + alignement R:R)
Semaine 2  → Phase 1.3 + Phase 2 (jambes Straddle + tableau momentum)
Semaine 3  → Phase 3 (fix SMC signal + ML dans SMC/Rockets)
Semaine 4  → Phase 4 (unification backtest)
Semaine 5  → Phase 5 (tests + observabilité + nettoyage)
Semaine 6+ → Phase 6 (signaux chart SMC)
Phase 7    → Prix temps réel métaux/forex via Alpaca (à planifier)
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
