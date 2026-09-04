# ROADMAP — Native Trading AI

> État au 4 septembre 2026 (après audit complet). Cette roadmap ne contient que
> **ce qu'il reste à faire**. Le détail des phases livrées SMC v12 vit dans
> `docs/AMELIORATIONS_SMC_V12.md`, les études des étapes 3-4 dans `docs/ETAPE3_*.md`
> et `docs/ETAPE4_CALCUL_TRADES.md`.
>
> **État visé en fin de feuille de route : 0 dette technique, 0 code mort,
> 0 fichier inutile.** Chaque chantier se termine par son propre rangement.

---

## État des lieux (bref — pour situer le travail restant)

| Verticale | État | Moteur | Source données | IA |
|---|---|---|---|---|
| **SMC** | Officielle | v12 figé (miroir Pine/Rust/MQL5) + TP1/TP2/TP3/trailing/fractions réglables + armement par couple (H1 désarmé) | XAU/XAG/NAS/SP/DAX : MT5/Axi · BTC : Bybit | Analyse (bouton) + analyste du rapport |
| **Straddle** | Observation | v2 unifié (2 jambes à E, T-10 s, lifecycle commun au tick) | XAU/BTC/NAS/SP : MT5+Axi · annonces US | À construire (après gate 3) |
| **Rockets** | Observation | Scanner D1 /10 + gestion journal (R1→50 %+trailing) — crypto (Binance) + actions US Tiingo | Binance + Tiingo | Catalyseur news + ranker ✅ |

**Infrastructure** : runtime tick intrabar (~1 s mesuré) · EA MT5/Axi (temps réel +
historique 24 mois) · Bybit WS · rejeu paramétrique SMC/straddle couplé aux réglages ·
capital simulé composé (lot au capital d'époque — vérifié) · analyste IA qwen3:32b local
(prompts éditables) · Telegram (imminence seule) · presse traduite FR · centre d'analyse
« Rapport d'activité » (périodes J/S/M, verdicts, assets × TF, IA à la demande) ·
historiques live (polling 5 s, colonne Lot, tri par colonne).

**Chiffres** : 60 629 lignes Rust (17 crates) · 23 506 TS/Vue · 12 261 MQL5 ·
478 tests Rust · 105 routes API (73 consommées par le front, toutes servies).

**Vérités d'audit (04/09)** : cohérence backend↔frontend quasi parfaite, front sans
orphelins — mais dette d'études accumulée (bins, routes, tables, workers legacy),
un bug prompts vision, zéro test frontend.

---

## À faire — par ordre de priorité

### 1. Corrections immédiates (bugs découverts par l'audit)

- [ ] **Prompts vision — analyse graphique dégradée** : `vision_1tf`/`vision_multi_tf`
      sont listés dans l'UI Prompts mais absents du registre `defaults()` →
      `prompt_effectif` retourne `""` (system prompt vide pour l'analyse Claude) et le
      PUT renvoie 404. Brancher les constantes existantes de `ollama/prompts_vision.rs`
      dans `defaults()` — puis relecture qualité des deux prompts (cf. §9).
- [ ] **`ferme_le` de réconciliation** : la réconciliation runtime_replay écrit le ts du
      moment où elle rejoue au lieu du ts réel de clôture → la MFE des passes orphelines
      avale l'après-passe (cas BTC 01/09 : +3,54R affiché pour une excursion réelle de
      +1,59R). Écrire le ts de la barre de clôture réelle de la dernière jambe.
- [ ] **Commentaires mensongers** : `main.rs:167` (endpoint `/api/pre_alertes` inexistant),
      `data_handlers.rs:43` (`POST /api/data/collect` supprimé), `worker_handlers.rs:162`
      (`GET /api/worker/assets` supprimé), TODO `http.client.ts:3` — corriger ou supprimer.

### 2. Grand nettoyage — vers 0 dette, 0 code mort (audit 04/09)

**Backend :**
- [ ] Supprimer les 13 binaires d'étude jetables (`comparatif_be|bpr|mega|sessions|pd|sweep|tp3`,
      `etape4_comparatif`, `passe_finale`, `probe_bpr|confirmation|sessions`, `validation_spx`) —
      décisions toutes actées et figées dans les prompts. **Garder** : `news_collector`,
      `backfill_profond`, `replay_v12`, `debug_zones` (vivants).
- [ ] Supprimer `ollama_signal_ia_handler.rs` (module v1 non routé) + sa déclaration `main.rs`.
- [ ] Supprimer les workers v1 maintenus compilés artificiellement (`rockets_suivi`,
      `demarrer_worker_suivi_signaux` + leurs `let _`) et retirer `#![allow(dead_code)]`
      de `state.rs` une fois le chantier ML (§11) tranché.
- [ ] Tables DB mortes : `DROP smc_analyses_llm`, `temp_metrics`, `positions` (0 lignes, 0 accès).
      ⚠️ `signaux_archive` est l'archive de la purge du 02/09 — **conserver**, documenter.
- [ ] Dépendances : retirer `polars`, `statrs`, `aes-gcm`, `config`, `actix-rt`, `ta`
      (workspace, jamais déclarées) et `uuid` (api).
- [ ] Fichiers racine étrangers : `test_tch.rs`, `output.csv`, `patch_ml_retrain.sh`, `patch_smc.sh`.
- [ ] Fonctions prompts orphelines : `rockets_filtre::filtrer_signal`, `smc_filtre::filtrer_signal_smc`
      (statut DORMANT documenté dans l'UI — supprimer le code, garder la mention).

**Frontend :**
- [ ] Dépendances : `jspdf`, `jspdf-autotable`, `topojson-client`, `world-atlas` (0 import) ;
      `vitest`, `@vue/test-utils`, `jsdom` — soit supprimées, soit réutilisées (§12 tests).
- [ ] Méthodes API mortes : `putWorkerConfig` (+ type `WorkerConfigUpdate`), `traduire`.
- [ ] Fonctions mortes : `jourSemaineParis`, `classeVerdictSignal`, `labelVerdictSignal`, `palierActuel`.
- [ ] ~23 types TS morts + 16 réexports inutiles (`api.service.ts`, `api.types.*`) ;
      8 redirects router défensifs (app Tauri sans deep-links) ; 3 classes CSS mortes
      (`AssetParamsPanel.css`) ; commentaire obsolète `vite.config.ts`.

**Critère de fin de section** : `cargo check --workspace` sans warning `dead_code`/`unused`,
`npm run build` propre, `npm run test` (une fois les tests écrits) vert, aucune table,
fichier ni dépendance sans consommateur.

### 3. Étape 5 — Résiduel : validation numérique du miroir MQL5

- [ ] **EA dans le Strategy Tester** : backtest `smc_ea_v12.mq5` sur une paire/période de
      référence (ex. XAU M15) et comparaison aux chiffres du replay Rust sur les mêmes bornes
      (nombre de signaux, verdicts, R cumulé — écart attendu ≈ 0)
- [ ] **Écart ≠ 0** : le tracer règle par règle jusqu'à la divergence (le miroir est la base
      de l'automatisation future des ordres — il doit être exact)
- [ ] **Pine dans TV** (action propriétaire) : coller le Pine de `docs/reference/`
      dans TradingView sous « Scalp à Nono »
- [ ] **Unité commune points** : le rapport du Strategy Tester s'exprime en points —
      l'unité de validation sera le point (cf. §13 trades individuels)

### 4. Gate 3 — Straddle en conditions réelles

- [ ] Vérifier la journalisation des passes (table signaux, verdicts SL/BE/TS/TimeStop + R)
- [ ] Bilan gate 3 : verdicts, R cumulé, comportement trailing sur les annonces
- [ ] Décision propriétaire : passage Officielle ou ajustements
- [ ] Si Officielle : activer le son Telegram (template prêt, dormant)
- [ ] Rappel money management (décision 04/09) : une passe peut coûter jusqu'à −1,5R
      nominal (jambe −1R + tampon/time-stop de la survivante) — assumé, lot inchangé

### 5. SMC v12 — Surveillance production

- [ ] **SP500 live** : après ~2 semaines, comparer la production réelle au replay (fréquence
      M15/M5, verdicts — règle 30 trades). Divergence marquée → étude calibration dédiée
      (profil actuel = miroir NAS100)
- [ ] **Mega-orders live** : confirmer l'apport +21.3R en réel (delta replay concentré BTC M5 —
      réserve documentée)
- [ ] Tout réglage ne bouge que sur preuve ≥ 30 trades remplis par tranche (anti-overfitting)
- [ ] Ré-armer des couples coupés via l'outil Timeframes par asset (décision 04/09) et
      mesurer l'effet (le comparatif 24 mois reste la référence : M15 +0,051 R/trade)

### 6. Test de vérité au centime

- [ ] Comparer bougie par bougie (OHLCV) nos M1 Axi vs le graphique MT5 sur une session complète
- [ ] Comparer les signaux SMC sur XAU (même source → aucun écart attendu)
- [ ] Si écarts : les tracer et les corriger
- [ ] Documenter le verdict au journal

### 7. Décisions propriétaires en attente (à trancher, puis exécuter)

- [ ] **WR SMC et expirés** : les Expire comptent au dénominateur du WR du re-jeu (59 % avec,
      74 % sans) — même logique que les camemberts (expirés exclus) ?
- [ ] **Routes diagnostics sans consommateur front** : `/api/smc/rejeu`, `/api/straddle/rejeu`,
      `/api/runtime/concordance|replay|emissions` — garder comme outils d'inspection (curl)
      ou les câbler dans la vue Données ?
- [ ] **Endpoints rockets actions non câblés** (`scan`, `prescreen`, `univers`, `contexte`,
      `news/collecter`, `backfill` POST) : câbler dans l'UI Données (déclenchement manuel)
      ou supprimer (les boucles backend tournent déjà seules) ?
- [ ] **`PATCH /api/assets/{id}/ml`** : route morte — supprimer.
- [ ] **`GET /api/presse/briefs/{id}`** : servi jamais appelé — supprimer ou brancher.
- [ ] **Ancien système `rockets_signaux`** (table 0 ligne, 10 fichiers) : purge dans le
      chantier rockets (§10) ou nettoyage immédiat.
- [ ] **Prompts dormants** (`smc_filtre`, `straddle_signal`, `rockets_filtre`) : garder la
      mention DORMANT dans l'UI (retours possibles documentés) après suppression du code mort.

### 8. Rôles IA (après gate 3 / accumulation)

- [ ] **Analyse des passes straddle** : l'analyste lit les passes journalisées (annonce →
      range → fill → verdict R) et explique ce qui marche / coince
- [ ] **Recommandation agenda + minutage** : quels événements valent le coup, à quelle minute
      entrer — propositions validées par le propriétaire dans les réglages
- [ ] **Voile des setups SMC** : l'analyste lit annonces vs confirmés vs dissipés et
      identifie les caractéristiques des setups qui tiennent
- [ ] **Analyse rebranchée** : bouton Analyse SMC reconnecté sur les données propres
      (remplis/jamais remplis/dissipés/verdicts)
- [ ] **Décision sur preuve — filtre temps réel** : les setups dissipés partagent-ils des
      caractéristiques repérables ? → décision sur un éventuel filtre, pas avant
- [ ] **Journalisation du détail scoring** (préalable à l'analyse par confluence) :
      stocker le détail point par point à l'émission du signal

**Conviction IA à l'émission — colonne « IA » des tableaux (SMC + Straddle)** : la colonne
est réservée à la conviction (0-100 + justification en infobulle) donnée par l'analyste à
chaque signal à l'émission (`signaux.llm_conviction/llm_raison` = NULL, reliquat v1).

- [ ] **Prompt dédié** : le signal complet (asset, TF, direction, niveaux, force, contexte)
      → JSON `{conviction 0-100, raison 1-2 phrases}` (même format que le ranker Rockets)
- [ ] **Déclencheur à l'émission** : asynchrone, jamais dans le chemin du signal (R4)
- [ ] **Écriture** : `UPDATE signaux SET llm_conviction, llm_raison` — l'affichage existe
- [ ] **Observation d'abord** : l'IA note, elle ne filtre rien (constitution)
- [ ] **Corrélation sur preuve** : ≥ 30 trades avec conviction → croiser conviction × verdict
      → décision propriétaire sur un éventuel filtre — pas avant

### 9. Revue complète des prompts IA

- [ ] Purge des prompts morts (`smc_signal`, `smc_filtre`, `rockets_opportunites` — cf. §7),
      audit des actifs, alignement sur les mécaniques actées (étapes 3-4 répercutées)
- [ ] **Relecture/correction/amélioration de TOUS les prompts actifs** : cohérence avec la
      constitution (l'IA propose, ne règle jamais), formats JSON robustes (confiance entière,
      replis de parse), ancrage sur les conventions actuelles ($ réels composés, R pondéré/net —
      jamais R de référence ni pips), et vérification que chaque prompt éditable reste
      synchrone avec les mécaniques du moteur qu'il décrit
- [ ] Les définitions injectées (`smc_definition`, `straddle_definition`, `rockets_definition`)
      doivent refléter l'armement actuel (H1 désarmé, TP réglables, tampon straddle)

### 10. Rockets — Extensions

- [ ] **Véto unlocks** : source libre (calendrier public de déverrouillages de tokens)
      → intégrer au scanner (éliminatoire si unlock majeur < 30 jours)
- [ ] **ETF via Tiingo** : lever l'exclusion ETF + profils 2/3/4 % dédiés (répertoire
      NASDAQ Trader) — après validation de l'Observation actions
- [ ] **Analyse par pilier** : l'analyste relie les critères du /10 aux verdicts →
      propositions de recalibrage chiffrées

### 11. Boucle ML — réveiller ou enterrer (décision d'ouverture)

Les onglets « Métriques ML »/« Dashboard LLM » sont branchés sur des endpoints sains mais
vides (tables de feedback à 0 ligne depuis le pivot vers les moteurs déterministes).

- [ ] **Décision préalable** : réveiller la boucle OU supprimer l'UI + tables + code
      (`ml_samples.rs`, `ml_feedback.rs`, `ml_training_samples`, `ml_suggestions_log`,
      `smc_features_snapshot`) — l'état « suspendu » à durée indéterminée est la pire option
- [ ] Si réveil : **alimenter le feedback à la clôture** (features à l'émission + verdict en R
      par stratégie ; rejouer l'existant ~96 SMC + 19 straddle), remettre les vues en données
      (fraîcheur affichée), réveiller le Dashboard LLM (suggestions appliquées manuellement
      uniquement), réentraînement sur décision propriétaire (`POST /api/ml/retrain`),
      seuils d'activation par étage

### 12. Robustesse

- [ ] **Tests frontend** : vitest est installé, 0 test existe — couvrir la logique pure
      (tri d'historique, formateurs R/$/pips, camemberts/lignesClassement, palierMax)
- [ ] **Sauvegarde automatique de la base** : 24 mois d'historique MT5 + tout le vécu dans un
      seul fichier SQLite — backup quotidien horodaté (rétention 30 j) au démarrage du run.sh
- [ ] Vérifier l'affichage des zones SMC sur l'historique Axi profond (2 ans — artefacts au
      changement de source à tracer)
- [ ] ETH : réactiver et re-backfiller si souhaité (purgé par la rétention 24 mois)
- [ ] `worker_historique_mois` (6) vs historique MT5 (24) : harmoniser avec la rétention

### 13. Unités & métriques — résiduels

**Livré le 04/09** : badge R pondéré (référence en infobulle), camemberts/histogramme en
dollars réels, badge straddle/rockets = R réalisé, MFE/Lot/tri des historiques.

- [ ] **Trades individuels → points MT5** : aligner tooltips/historiques sur `_Point`
      (digits du symbole, lu depuis la connexion MT5 sinon convention par asset) —
      l'unité commune de l'étape 5 (§3)
- [ ] **Vigilance sizing** : `taille_pip`/`valeur_pip` alimentent le calcul des lots —
      tout chantier d'unités ne touche QUE l'affichage

### 14. Rapport d'activité — approfondissements

**Livré le 04/09** (phases 1-3) : bloc dashboard, `/analyses` à onglets, IA à la demande
(cache du jour, règle des 30 trades ancrée), croisé asset × TF, prompt `analyse_rapport`
éditable (Configuration & Métriques IA).

- [ ] **Historisation des rapports** : snapshot quotidien persisté (table dédiée) pour
      suivre l'évolution des métriques ET des avis IA jour après jour
- [ ] **Heatmap heure×jour** : contribution $ par créneau horaire (parcours naturel du
      straddle) — réutilise le calcul des créneaux de volatilité

### 15. Exécution réelle — la prochaine frontière

L'app observe, mesure, informe — elle ne passe pas d'ordres. Le miroir MQL5 existe déjà ;
c'est le saut qualitatif.

- [ ] **Décision de principe propriétaire** : un EA exécutant recevant lots/niveaux
      (mode « ordres auto-validés » compatible constitution : l'IA ne décide jamais,
      le moteur déterministe exécute ce qu'il signale déjà)
- [ ] Si acté : cahier des charges (périmètre SMC d'abord, validation humaine par trade
      au démarrage, garde-fous — risque max/jour, kill-switch, journal d'ordres),
      puis EA exécutant + test Strategy Tester avant tout réel

### 16. Fonctionnalités candidates (backlog, non priorisées)

- [ ] **Export du rapport d'activité** (PDF/Markdown) — vraisemblablement l'intention
      initiale de `jspdf` (cf. nettoyage §2 : dépendance à retirer tant que non décidé)
- [ ] **Journal de bord du propriétaire** : notes attachées aux trades (contexte, ressenti,
      décision) — matière première pour l'analyse IA future
- [ ] Agenda intelligent — créneaux de volatilité récurrents (Straddle IA) : calcul
      statistique heure×jour sur 24 mois M1 → endpoint → l'analyste propose armer/ignorer →
      créneaux armés = annonces synthétiques (démarrage en Observation)

---

## Règles transverses (non négociables)

| Règle | Détail |
|---|---|
| **Pine = étalon** | Le moteur v12 est figé sur le Pine (md5 vérifié). Toute déviation = bug Rust/MQL5, jamais « amélioration » |
| **Discussion avant construction** | Chaque étape fait l'objet d'une discussion de définition avant le code |
| **Vocabulaire français** | États : Officielle / Observation / Construction. Pas d'anglicismes dans l'app |
| **Leçons L1-L11** | À relire avant toute intervention (archivées au journal) |
| **Telegram = imminence seule** | Pas de clôture/fill/BE/TP. Le lot s'affiche avec le montant risqué |
| **L'IA n'exécute jamais** | « L'IA lit, juge, propose, explique — les moteurs décident, toi seul règles » (constitution gravée 24/08) |
| **Mesure avant décision** | Toute modification de moteur/réglage s'appuie sur des chiffres (rejeu, comparatif, production ≥ 30 trades) |
| **Zéro dette en sortie de chantier** | Tout chantier emporte ses échafaudages : pas de code mort, pas de fichier orphelin, pas de dette laissée derrière |
