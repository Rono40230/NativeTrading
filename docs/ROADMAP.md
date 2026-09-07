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

### 1. Corrections immédiates (bugs découverts par l'audit) — ✅ FAIT le 04/09

- [x] **Prompts vision — analyse graphique dégradée** : module `prompts_vision.rs`
      déclaré (il n'était même pas compilé), constantes branchées dans `defaults()`
      → system prompt restauré, PUT/DELETE `/api/prompts/vision_1tf|vision_multi_tf`
      fonctionnels. Relecture qualité à faire dans la revue prompts (§9).
- [x] **`ferme_le` de réconciliation** : `reconcilier_evenements` et
      `reconcilier_proximite` écrivaient l'horloge murale du replay (`emis_le`) au
      lieu de l'axe historique (`debut_barre`) → MFE des clôtures réconciliées
      gonflée par l'après-passe (cas BTC 01/09). Corrigé — les clôtures déjà écrites
      restent telles quelles (pas de rétro-correction), les prochaines sont exactes.
- [x] **Commentaires mensongers** : `main.rs` (endpoint `/api/pre_alertes` inexistant),
      commentaires d'endpoints fantômes `POST /api/data/collect` et
      `GET /api/worker/assets` supprimés, TODO `http.client.ts` retiré.
- [x] **MFE calculée depuis l'émission au lieu du remplissage** (04/09, trouvé par le
      propriétaire : « un SL ne peut pas suivre une excursion de +5,38R ») : pendant
      l'attente du retest le prix court sans position ouverte — BTC M1 : +5,4R affichés
      pour +1,5R réels après 169 min d'attente ; XAUUSD : +10,8R pour +0,4R. Fenêtre
      corrigée = [heure_entree, ferme_le] (la vie du trade, rien d'autre).

### 2. Grand nettoyage — vers 0 dette, 0 code mort (audit 04/09) — ✅ FAIT le 04/09

**Backend** (cargo check --workspace : 0 erreur, 0 warning — 478 tests verts) :
- [x] 13 binaires d'étude jetables supprimés (`comparatif_*` ×7, `etape4_comparatif`,
      `passe_finale`, `probe_*` ×3, `validation_spx`). **Gardés** : `news_collector`,
      `backfill_profond`, `replay_v12`, `debug_zones`.
- [x] `ollama_signal_ia_handler.rs` (module v1 non routé, jamais compilé) supprimé.
- [x] Workers v1 supprimés : `rockets_suivi_worker.rs`, `demarrer_worker_suivi`,
      `demarrer_worker_suivi_signaux`, `calculer_verdict`, `sync_feedback_historique`,
      `reconcilier_orphelins`, `reconcilier_feedback` + tous les `let _` et imports morts.
- [x] Tables DB mortes : migration `0097` (DROP `smc_analyses_llm`, `temp_metrics`,
      `positions`). `signaux_archive` conservée (archive de purge, documentée).
- [x] Dépendances retirées : `polars`, `statrs`, `aes-gcm`, `config`, `actix-rt`, `ta`
      (workspace + `indicators`), `uuid` (api).
- [x] Fichiers racine étrangers supprimés : `test_tch.rs`, `output.csv`, `patch_*.sh`.
- [x] Prompts orphelines purgées : `rockets_filtre.rs` et `rockets_contexte.rs` supprimés
      (la const vit dans `ollama/prompts.rs`), `smc_filtre.rs` réduit à sa constante
      (DORMANT documenté). Champs morts : `ResultatCategorisation.evenement_*`,
      `BarCollectors.seuil_ib/sessions_raw`, `LigneVeille.groupe`, alias `CacheAlertesPartage`.
- [x] 5 gardes `Asset::try_from` irréfutables → `Asset::from` ; imports/mut inutiles purgés
      (dont les warnings préexistants d'`asian_hl.rs`).

**Frontend** (npm run build : 0 erreur) :
- [x] Dépendances retirées : `jspdf`, `jspdf-autotable`, `topojson-client`, `world-atlas`,
      `vitest`, `@vue/test-utils`, `jsdom` + script `test` (seront réinstallés au §12).
- [x] Méthodes API mortes : `putWorkerConfig` (+ type), `traduire` (+ `TraductionReponse`).
- [x] Fonctions mortes : `jourSemaineParis`, `palierActuel` ; `classeVerdictSignal`/
      `labelVerdictSignal` dé-exportées (utilisées en interne — l'audit s'était trompé).
- [x] Types morts (`RocketSignalSave`, `StraddleSeuilsEffectifs`, `SmcBaremes`…) et
      ~20 réexports inutiles retirés ; 8 redirects router défensifs supprimés ;
      3 classes CSS mortes ; commentaire obsolète `vite.config.ts`.

**Résiduel du critère de fin** : `#![allow(dead_code)]` de `state.rs` reste jusqu'au
tranchage ML (§11) ; les types composant `ReponseIndicators` ont été conservés
(contrat API vivant — correction du diagnostic d'audit).

### 3. Étape 5 — Résiduel : validation numérique du miroir MQL5

**Mode d'emploi livré** (04/09) : `docs/VALIDATION_MQL5.md` — procédure Strategy
Tester (XAU M15, 2 mois, modélisation à noter), extraction de la référence Rust
(`jq` sur `/api/smc/rejeu` ou binaire `replay_v12`), tolérances par métrique et
méthode de traçage des divergences (arbitrage intrabar ≠ bug de miroir).

- [ ] **EA dans le Strategy Tester** (action propriétaire) : backtest `smc_ea_v12.mq5`
      selon le guide, comparaison aux chiffres du replay Rust sur les mêmes bornes
      (nombre de signaux, verdicts, R cumulé — écart attendu ≈ 0)
- [ ] **Écart ≠ 0** : le tracer règle par règle jusqu'à la divergence (le miroir est la base
      de l'automatisation future des ordres — il doit être exact)
- [ ] **Pine dans TV** (action propriétaire) : coller le Pine de `docs/reference/`
      dans TradingView sous « Scalp à Nono »
- [ ] **Unité commune points** : le rapport du Strategy Tester s'exprime en points —
      l'unité de validation sera le point (cf. §13 trades individuels)

### 4. Gate 3 — Straddle en conditions réelles

- [x] **Journalisation vérifiée** (04/09) : 19 passes remplies complètes (verdict +
      R net + prix de sortie + Telegram), 0 active orpheline.
- [x] **Bilan gate 3 chiffré** (04/09) : 11 tp2 / 7 sl / 1 be — **R net +3,93R**,
      58 % de passes gagnantes ; les 7 SL à −1,50R (jambe −1R + tampon, assumé) ;
      trailing opérationnel (29 jambes ont armé TP2, verdicts tp2 verrouillant
      jusqu'à +2,5R net). Fenêtre 27/08 → 04/09, annonces US + ouvertures DAX.
- [ ] Décision propriétaire : passage Officielle ou ajustements
- [ ] Si Officielle : activer le son Telegram (template prêt, dormant)
- [ ] Rappel money management (décision 04/09) : une passe peut coûter jusqu'à −1,5R
      nominal (jambe −1R + tampon/time-stop de la survivante) — assumé, lot inchangé

### 5. SMC v12 — Surveillance production

- [x] **Premier point de surveillance prod ↔ re-jeu** (04/09, 71 clôtures prod vs
      135 re-jeu sur 02-04/09) : comptage aligné sur les remplis ; constat clé —
      divergences de VERDICT sur M1 (XAGUSD M1 : prod +4,2R vs re-jeu −2,4R ;
      XAUUSD M1 : prod ~0 vs re-jeu +3,3R) : la prod évalue la gestion au TICK,
      le re-jeu rejoue des BARRES M1 (précédence conservatrice SL-d'abord dans
      la minute) — limite structurelle connue du replay barre-vs-tick, à
      documenter dans les métriques (pas un bug). SP500 M1 : −1R des deux côtés ✓.
      Prochain point à ~2 semaines (règle 30 trades par couple).
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

### 7. Décisions propriétaires — ✅ VIDÉE le 05/09 (toutes tranchées)

- [x] **Jauges de sentiment retirées de l'UI** (05/09) : jauge Fear & Greed principale
      + mini-jauges crypto/forex/métaux/indices supprimées (aucun intérêt propriétaire).
      Chaîne front retirée proprement (store, service, types `SentimentComposite`/
      `FearGreedData`). **Backend conservé** : le composite alimente le filtre de
      sentiment des signaux SMC (alignement direction × sentiment, marquage
      « Extreme » — `sentiment_filter`) ; `/api/sentiment/composite` reste en
      inspection. Les listes de cours USA/Europe/MP/Cryptos + VIX restent affichées.
- [x] **Rockets — sentiment sans veto macro (tranché 05/09)** : le point sentiment n'exige plus
      `BTC prix > MM50 > MM200` (golden cross retardé qui privait TOUT l'univers crypto du point
      en début de reprise — BTC +27 %/3 s. mais MM50 encore 0,9 % sous MM200 = 0/24 candidats).
      Le sentiment = force relative pure (battre la référence sur 4 semaines). Le régime
      individuel reste exigé par le critère « tendance » ; le risque pump reste couvert par
      VCP/liquidité/stop −1R/trailing. Test live : 2 rockets ce soir-là (BNB, CAKE).
- [x] **Univers actions par narratif + liquidité (tranché 05/09)** : le quota Tiingo
      gratuit réel est **500 symboles uniques/mois** (mesuré — septembre épuisé, reprise
      le 1ᵉʳ octobre). Décision : périmètre actif plafonné à **450** par liquidité
      (dollar-volume ≥ 2 M$/j sur 63 séances, prix ≥ 5 $, ≥ 40 séances — réglables en
      table `configuration` : `univers_taille_max`, `univers_dv_min`, `univers_prix_min`,
      `univers_seances_min`) ; file de backfill = prioritaires → pionniers **narratifs**
      (table `narratifs`, seed T1 électricité-IA/infra-IA/défense/space + T2 cycles +
      T3 hypes — maintenue par le propriétaire, l'IA propose) → reste alphabétique ;
      réponse Tiingo vide → état `sans_donnees` (jamais retenté — fin de la file
      empoisonnée par les delistings). Recalcul quotidien (boot + 24 h). Compteur UI =
      couverts/actifs (449/498 à froid, tend vers 450/450 en croisière).
**Tranchées et exécutées le 05/09 (1a-2a-3a-4a-5a-6c-7a)** :

- [x] **WR SMC sans expirés (1a)** : les Expire ne comptent plus au dénominateur du WR du
      re-jeu — même logique que les camemberts (un ordre jamais rempli n'est pas un trade
      pris). Ils restent dans les clôtures (capital inchangé, R pondéré = 0).
- [x] **Routes diagnostics gardées en curl (2a)** : `/api/smc/rejeu`, `/api/straddle/rejeu`,
      `/api/runtime/*` — outils d'exploitation prouvés (audits, surveillance §5), zéro
      maintenance, volontairement non câblées.
- [x] **Endpoints rockets actions (3a)** : `univers/charger`, `univers`, `contexte`, `scan`,
      `prescreen`, `news/collecter` supprimés (routes + handlers + `rockets_actions.rs`
      entier + fns db mortes + DROP table `prescreen_actions` — migration 0101, journal
      write-only). Conservés : `backfill` POST câblé en bouton **« ⚡ Rattraper maintenant »**
      (page Données — un lot immédiat après l'ajout de pionniers narratifs) et
      `backfill/etat` (le compteur). NB : le réimport NASDAQ Trader n'a plus d'endpoint —
      l'univers vit en base ; à recâbler depuis git si un jour nécessaire.
- [x] **`PATCH /api/assets/{id}/ml` supprimé (4a)** : route + handler + `db::assets::
      set_ml_actif`. La colonne `ml_actif` reste (lecture du tranchage §11).
- [x] **`GET /api/presse/briefs/{id}` supprimé (5a)** : servi jamais appelé.
- [x] **Ancien système `rockets_signaux` : NE RIEN TOUCHER (6c)** — purge et recâblage
      du bouton 📊 Analyse (branché dessus, table 0 ligne) au chantier §10.
- [x] **Prompts dormants purgés (7a)** : `smc_filtre`, `straddle_signal`, `rockets_filtre`
      supprimés du code, des defaults() et de l'UI (git garde l'historique).

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
- [x] **Journalisation du détail scoring — FAIT le 07/09** : instantané JSON
      de qualification capturé aux DEUX points (OB + BSZones) — source, score
      zone, force, qualité, sweep frais, premium/discount, score live et ses
      composantes (miroir du diagFlags MQL5). Chemin : Trade.detail →
      SignalBrut.detail → table `smc_scoring_detail` (migration 0105,
      idempotent). Lecture seule — rien ne le consomme dans la décision.
      Lecteur `details_avec_verdicts` prêt pour l'analyste (setup qualifié ×
      verdict = la matière des « pourquoi 35 SL »).

**Conviction IA à l'émission — colonne « IA » des tableaux (SMC + Straddle)** : la colonne
est réservée à la conviction (0-100 + justification en infobulle) donnée par l'analyste à
chaque signal à l'émission (`signaux.llm_conviction/llm_raison` = NULL, reliquat v1).

- [x] **Conviction IA à l'émission — FAIT le 07/09** : prompt
      `conviction_signal` (éditable, visible dans l'UI des prompts) — le
      signal complet + le détail de qualification (les deux chantiers §8 se
      nourrissent) → JSON `{conviction, raison}`. Déclencheur asynchrone à
      l'insertion officielle (tokio::spawn, jamais dans le chemin du signal —
      re-émissions non doublées), parse robuste (JSON puis repli), UPDATE
      `llm_conviction/llm_raison` — la colonne « IA » des tableaux vit.
      Observation d'abord : aucun filtrage, corrélation sur preuve après
      ≥ 30 trades notés.
- [ ] **Observation d'abord** : l'IA note, elle ne filtre rien (constitution)
- [ ] **Corrélation sur preuve** : ≥ 30 trades avec conviction → croiser conviction × verdict
      → décision propriétaire sur un éventuel filtre — pas avant

### 9. Revue complète des prompts IA — relecture finale REPORTÉE à la fin du développement

*(Décision propriétaire 05/09 : la relecture de fond reviendra en toute fin de
développement, quand les mécaniques seront figées. Une première passe a déjà corrigé
le plus urgent — § ci-dessous — pour que l'IA ne décrive pas des moteurs morts.)*

- [ ] **Relecture finale** (fin de développement) : re-passée complète des 15 prompts
      contre les mécaniques figées, purge des prompts morts (`smc_signal`, `smc_filtre`,
      `rockets_opportunites` — cf. §7), harmonisation du ton et des formats JSON.
- [x] *(05/09, première passe)* `rockets_definition` réécrite fidèle au moteur réel
      (classement /10 à 4 piliers, sentiment = force relative SANS veto macro, verdicts
      Alpha ≥ 9/Rocket ≥ 7/suivi ≥ 5, univers top 300 Binance + 450 actions narratives,
      gestion −1R puis R1 → 50 % + trailing %, MM profils) ; `rockets_analyse` (prompt
      v1 ATR/phases/RSI remplacé par la stratégie VCP réelle) ; `smc_analyse` (gestion
      complétée : TP réglables, ventes partielles 50/30/20, trailing optionnel).
      Conformes vérifiés : `analyse_rapport`, `smc_definition`, `straddle_definition`,
      `rockets_catalyseur`, `rockets_ranker`, `straddle_analyse`, `vision_1tf`,
      `vision_multi_tf`, `coach`. Aucun override actif.
- ⚠️ **Découvert — pipeline `rockets_analyse` branché sur l'ancienne table**
      (`db::rockets::signaux_pour_analyse` = `rockets_signaux`, 0 ligne — le bouton
      📊 Analyse répond toujours « pas assez de trades »). Le prompt est prêt pour le
      vrai moteur ; recâbler sur `rockets_positions`/`rockets_candidats` au §10.
- [ ] **Relecture finale** (fin de développement) : re-passée complète des 15 prompts
      contre les mécaniques figées ; purge des prompts morts (`smc_signal`, `smc_filtre`,
      `rockets_opportunites` — cf. §7) ; cohérence avec la constitution (l'IA propose,
      ne règle jamais) ; formats JSON robustes (confiance entière, replis de parse) ;
      ancrage sur les conventions actuelles ($ réels composés, R pondéré/net — jamais
      R de référence ni pips), et vérification que chaque prompt éditable reste
      synchrone avec les mécaniques du moteur qu'il décrit
- [x] Les définitions injectées reflètent l'armement actuel (04/09) : `smc_definition`
      actualisée (TP1/TP2 réglables, TP3 lointaine/R fixe + repli croisé, trailing
      optionnel, ventes partielles 50/30/20, H1 désarmé, périmètre par couple) ;
      `straddle_definition` actualisée (moteur unifié : TP3 ±3R, verdicts TP3/TS/TP2+BE/
      TP1+BE/SL/BE/Expire, comptabilité TP acquis, passe ≤ −1,5R assumée).
      `rockets_definition` vérifiée — inchangée (mécaniques courantes).
- Dormants v1 confirmés (`rockets_filtre`, `smc_filtre`, `straddle_signal`) : purge
      liée à la décision §7 (mention DORMANT).

### 10. Rockets — Extensions

**Noyau v1 purgé et recâblé le 05/09** (façade branchée sur la table vide
`rockets_signaux` — 0 ligne) :
- [x] **Historique unifié** : la page Rockets utilise la table partagée
      (`HistoryTable` + `useHistoriqueStrategie('rockets')`) — mêmes colonnes,
      tri, MFE, lot, journal que SMC/straddle. Verdict TS (sortie trailing) :
      badge 🏁 TS avec le R réel du moteur. `RocketsTableau`,
      `useRocketsHistory`, `historiqueRockets/syncRockets/annulerRocket` et
      l'annulation de trade supprimés.
- [x] **Analyse 📊 recâblée** : lecture des signaux officiels clôturés
      (`signaux` : verdicts SL/TS, R réalisés) + vivier du scanner (paliers ×
      univers, suivis/éliminés) + réglages — contexte construit côté api, la
      crate llm ne fait que l'analyse. Modale Performance réécrite (KPIs v2,
      par univers, verdicts ; heatmap probabilités conservée). Garde : 5 trades
      clôturés minimum (la verticale est jeune — c'est la vérité).
- [x] Purge backend v1 : `rockets_suivi*` (worker + route sync),
      `rockets_listing`, `rockets_niveaux` (strategies), `rockets_prix`,
      fns v1 de `db::rockets` (le module ne garde que la persistance des
      analyses), routes historique/actifs/signal DELETE.
- ⚠️ Table physique `rockets_signaux` NON droppée : les modules ML
      (feedback/features/blacklist…) la lisent encore — DROP différé au
      tranchage §11 (qui décidera aussi du sort de ces modules).

- [x] **Poste d'observation des positions ouvertes (06/09, miroir du Journal de
      Trading)** : la section « en cours » devient deux sections — À risque /
      Neutralisées — avec les colonnes de pilotage : Risque %, Invalidation,
      Entrée, Cours live (Binance cryptos / D1 Tiingo actions), Tendance, Qté
      (lot officiel enregistré à l'ouverture — migration 0102), Montant, P/L
      latent, R latent, R1, Vente R1 ; neutralisées : R1 encaissé, Trailing,
      Qté restante, P/L complet, Évolution R. **Lecture seule** — le moteur
      décide ; lignes qui pulsent verte à R1, rouge à l'invalidation/trailing.
      Endpoint GET /api/rockets/positions enrichi.
- [x] **Recadrage moteur : gestion en continu (décision propriétaire 06/09)** — « ne pas
      attendre la clôture D1 : dès que R1 est atteint = neutralisation et
      déclenchement du TS ». Boucle à 30 s sur la bougie D1 EN COURS (live).
      Aucune règle de `pas_gestion` changée (précédence stop-avant-R1
      conservée). Premier cycle réel : RAY neutralisée puis sortie TS +1,11 R
      en 30 secondes.
- [x] **Historique des trades dans la logique rocket (06/09)** :
      `RocketsHistoriqueTable` — Classement /10 (joint au signal), Ouvert
      le/Durée/Fermé le, Entrée, Invalidation, Qté, Montant, R1 encaissé,
      Trailing final, Sommet (migration 0103, suivi à chaque cycle), Sortie,
      Verdict TS/SL, R réalisé, P/L $. Synthèse : N · WR · ΣR · Σ$. Endpoint
      GET /api/rockets/historique.
- [x] **Harmonisation du layout de la page Rockets (06/09)** : colonne Setups
      sur TOUTE la hauteur à gauche ; à droite, trois blocs au design identique
      (carte + barre de couleur latérale : rouge = à risque, ambre =
      neutralisées, violet = historique), même largeur, empilés — l'historique
      occupe le reste de la hauteur. Composable `usePositionsRockets` partagé,
      sections `PositionsARisqueTable`/`PositionsNeutraliseesTable` ; la page
      quitte `StrategyShell` (SMC/straddle inchangées).
- [x] **Actions US en gestion + cours live Yahoo (décision propriétaire 06/09 —
      Finnhub écarté après soucis, Yahoo approuvé par le Journal de Trading)** :
      module `yahoo_quotes` (pattern éprouvé du journal : UA navigateur +
      cookie fc.yahoo.com + crumb caché 30 min + v7/quote) ; le scanner actions
      OUVRE les cassures (même chemin que la crypto : news ≥ 7 + ranker + lot
      officiel — fn `ouvrir_position` extraite et partagée) ; la gestion 30 s
      évalue les positions actions sur la séance du jour Yahoo (haut/bas/dernier =
      bougie en cours) ; l'endpoint positions sert le live Yahoo (tendance via
      veille dérivée). Hors session US : dernière valeur connue.
- [x] **Carte rockets connectée aux trades réels (06/09)** : le canal était
      débranché à la SOURCE — `inserer_signal_officiel` n'écrivait jamais
      `heure_entree` (mécanique d'ordre en attente SMC, sans objet en rockets
      où la position s'ouvre à l'émission) → les trades étaient classés « non
      remplis » et exclus de la performance/capital (RAY +1,11R invisible,
      courbe vide). Fix : heure_entree écrite à l'émission pour rockets +
      reprise rétroactive de RAY/BNB. Vérifié : capital 10 000 → 10 111,34 $
      (point RAY), perf total=1 gagnant=1 ΣR=+1,11, non_remplis=0. Sur la
      carte : camembert **Verdicts** (TS/SL) à la place du TF muet (D1
      unique), **Classement univers** (Crypto/Actions) à la place du
      classement TF ; badge **« N en cours 🚀 »** avec P/L latent des
      positions ouvertes en infobulle (composable partagé du poste
      d'observation).
- [x] **Suppression de l'analyse graphique et du Coach IA (décision propriétaire
      06/09 — « je ne m'en servirai jamais »)** : page Fonctionnalités IA réduite
      aux prompts seuls ; tuile dashboard IA = modèle + raccourci Prompts ;
      purge complète — front (vues, composants, composables, types
      ReponseChatIA/ReponseChartIA/ImageAvecTF, services chat/diagram/chart),
      routes /api/ia/chat|diagram|chart, handlers coach/chart, couches llm
      (anthropic.rs, vision, diagram_templates, prompts_vision, contexte,
      prompts coach — le client Ollama générique restitué en client.rs),
      entrées UI des prompts. L'analyse SMC texte (/api/ia/analyse, bouton
      🔍 des graphiques) est CONSERVÉE (vivante).
- [x] **Métriques ML et Dashboard LLM en accès direct (06/09)** : les deux
      ex-onglets de la page prompts deviennent des vues routées (`/ia/ml`,
      `/ia/llm`) ouvertes par les boutons de la tuile Fonctionnalité IA
      (✏️ Prompts · 📉 Métriques ML · 🤖 Dashboard LLM). La page prompts ne
      garde que les prompts.
- [x] **Compteurs de vie dans le bloc Data & IA Engine (06/09)** : MT5 `X/Y
      frais` (symboles avec bougie < 120 s), Bybit `+N bougies/j` (flux du
      jour), Presse `N articles` (total exposé par GET /api/presse/articles),
      LLM `N appels` (compteur du jour incrémenté aux 5 sites de POST Ollama,
      exposé par GET /api/ia/status — reset UTC). API Serveur laissé tel quel.
- [x] **Bloc 📐 Surveillance Assets supprimé (06/09 — « je ne le regarde
      jamais »)** : composant + sa plomberie privée dans DashboardHome
      (chargerPrixActifs : 5 fetchs de bougies par asset toutes les 60 s,
      assetsAvecPrix, assetsDisplay). Le prix temps réel passe uniquement par
      le store WebSocket (btcPrix du bloc Engine inclus). SentimentMarche et
      le calendrier (colonne droite) inchangés.
- [ ] **Véto unlocks** : source libre (calendrier public de déverrouillages de tokens)
      → intégrer au scanner (éliminatoire si unlock majeur < 30 jours)
- [ ] **ETF via Tiingo** : lever l'exclusion ETF + profils 2/3/4 % dédiés (répertoire
      NASDAQ Trader) — après validation de l'Observation actions
- [ ] **Analyse par pilier** : l'analyste relie les critères du /10 aux verdicts →
      propositions de recalibrage chiffrées

### 11. Boucle ML v2 — réalimentée par les vraies sources (réveillée le 06/09)

**Décision propriétaire 06/09 : RÉVEILLER** — le rôle de la boucle n'est pas de prédire
(interdit par la constitution) mais de **produire des observations statistiques pour
l'IA et le propriétaire** : importance des features (« qu'est-ce que les trades gagnants
partagent ? ») → suggestions → lues par le Dashboard LLM et l'analyste. L'état factuel
au 06/09 : 37 entraînements jusqu'au 15/08 (dernière accuracy 0,513 — pile ou face),
puis déconnexion — les collecteurs buvaient le v1 mort, les vraies sources (re-jeu SMC
166 clôtures, passes straddle, positions rockets) ne les alimentaient plus. La boucle
ne manquait pas d'intérêt : elle était **débranchée**. Première mission naturelle :
l'analyse des 35 SL SMC (leçon du re-jeu du 06/09 — le levier est dans la sélection).

- [x] **1. Rebrancher les collecteurs — FAIT le 06/09** : collecte continue
      branchée à `fermer_signal_par_cle` (LE point de passage des clôtures SMC,
      straddle ET rockets) ; rattrapage idempotent au boot (migration 0104 :
      `signal_id` + index unique) — vérifié en production : **107 samples**
      (SMC 87 dont 30 Expire, straddle 19, rockets 1). Convention : la base
      vécue = vérité terrain (le re-jeu reste un outil d'étude de réglages) ;
      les expirés inclus (classe prédictible). NB : les features détaillées du
      setup à l'émission (59 features) attendent la journalisation du scoring
      (§8) — les samples portent aujourd'hui identité + niveaux + verdict + R.
- [x] **2. Réentraîner — FAIT et VÉRIFIÉ le 06/09** : backfill des features
      (106 snapshots reconstitués depuis les bougies historiques — 52 OHLCV,
      7 contextuelles à 0 en attente de la journalisation §8), labels branchés
      sur ml_training_samples (binaire R>0, expirés exclus), `calculer_importances`
      publicisée et branchée (fini les défauts à 0.0). **Premier entraînement
      réel : XGB SMC 57 samples, OOS 66,7 %, sauvegardé** — importances par
      permutation : vol_5 (9,3 %), range_rel, open_rel, momentum_20. Les
      cartes Métriques ML et le Dashboard LLM ont de la matière. Straddle (19)
      et rockets (1) < 50 — silencieux jusqu'à accumulation (prévu).
- [x] **3. Nourrir l'IA — FAIT le 06/09** : le contexte du prompt `analyse_rapport`
      embarque le TOP 8 des features par permutation (« ML — features qui distinguent
      les gagnants : vol_5 (9,3 %), range_rel… ») — l'analyste lit les corrélations,
      pas seulement le passé. Le monitoring ML (cartes Métriques ML / Dashboard LLM)
      est rebranché sur ml_training_samples (WR 63 % sur 57 clôtures, par verdict,
      dérive) et sert les VRAIES importances + le dernier entraînement
      (`features_importances`, `dernier_entrainement`).
- [x] **4. Alléger le build — FAIT le 06/09** : audit — le GPU n'accélérait que
      le LSTM (repli CPU déjà prévu partout, panic OOM → fallback), et c'est
      xgboost qui fait le vrai travail v2 (importances, OOS). Feature `cuda`
      retirée de la dépendance ml → **libtorch/CUDA hors du build**, variables
      LIBTORCH/LIBCLANG/BINDGEN retirées du run.sh. Le workspace compile et
      461 tests passent SANS libtorch. Restent : libs hôte (WebKit/GTK) et le
      workaround GCC 15 (xgboost_lib-sys).
- [x] **5. Suggestions — rebranchées le 06/09** : le suggester
      (`params_suggester` → GET /api/ml/suggestions, lu par le Dashboard LLM)
      buvait les tables feedback mortes — ses stats globales SMC/rockets/
      straddle sont maintenant sur ml_training_samples. Garde-fous conservés
      (≥ 50 samples et seuils de confiance ; rockets/straddle silencieux
      jusqu'à accumulation). Consultatives : jamais appliquées automatiquement.

**§11 TERMINÉE le 06/09** — boucle v2 complète : collecte continue + 107
samples, XGB SMC OOS 66,7 %, importances réelles dans le prompt
`analyse_rapport` et le monitoring, suggester réalimenté, build sans
libtorch. La suite de l'enrichissement passe par §8 (journalisation du
scoring à l'émission — elle remplira les 7 features SMC aujourd'hui à 0).

### 12. Robustesse

- [x] **Tests frontend** (04/09) : vitest@2 (node, sans DOM — jsdom/test-utils inutiles) +
      21 tests verts sur la logique pure : répartition/classement/lignesClassement
      (plus grand reste, ligne « autres »), palierMax (verdicts, R réels des TP,
      pénalité jambe morte straddle), formateurs R/$ (anti « −0.0 », séparateur fr).
      `npm test` → vitest run.
- [x] **Sauvegarde automatique de la base** (04/09) : `run.sh` copie la base à chaque
      démarrage (sqlite3 .backup si dispo — copie cohérente base ouverte —, sinon cp),
      horodatée dans `data/backups/`, rétention 30 sauvegardes.
- [x] **Zones SMC sur l'historique Axi profond — vérifié PROPRE le 07/09** :
      2 ans de M1 audités — zéro doublon (aucun (asset, tf, ts) répété), et
      les 13 seuls gaps > 1 h hors weekend sont TOUS des fermetures réelles :
      Noël et Jour de l'An (24 h chacun, 2 occurrences), Thanksgiving et
      Juneteenth (107 min), et la pause de maintenance quotidienne du flux
      (62 min, 20:58→22:00). Aucun artefact de changement de source — les
      zones SMC construites dessus reposent sur des données saines.
- [ ] ETH : réactiver et re-backfiller si souhaité (décision propriétaire —
      hors périmètre autonome)
- [x] **`worker_historique_mois` 6→24 — FAIT le 07/09** : valeur en base ET
      défaut du code alignés sur 24 (harmonisés avec la rétention et la
      profondeur MT5 ; les 6 assets principaux couvrent déjà 24 mois de M1).
      (réglage utilisateur via la config)

### 13. Unités & métriques — résiduels

**Livré le 04/09** : badge R pondéré (référence en infobulle), camemberts/histogramme en
dollars réels, badge straddle/rockets = R réalisé, MFE/Lot/tri des historiques.

**Livré le 05/09** :
- [x] **Historique — colonnes Ouvert le / Durée** : « Ouvert le » affiche le REMPLISSAGE
      (l'ouverture réelle de la position) au lieu de l'émission — l'attente du retest
      n'est pas de la vie en position (cas BTC 04/09 : émis 21:04, rempli 23:52:30,
      mort 31 s plus tard) ; l'émission part en infobulle. Nouvelle colonne « Durée »
      (vie de la position : remplissage → fermeture ; straddle : heure E → clôture de
      la passe), triable, format 31 s / 2 mn 30 s / 2 h 13 mn / 1 j 3 h. Tri par défaut
      et égalités alignés sur le remplissage (`formatDuree` testée, SMC+straddle via
      table partagée).
- [x] **Lot toujours en 2 décimales** (colonne Lot et partout), colonne Sortie réduite
      au prix (±R redondant avec Palier max).
- [x] **Courbe du capital bicolore** (cartes stratégies dashboard) : ligne pointillée
      au capital de départ, courbe verte au-dessus / rouge en dessous (découpe SVG par
      clip calé sur la ligne — géométrie partagée extraite dans `useCourbeCapital.ts`
      + `CourbeCapital.vue`, parent repassé sous la limite 600 lignes du pré-audit).
- [x] Libellés de la définition rockets alignés sur le code (surperformance = point
      Sentiment, pas Tendance — erreur préexistante).
- [x] **Harmonisation straddle carte ↔ re-jeu (05/09, dette signalée par le propriétaire)** :
      la carte du dashboard vivait en base vécue (R + capital + camemberts) pendant que
      le rapport d'activité servait le re-jeu — deux conventions pour la même stratégie.
      `performance_strategie` et `capital_strategie` servent désormais le re-jeu straddle
      (comme SMC : R net en badge, référence en infobulle), repli base marqué
      « ⏳ recalcul » pendant le calcul au boot, snapshots §14 protégés du transitoire.
      Rockets reste en base vécue (pas de re-jeu — par design).

- [x] **Trades individuels → points MT5** (04/09) : la convention par asset existait
      déjà (`asset_params.pip_to_points`, peuplée : 100 métaux/BTC, 10 indices/forex —
      déjà utilisée par le panneau risque et les alertes). Aligné : historique — la
      mention du palier affiche désormais des **points MT5** (unité broker) au lieu
      des pips ; tooltip des trades en cours = « pts d'abord, pips ensuite ». Les
      messages Telegram restent aux niveaux en prix + lot (règle transverse) —
      lecture MT5 des digits réels non nécessaire tant que les conventions
      suivent le broker (à revoir seulement si un asset change de digits).
- [ ] **Vigilance sizing** : `taille_pip`/`valeur_pip` alimentent le calcul des lots —
      tout chantier d'unités ne touche QUE l'affichage

### 14. Rapport d'activité — approfondissements

**Livré le 04/09** (phases 1-3) : bloc dashboard, `/analyses` à onglets, IA à la demande
(cache du jour, règle des 30 trades ancrée), croisé asset × TF, prompt `analyse_rapport`
éditable (Configuration & Métriques IA).

- [x] **Historisation des rapports** (04/09) : table `analyses_snapshots`
      (migration 0098) — un snapshot par stratégie/jour écrit paresseusement au
      premier calcul (`INSERT OR REPLACE`, avis IA préservé), l'avis IA du jour
      rattaché au snapshot (survit aux redémarrages — le cache mémoire, lui, non).
      Endpoint `GET /api/analyses/{id}/historique` + carte « 📈 Évolution jour
      après jour » dans l'onglet (courbe du capital, 14 derniers jours :
      capital · hier $ · ΣR · confiance de l'avis en infobulle)
- [x] **Heatmap heure×jour** (05/09) : contribution $ par créneau heure×jour dans
      l'onglet Stratégie du rapport (cases lun→dim × heures, intensité = dollars,
      infobulle trades + $) — `heatmap_hj` côté backend, réutilise le parcours des
      clôtures.

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
- [x] **Journal de bord du propriétaire** (04/09) : table `journal_bord` (migration 0099 —
      fil append-only horodaté par trade, suppression d'entrée possible), endpoints
      `GET/POST /api/journal/{signal_id}` + `DELETE /api/journal/entree/{id}` +
      `GET /api/journal/comptes`. UI : modale 📝 dans l'historique (colonne dédiée,
      badge du nombre de notes, entrée par ⏎) — SMC et straddle. Matière première
      posée pour l'analyse IA (§8 : l'analyste lira ces notes en contexte).
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
