# INVENTAIRE — Cartographie du code au démarrage de la refonte

> **Phase 0 de la ROADMAP.** Statut par composant : **GARDER** (utilisé tel quel) / **MIGRER** (retravaillé pour le runtime) / **GELER** (intact, hors périmètre jusqu'à décongélation) / **SUPPRIMER** (code mort confirmé ou remplacé après bascule).
>
> Réalisé le 2026-08-14 par audit exhaustif des 12 crates du workspace + frontend.

---

## 1. Vue d'ensemble

| Périmètre | Taille | Contenu |
|---|---|---|
| `backend/crates/` | 12 crates | api, data, db, common, smc, strategies, indicators, risk, ml, llm, news, notifications |
| `api/src/` | 109 fichiers, ~18 000 lignes | 40+ domaines de handlers HTTP + **24 workers de fond** |
| `smc/src/v12/` | 29 fichiers, ~8 700 lignes | Moteur Pine v12 — 178 tests, événementiel par barre |
| `db/` | 44 modules, 37 tables, 66 migrations | SQLite (sqlx), WAL |
| `frontend/` | 18 vues, ~110 composants, 12 stores | Vue 3 + Tauri |

## 2. Découvertes structurantes (impact direct phase 1-2)

1. **`bybit_ws.rs` ne voit pas les ticks** : il filtre `confirm: true` et n'écrit que les bougies fermées. Le runtime tick exigera d'étendre le worker pour capter le flux **non confirmé** (Bybit pousse la kline en formation à chaque mise à jour, `confirm: false`) ou le topic `publicTrade`.
2. **Le moteur v12 est déjà événementiel par barre** : `SmcV12Engine::update(&BarInput) -> SmcOutput` traite UNE barre fermée dans l'ordre exact du Pine (ATR → pivots → structure → BOS → MSS → liquidités → sweep → FVG → OB → … → scoring → signals → lifecycle). C'est le modèle bar-replay. La migration phase 2 = ajouter la couche intrabar (`on_tick`), pas réécrire.
3. **Cinq chemins de signaux parallèles coexistent**, tous timer + bougies fermées en DB :
   - **A** `signal_engine_analyse` (5 min) — `strategies::smc_directional` + gate ML + LLM
   - **B** `smc_boucle` (15 min) — `smc::scorer` v1 + calibration + few-shot + Ollama
   - **C** `straddle_boucle` (15 min) — créneaux + gate ML + Ollama
   - **D** `rockets_scan` (5 min) — scan Binance + calibration
   - **E** génération manuelle HTTP
   Ils seront tous remplacés par le runtime, puis supprimés à la bascule (2.8).
4. **Deux moteurs SMC parallèles** (A et B) font double emploi — aucun des deux n'utilise le v12 en continu.
5. **Une ébauche v12 à la demande existe déjà** : `smc_v12_handlers.rs` + `smc_v12_collect.rs` + `smc_v12_out.rs` (POST `/api/smc/v12/analyse`) — à recâbler sur l'état du runtime.
6. **24 workers de fond tournent en permanence** (16 spawnés par `AppState::new()`, 8 par `main()`) : signaux ×4, feedbacks ×3, calibrations ×3, ML ×3, données ×4, divers ×4. La plupart gelés par la roadmap.

---

## 3. Décision par crate

| Crate | Statut | Raison |
|---|---|---|
| `common` | **GARDER** | Types partagés (Candle, Signal, Timeframe, Direction, Asset, erreurs, helpers Paris). Feuille du graphe, zéro dépendance. |
| `db` | **GARDER** | Archive + config. Sera complétée en phase 1 par la table d'observation du runtime. |
| `data` | **MIGRER** (bybit_ws) / **GARDER** (reste) / **GELER** (dukascopy) | Voir détail §4. |
| `smc` | **MIGRER** (v12) / **GELER** (v1) | V12 = futur plugin. V1 encore utilisé par les chemins A/B (remplacés en phase 2). |
| `strategies` | **MIGRER** (partiel) / **SUPPRIMER** (straddle.rs) / **GELER** (rockets) | Voir détail §5. |
| `indicators` | **GARDER** | Utilisé par ml/smc/strategies/api. 2 modules morts supprimables. |
| `risk` | **GELER** | Crate entière non consommée (dépendance déclarée, jamais importée). Fonction redeviendra pertinente post-phase 3. |
| `ml` | **GELER** | Gel protecteur. Modèles sur disque conservés. |
| `llm` | **GELER** | Ollama + Anthropic. Recâblé post-phase 4. |
| `news` | **GELER** | Deviendra producteur séparé en phase 4. |
| `notifications` | **GARDER** | Telegram worker (poll DB 15 s). À brancher en direct sur le runtime en phase 2 pour la latence (< 1 s vs 15 s). |
| `api` | **MIGRER** (cœur) / **GARDER** (CRUD) / **GELER** (le reste) | Voir détail §6. |
| `frontend` | **GARDER** (shell + vues) | Évolutions gelées jusqu'à post-phase 2. |

---

## 4. Crate `data` — détail

| Fichier | Statut | Notes |
|---|---|---|
| `bybit_ws.rs` (697 l.) | **MIGRER** | Symboles/timeframes lus en DB (pipeline piloté UI ✅). **Filtre `confirm: true` → extension requise phase 1.4** : émettre la kline en formation (`confirm: false`, poussée par Bybit à chaque tick) sur un canal mémoire vers le runtime, en plus de l'écriture DB actuelle. |
| `worker_config.rs` / `worker_status.rs` | **GARDER** | Config DB + statut atomique des workers. |
| `providers/binance.rs` | **GARDER** | REST Bybit (nom historique). Mapping Asset→symbole hardcodé (12 assets) — acceptable. |
| `prix_utils.rs` | **GARDER** | Prix spot REST + fallback DB. |
| `dukascopy.rs` (580 l.) | **GELER** | **Complet et testé** (bi5, LZMA, agrégation M1→TF). Phase 5. |
| `lib.rs` (trait `DataProvider`) | **GARDER** | |

## 5. Crates `smc` + `strategies` — détail

### smc/v12/ — **MIGRER** (cœur de la phase 2)

Tous les modules sont événementiels (1 barre fermée par appel), ordre Pine respecté, tests unitaires partout :

| Module | Lignes | Rôle |
|---|---|---|
| `mod.rs` | 396 | `SmcV12Engine::update(&BarInput) -> SmcOutput` — orchestrateur 23 détecteurs |
| `types.rs` | 549 | ~30 structs d'événements/zones, `BarInput`, `SmcOutput` |
| `liquidites.rs` | 592 | PDH/PDL/PWH/PWL + EQH/EQL |
| `mtf.rs` | 521 | Multi-timeframe H1/H4/W1/MN |
| `scoring_v11.rs` | 507 | Score 16 composantes, sticky par OB |
| `signals.rs` | 464 | Génération trades v11 + BSZones, anti-doublon |
| `order_blocks.rs` | 391 | OB ROC + lifecycle 3 états + Breakers |
| `mss.rs` | 381 | MSS/CHOCH |
| `lifecycle.rs` | 372 | Fill/SL/BE/TP intrabar (déjà conçu intrabar !) |
| `sweep.rs` | 322 | Machine 5 phases |
| + 19 autres | ~4 500 | fvg, sentiment, zone_coeur, ndog, calibration, ote, pivots, structure, bos, breaker, propulsion, imbalance, premium_discount, kill_zones, atr, trade, bs_helpers, scoring_bs_zones |

⚠️ `tests.rs` dépend d'un CSV **hors workspace** (`/mnt/IA/nautilus-smc-spike/xauusd_m15.csv`) ; si absent, les tests passent sur 0 barres silencieusement. À recâbler en phase 2 (replay harness).

### smc v1 (racine) — **GELER**

`lib.rs` (scorer batch) + 14 fichiers (imbalance, fibonacci, ifvg, liquidites, choch, order_blocks, bpr, sweep, bos, tendances, kill_zone…). Consommé par les chemins A/B (remplacés en phase 2) et `ml`. **Suppression à trancher après la bascule 2.8.**

### strategies/

| Fichier | Statut | Notes |
|---|---|---|
| `position_tracking.rs` | **MIGRER** | Moteur universel de verdicts (TP partiels, trailing, SL progressif) — réutilisé par les plugins phase 3 |
| `straddle.rs` (StraddleStrategy, StraddleCreneauStrategy) | **SUPPRIMER** | **Code mort confirmé** : jamais instancié hors du crate |
| `straddle_precision.rs` | **GELER** → migrer phase 3 | Utilisé par `straddle_handlers` |
| `smc_directional.rs` | **GELER** | Chemin A — remplacé par le plugin v12 |
| `rockets_indicateurs/niveaux/position/filtres` | **GELER** → migrer phase 3 | Utilisés par rockets_scan/analyse/suivi |
| `lib.rs` (trait `Strategy`) | **GELER** | Le trait batch `analyze(&[Candle])` sera remplacé par le trait `Engine` du runtime |

## 6. Crate `api` — détail

| Groupe | Fichiers | Statut |
|---|---|---|
| **Chemin signaux timer** (le fautif) | `signal_engine*.rs` (5 fichiers), `smc_boucle.rs`, `straddle_boucle*.rs` (3), `rockets_scan.rs`, `prealerte_worker.rs`, `signal_filtre.rs` | **REMPLACER** (phase 2-3) puis **SUPPRIMER** à la bascule |
| Suivi/verdicts | `signaux_handlers.rs` (worker 5 min), `smc_feedback_job.rs`, `straddle_feedback_job.rs`, `straddle_moniteur_position.rs` (60 s), `rockets_suivi*.rs` (3 min) | **REMPLACER** par le lifecycle intrabar du runtime (phase 2.4), suppression après gate 2 |
| v12 à la demande | `smc_v12_handlers.rs`, `smc_v12_collect.rs` (433 l.), `smc_v12_out.rs` (354 l.) | **MIGRER** — deviennent une lecture de l'état du runtime |
| CRUD données/assets/config | `data_handlers.rs`, `data_csv_handlers.rs`, `data_mt5_handlers.rs`, `assets_handlers.rs`, `asset_params_handlers.rs`, `config_handlers.rs`, `worker_handlers.rs`, `dukascopy_handlers.rs` | **GARDER** |
| Prix/WS charts | `prix_handlers.rs`, `prix_stream.rs`, `ws_handlers/` | **GARDER** (présentation ; le `prix_stream` 2 s polling est indépendant du chemin signaux) |
| Calibrations/patterns | `smc_calibration_job.rs`, `straddle_calibration.rs`, `rockets_calibration.rs`, `patterns_echec_job.rs` | **GELER** (post-phase 3) |
| ML | `ml_handlers.rs`, `ml_insights_handlers.rs`, `ml_retrain_*.rs` (4), `scheduler.rs`, `ab_test_handlers.rs` | **GELER** |
| Ollama/LLM | `ollama_*.rs` (8), `prompts_handler.rs` | **GELER** (post-phase 4) |
| News/sentiment | `news_*.rs` (4), `sentiment_*.rs` (3) | **GELER** (phase 4 producteurs) |
| Indicateurs/tendance/volatilité | `indicators_handlers.rs`, `tendance_handlers.rs`, `volatility_handlers.rs` | **GARDER** |
| Calendrier/pips | `calendar_handlers.rs`, `pip_updater.rs` | **GARDER** |
| Analyses hebdo LLM | `smc_analyse_handler.rs`, `rockets_analyse_handler.rs` | **GELER** |
| `state.rs` (16 spawns) + `main.rs` (8 spawns) | — | **MIGRER** — la liste des workers sera réduite : runtime tick + bybit_ws + telegram + calendrier/pips d'abord |

## 7. Code mort confirmé (supprimable sans risque)

| Élément | Preuve |
|---|---|
| `risk` : crate entière | Dépendance déclarée dans api, **aucun import** dans tout le workspace |
| `strategies/straddle.rs` : `StraddleStrategy`, `StraddleCreneauStrategy` | Jamais instanciées hors du crate (le pipeline straddle api utilise ses propres modules) |
| `smc/fibonacci.rs::prix_sur_niveau` | 0 appel dans backend |
| `smc/imbalance.rs::score_pour_direction_legacy` | 0 appel |
| `indicators/supertrend.rs` + `indicators/tendance.rs` | 0 usage externe |
| `data/modele_rf.json` (1 Mo) | Aucune référence dans le code (héritage) |
| `notifications/telegram.rs::notifier_telegram_rocket` | `#[allow(dead_code)]`, remplacé par le worker |

## 8. Anomalies à corriger en temps utile

| Anomalie | Quand |
|---|---|
| `smc/v12/tests.rs` : CSV chemin absolu hors workspace, tests silencieux sur 0 bars | Phase 2 (replay harness) |
| Modèles ML référencés mais absents (`xgboost_straddle`, `xgboost_smc`, `lstm.pt`) | Gel ML — sans objet tant que gelé |
| `prix_stream.rs` : polling REST 2 s derrière une interface WS | Post-phase 2 (présentation) |
| `AppState::new()` spawn 16 workers d'un coup, sans contrôle ordre/pannes | Phase 1 (état runtime explicite) |

---

## 9. Points tranchés à la Gate 0 (2026-08-15) — TOUS VALIDÉS

1. **Flux tick Bybit** → **klines non confirmées** (`confirm: false` poussé à chaque tick ; chaque push contient l'OHLC intrabar complet).
2. **Chemins A/B** (signal_engine + smc_boucle) → **éteints dès le début de la phase 2** (le shadow mode les remplace ; suppression à la bascule 2.8).
3. **Code mort §7** → **supprimé le 2026-08-15** : crate `risk` (workspace + dep api), `strategies/straddle.rs`, `indicators/supertrend.rs`, `indicators/tendance.rs`, `smc::prix_sur_niveau`, `smc::score_pour_direction_legacy`, `notifications` bloc mort + dépendance `db` retirée, `data/modele_rf.json`. Workspace `cargo check` OK, 211 tests verts.
4. **smc v1 + smc_directional** → **suppression à la bascule 2.8**, sous réserve que `ml` (gelé) compile sans.
