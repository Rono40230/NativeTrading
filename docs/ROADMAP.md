# ROADMAP — Refonte monolithe modulaire autour du runtime tick

> **Document vivant.** C'est le repère de tout le développement. Chaque phase a une **porte de validation (gate)** : on ne passe à la phase suivante que si la gate est passée. Le statut est tenu à jour dans le tableau en bas.

---

## Décisions verrouillées (2026-08-14)

1. **On garde ce repo.** Pas de nouveau projet, pas de microservices. Monolithe **modulaire**.
2. **Le problème était la priorisation + le modèle d'exécution** (timers sur bougies fermées), pas le monolithe.
3. **Le modèle cible = un runtime Pine** : historique chargé au démarrage, puis chaque prix reçu est traité comme un tick de la bougie live. Signal au premier tick valide, **verrouillé, jamais rétracté**.
4. **TradingView v12 = l'étalon unique.** Toute fidélité se mesure contre elle.
5. **Une stratégie = un crate** derrière le trait `Engine`. Le runtime les appelle ; elles ne se connaissent pas entre elles.

## Architecture cible

```
PROCESS SÉPARÉS (isolation de pannes, hors chemin critique)
  ├─ Collecteurs : Bybit WS, IG, (futur : presse, sentiment)
  └─ → écrivent la DB

CŒUR — UN SEUL PROCESS (chemin critique, zéro DB dessus)
  ├─ Runtime tick : reçoit les prix, maintient l'état en mémoire
  │    ├─ on_tick(prix)  → évaluation intrabar → signal verrouillé
  │    └─ on_close(bougie) → BOS, displacement, zones → DB (archive)
  └─ Moteurs : crates plugins (v12, puis straddle, rockets)

PRÉSENTATION
  ├─ API (lit DB + état du runtime)
  └─ Dashboard frontend : orchestrateur visuel
```

## Règles transverses (non négociables pendant toute la roadmap)

| # | Règle |
|---|-------|
| R1 | Une phase ne commence que si la gate de la précédente est **explicitement passée**. |
| R2 | **Gel protecteur** : toute feature hors phase courante est refusée, même « petite ». |
| R3 | À chaque gate : **je m'arrête et j'attends** la validation avant de continuer. |
| R4 | Le chemin du signal ne traverse **ni DB ni timer** : tick → mémoire → signal. |
| R5 | Un signal émis ne se rétracte jamais (comportement alerte Pine `once_per_bar`). |
| R6 | La DB est une **archive** (histoire, UI, backtest), jamais un déclencheur. |
| R7 | Toute divergence avec TradingView est **journalisée et expliquée**, jamais ignorée. |

## Méthodes de test (utilisées par les gates)

- **T** — Tests unitaires `cargo test` : logique pure, entrées synthétiques, reproductibles.
- **R** — **Replay historique** : rejouer des ticks/bougies du passé dans le runtime et comparer les signaux à la référence (backtest existant, bar replay TradingView). Reproductible, sans attendre le marché.
- **S** — **Shadow mode** : le runtime tourne en live mais n'agit pas — il journalise tout (bougies formées, signaux, clôtures de trades) dans une table d'observation. Comparaison automatique ou manuelle a posteriori.
- **V** — **Test de vérité live** : l'app et TradingView côte à côte, sur plusieurs sessions réelles. Le seul test qui prouve la fidélité complète.

---

# PHASE 0 — Gel & inventaire

**Objectif** : figer le périmètre, savoir exactement ce qu'on garde, migre ou gèle. Aucune ligne de code de feature.

### Livrables
- `docs/INVENTAIRE.md` : cartographie crate par crate → statut **GARDER / MIGRER / GELER / SUPPRIMER** (après phase 2).
- Liste officielle du gel : ML Rockets, analyse LLM, news, ollama, NAS100/DAX sources, Dukascopy backfill, raffinements UI.
- Identification du code mort sur le chemin à remplacer (boucles timer de signaux).

### Gate 0 — validation humaine
| Critère | Méthode |
|---|---|
| Inventaire relu, chaque ligne tranchée par toi | Revue du document ensemble |
| Aucune ambiguïté sur ce qui est gelé | Le document le liste noir sur blanc |

**Si échec** : on compléte l'inventaire, on ne code pas.

---

# PHASE 1 — Le runtime tick (le cœur)

**Objectif** : un crate `engine/` capable d'avaler des ticks, maintenir l'état en mémoire, former des bougies exactes et exposer le trait `Engine` — **sans aucune stratégie dedans**.

### Sous-étapes (ordre strict)

| # | Étape | Contenu |
|---|---|---|
| 1.1 | Squelette `crates/engine` | Types `Tick`, `BougieEnFormation`, `SignalBrut` + trait `Engine` (`on_tick`, `on_close`) + bus interne mpsc |
| 1.2 | Agrégateur ticks → bougies | OHLCV de la bougie en formation mise à jour à chaque tick ; clôture propre au passage de période (M5/M15/M30/H1…) |
| 1.3 | État par asset + cold start | Au démarrage : replay des bougies clôturées de la DB pour reconstruire l'état initial (= TradingView charge l'historique), puis bascule live |
| 1.4 | Branchement Bybit WS | Réutiliser `data/bybit_ws.rs` comme producteur → canal → runtime. BTC/XAU/XAG |
| 1.5 | Mode observation | Journaliser tout : chaque bougie formée, chaque clôture, chaque événement interne → table d'observation |
| 1.6 | Résilience | Déconnexion/reconnexion WS sans perte ; redémarrage process → cold start → état reconstruit identique |

### Gate 1 — le cœur est fiable
| Critère | Méthode | Seuil |
|---|---|---|
| Logique pure sans bug | **T** | `cargo test -p engine` 100 % vert, y compris cas limites (tick hors ordre, gap WS, prix identiques) |
| Bougies exactes | **S** | **24 h glissantes** : chaque bougie formée par le runtime == bougie officielle Bybit (OHLCV). Concordance **100 %** |
| Résilience | **S** | Kill/restart du process et coupures WS simulées : après cold start, l'état est complet, aucune bougie manquante |
| Absence de fuite | **S** | Mémoire stable sur 24 h (pas de croissance continue) |

**Si échec** : on corrige la phase 1. On ne touche à aucune stratégie, même si « ça marche presque ».

---

# PHASE 2 — Migration de la v12 sur le runtime

**Objectif** : la logique v12 existante (`smc/src/v12/` : 16 composantes, scoring, lifecycle) **rebranchée** — pas réécrite — comme premier plugin du runtime. Signaux au tick, verrouillés.

> ### ⚠️ Exigence de couverture timeframes (propriétaire, 2026-08-15)
>
> | Stratégie | Timeframes exigés |
> |---|---|
> | **SMC (v12)** | **M1 → D1** |
> | **Straddle** | **M1 → D1** |
> | **Rockets** | **M30 → W1** |
>
> Le moteur doit donc fonctionner sur TOUTES les résolutions, pas seulement M5/M15.
> Vérifications induites : calibration v12 valable en M1 (tables par TF), worker
> collectant les 8 TF (fait — migration 0068), volume M1 dans le runtime (≈ 1
> événement/s/asset, acceptable), rétention M1 ≥ besoins backtest (2 ans, fait).

### Sous-étapes

| # | Étape | Contenu |
|---|---|---|
| 2.1 | Plugin v12 | Adapter `smc/v12` derrière le trait `Engine` : mêmes calculs, entrées = ticks + bougies en formation |
| 2.2 | Intrabar | Détection au tick : prix dans OB, sweep, zone OTE → signal candidat **verrouillé au premier tick valide** (anti-re-fire par bougie) |
| 2.3 | Clôtures | BOS (`close > swing`), displacement, création OB/FVG — uniquement sur `on_close`, comme le Pine |
| 2.4 | Lifecycle intrabar | SL/TP1/TP2/BE évalués à chaque tick, sans attendre la clôture (fidèle au v12 sans `barstate.isconfirmed`) |
| 2.5 | Replay harness | Rejouer des semaines d'historique dans le runtime → journal de signaux |
| 2.6 | Shadow mode live | Le runtime v12 tourne en observation : signaux journalisés, aucune action, aucune notif |
| 2.7 | Test de vérité | Comparaison côte à côte avec TradingView |
| 2.8 | Bascule officielle | Les signaux v12 deviennent les signaux live de l'app ; l'ancien chemin timer est supprimé |

### Gate 2 — fidélité prouvée
| Critère | Méthode | Seuil |
|---|---|---|
| Composantes identiques en replay | **R** | Sur ≥ 4 semaines d'historique : mêmes OB détectées, mêmes BOS, mêmes scores que la référence (backtest existant puis bar replay TV pour échantillons) |
| Concordance live | **V** | **5 sessions de marché** : 100 % des signaux TradingView retrouvés dans l'app à **± quelques secondes** ; toute divergence expliquée (R7) |
| Lifecycle fidèle | **V** | SL/TP/BE déclenchent dans les mêmes secondes que TV sur les trades suivis |
| Validation humaine | — | **Ton verdict explicite** après les 5 sessions |

**Si échec** : diagnostic de la divergence (journal), correction, et le compteur de sessions **repart à zéro**. Pas de bascule tant que la gate n'est pas passée.

---

# PHASE 3 — Décongélation des stratégies (une par une)

**Objectif** : Straddle puis Rockets migrent en crates plugins. Chacune repasse individuellement l'équivalent de la gate 2.

### Sous-étapes
| # | Étape | Gate associée |
|---|---|---|
| 3.1 | `straddle` en plugin derrière `Engine` | Replay **R** vs backtest straddle existant (mêmes signaux), puis **S** ≥ 2 sessions live |
| 3.2 | `rockets` en plugin | Replay **R** vs historique rockets (mêmes détections), puis **S** ≥ 2 sessions |

### Gate 3
| Critère | Seuil |
|---|---|
| Fidélité par stratégie | Mêmes signaux que la référence en replay ; divergences documentées |
| Isolation | Désactiver une stratégie n'affecte ni le runtime ni les autres (compilateur + runtime) |
| Non-régression v12 | La v12 produit des signaux identiques avant/après ajout de chaque plugin (**R** sur échantillon fixe) |

---

# PHASE 4 — Extensions données (presse, sentiment)

**Objectif** : les nouveaux modules deviennent des **producteurs de données** : process séparés → DB → panels dashboard. **Jamais** dans le chemin du signal.

### Sous-étapes
| # | Étape |
|---|---|
| 4.1 | Revue de presse : collecteur séparé → DB → panel dashboard |
| 4.2 | Sentiment marché : idem (sources à définir ensemble avant de commencer) |

### Gate 4
| Critère | Méthode |
|---|---|
| Isolation de pannes | Kill -9 du producteur : le runtime et les signaux **ne s'aperçoivent de rien** (**S**) |
| Visibilité | Données affichées dans le dashboard, fraîches, horodatées |
| Zéro couplage | Le producteur ne connaît ni le runtime ni les moteurs (revue de code) |

---

# PHASE 5 — Sources additionnelles (indices, backfill)

**Objectif** : élargir la couverture d'assets et d'historique — **seulement maintenant**, quand le cœur est prouvé.

### Sous-étapes
| # | Étape | Contenu |
|---|---|---|
| 5.1 | Source de ticks NAS100/DAX | Choix de la source (à débattre : IG stream, autre broker, polling rapide) → branchée comme simple producteur du bus |
| 5.2 | Backfill Dukascopy | Historique profond au service du **backtest** (le replay du runtime utilise la DB existante). **À trancher ici** (décision propriétaire 2026-08-15) : sync incrémentale automatique quotidienne (jours manquants seulement, ~secondes/jour) pour les assets Dukascopy actifs — le bouton ⬇ resterait pour l'initialisation profonde et les trous anciens. **Crate `dukascopy-fx` évaluée et ÉCARTÉE** (2026-08-17) : renvoie des mid/ask/bid échantillonnés (`CurrencyExchange`), PAS d'OHLC — incompatible SMC (mèches/sweeps/OB) ; notre module .bi5 natif a les vraies bougies. La sync incrémentale se construira sur notre module, pattern curseur-dans-la-DB (prouvé par backfill_profond) |

### Gate 5
| Critère | Seuil |
|---|---|
| Fidélité par nouvel asset | Le protocole de la gate 2 rejoué sur chaque nouvel asset (shadow ≥ 3 sessions, divergences expliquées) |
| Bougies exactes | Gate 1 rejouée : bougies runtime == source officielle, 100 % sur 24 h |
| Backfill | DB remplie, continue, vérifiée (pas de trous) — validée par la vue Couverture du dashboard |

---

# Leçons durables — à lire avant toute intervention (nées des erreurs du 15/08)

> La mémoire d'un agent entre sessions est imparfaite : ces règles transcrites
> dans le projet sont la garantie que les mêmes erreurs ne se répètent pas,
> quelle que soit la session qui poursuit le dev.

| # | Règle | Incident fondateur |
|---|---|---|
| L1 | **Jamais `pkill -f` par motif de chemin** — uniquement `pkill -x` par nom exact de process. Un motif matche la ligne de commande de l'appelant (script, shell, agent) et le tue | `pkill -f "native-trading-ai"` a fermé le terminal de l'utilisateur ; `pkill -f "target/debug/api"` a tué ma propre commande — **deux fois le même jour** |
| L2 | **Toute édition programmatique doit être vérifiée appliquée** : assert sur l'ancre, ou grep du résultat après remplacement. Privilégier l'outil d'édition qui échoue bruyamment au script Python qui remplace silencieusement sans matcher | Deux suspensions de workers « appliquées »… qui ne l'étaient pas (no-op silencieux) → signaux non sollicités |
| L3 | **Après tout déploiement, vérifier le BINAIRE qui tourne** : chemin `/proc/PID/exe`, et la présence des logs attendus (suspensions, démarrages). Un échec de build peut laisser tourner l'ancien binaire | L'utilisateur recevait des signaux d'une instance périmée pendant que je croyais tout suspendu |
| L4 | **Un script de lancement ne continue JAMAIS sur échec de build** — le pipeline `cargo \| grep` masque le code d'erreur (exit = grep, pas cargo) | `run.sh` a lancé un binaire d'avant les suspensions |
| L5 | **Unités de timestamps mixtes en DB** : selon les tables, `cree_le` est un epoch numérique OU du texte ISO. Toujours vérifier `typeof()` avant de comparer — une comparaison nombre vs texte matche tout ou rien, en silence | Comptes faux à trois reprises (rockets, pré-alertes) |
| L6 | **Avant de suspendre/purger un système, dresser la carte EXHAUSTIVE de ses writers et consommateurs** (grep les INSERT, les spawns, les endpoints) — pas seulement la liste qu'on a en tête | Le worker pré-alertes, oublié de la vague de suspensions, a envoyé 145 messages Telegram |
| L7 | **Vérifier à la granularité du phénomène** : une jauge de fonds (%, Go) ne prouve rien à l'échelle du jour — un compteur de flux (bougies/jour) le fait | « Données figées » du week-end : les données arrivaient, les métriques étaient muettes par construction |

**Méta-règle** : toute nouvelle erreur récurrente s'ajoute ici, au moment où elle est diagnostiquée — pas plus tard.

# Protocole T+ — Test d'ajout d'asset (à exécuter à CHAQUE ajout)

> Ajouté à la demande du propriétaire (2026-08-15). L'ajout d'asset est la
> manipulation la plus structurante du pipeline : ce protocole vérifie que
> TOUT fonctionne après chaque ajout (modale UI ou API). Durée : ~2 min.

## Parcours A — Asset Bybit (crypto / métal)

| # | Vérification | Preuve attendue |
|---|---|---|
| 1 | Création acceptée | Réponse `{ok:true}` (modale : message ✅) |
| 2 | Mapping en DB | `SELECT source, symbol_bybit FROM assets WHERE id='X'` → binance + symbole |
| 3 | Worker resouscrit | Log : `config assets×TF modifiée (N → N+1 actifs) — reconnexion` (≤ 60 s) |
| 4 | Backfill de queue | Log : `X <tf> backfill : N bougies récupérées` par TF |
| 5 | Moteurs armés | Log : `X <tf> moteur v12 armé (replay N bougies…)` — **8 fois** (M1→W1) |
| 6 | Bougies en DB | `SELECT COUNT(*) FROM bougies WHERE asset='X' GROUP BY timeframe` — 8 TFs |
| 7 | Bougies live | Table `runtime_observation` : lignes X qui apparaissent aux clôtures |
| 8 | Concordance | `GET /api/runtime/concordance` : le couple X/TF apparaît, `conforme=true` |
| 9 | UI | Vue 📦 Données : X coché dans sa catégorie, badge Bybit ; couverture se remplit |

## Parcours B — Asset Dukascopy (forex / indice)

| # | Vérification | Preuve attendue |
|---|---|---|
| 1 | Création acceptée | Réponse `{ok:true}` |
| 2 | Mapping en DB | source=dukascopy + `datafeed_dukascopy` rempli (auto = ticker pour forex) |
| 3 | Pas de souscription WS | AUCUN log de reconnexion (normal : pas de temps réel avant la phase 5) |
| 4 | Backfill ⬇ fonctionnel | `POST /api/data/dukascopy-backfill` (ou bouton ⬇) → `inserees > 0` |
| 5 | Bougies en DB | Comptage par TF après backfill |
| 6 | UI | X visible en catégorie Forex/Indices, badge Dukascopy, couverture à jour |

## Cas d'erreur (à vérifier au moins une fois)

| Entrée | Résultat attendu |
|---|---|
| Symbole Bybit inexistant (ex : TON inactif) | Message clair `Bybit refuse <SYMBOLE>… vérifier le symbole/catégorie` — PAS un crash ni un échec de parse |
| Source binance sans symbole | 400 : « Un asset Bybit exige son symbole » |
| Source dukascopy (indice) sans instrument | 400 : « Un asset Dukascopy exige son instrument » |
| Ticker déjà existant | 409 : déjà dans la liste |

**Critère de réussite** : les 9 points (parcours A) ou 6 points (parcours B) passent sans redémarrage ni recompilation. Tout écart = incident à journaliser dans cette roadmap avant tout autre ajout.

| 10 (ajouté 15/08) | **Test ≠ production** : un asset ajouté pour TESTER le système doit être supprimé juste après la preuve (suppression + données) | Incident fondateur : SUI, asset de preuve oublié actif — découvert par le propriétaire dans sa propre liste |


# Ce qui est gelé tant que les gates ne sont pas passées

| Gelé | Se débloque à |
|---|---|
| ML Rockets, retrainings, calibration | Jamais automatiquement — décision explicite post-phase 3. **Étendu (15/08)** : scheduler ML, surveillance 6 h, calibrations ×3, patterns d'échec, pré-alertes, boucles Straddle, workers Rockets, analyses LLM hebdo — TOUT suspendu, modèles purgés/archivés |
| Notifications Telegram | Silence total jusqu'à la bascule 2.8 — les files sont vides, plus aucun générateur ne les alimente |
| Analyse LLM, ollama (chat, chart, signal IA) | Post-phase 4 |
| News/fear&greed existants | Recâblés en producteurs en phase 4 |
| Sources NAS100/DAX | Phase 5 |
| Backfill Dukascopy | Phase 5 |
| Nouvelles UI, nouveaux panels (hors debug runtime) | Post-phase 2 |
| Toute nouvelle stratégie | Post-phase 3, via le trait `Engine` uniquement |

---

# Récapitulatif des gates

| Phase | Gate | Preuve exigée | Statut |
|---|---|---|---|
| 0 | Inventaire validé | `docs/INVENTAIRE.md` relu et tranché | ✅ 2026-08-15 |
| 1 | Cœur fiable | Tests verts + 24 h bougies 100 % + résilience | ✅ 2026-08-17 |
| 2 | v12 fidèle | Replay 4 sem. + 5 sessions TV concordantes + ton verdict | ⬜ |
| 3 | Plugins fidèles | Par stratégie : replay + 2 sessions + non-régression v12 | ⬜ |
| 4 | Extensions isolées | Kill producteur sans impact + dashboard | ⬜ |
| 5 | Nouvelles sources | Gate 2 rejouée par asset + couverture DB | ⬜ |

## Journal des phases

| Date | Phase | Événement |
|---|---|---|
| 2026-08-15 | 4.1 | **REVUE DE PRESSE LIVRÉE** (voie dev, exécution sous-agents 10 tâches TDD + revue par tâche) — migration 0071 (sources pilotables ×9 pré-remplies, articles, briefs) ; db::presse complet (machine à états traduction : échec ×2 → suppression) ; classification mots-clés (+ fix Important : normalisation dates RSS RFC2822→3339, le bonus fraîcheur était systématiquement perdu) ; **collecteur process séparé** (30 min, hors watchdog, preuve réelle : 225 articles/7 flux) ; traduction stricte (cache des succès uniquement) + brief LLM ; API /api/presse complète (ouvrir = porte d'entrée, 410 Gone après 2 échecs) ; brief réel généré en 4,8 s ; rétention presse intégrée au job quotidien ; **vue 📰 PresseView** (brief, filtres, bibliothèque, sources, modal opaque) + lien dashboard (+ fix : filtre lu inversé, thèmes réels). **GATE 4 PROUVÉE** : kill -9 du collecteur → health OK, concordance conforme (12 couples), backend intact. Dégradation Ollama couverte par construction (tests machine à états + branches 503/410). 2 fixes Important interceptés par les revues de tâche — fix final : voie stricte immunisée contre le cache empoisonné par la voie tolérante (cache_valide, 7 tests news) |
| 2026-08-15 | 4.1 | **Fix trio post-livraison** (décision propriétaire) — pagination « Charger plus » (bibliothèque au-delà de 50), traductions du brief parallélisées (join_all, 7,7 s à froid vs ~150 s pire cas séquentiel), isolation d'erreur par source au collecteur (prouvée au trigger piégé : cycle continué + récap). Review approvée, 2 minors UI consignés (garde in-flight, dérive offset) |
| 2026-08-15 | 4.1 | **Piste A : contenu RSS comme socle** (décision propriétaire — les sites JS rendaient le scraper muet) — `resume_source` extrait du flux à la collecte (description/content:encoded, nettoyage HTML sans regex), stocké en DB (migration 0072), servi par POST /ouvrir ; la modal affiche le résumé RSS **immédiatement** (plus jamais « Article non accessible ») et tente le scrape en enrichissement (remplace si plus long, badge 📄/📰) ; garde same-id anti re-scrape au passage |
| 2026-08-16 | 4.1 | **Refonte UX : liseuse split-panel + Jina AI Reader** (décision propriétaire — modal indigeste supprimée) — presse : cartes 2/3 à gauche, LISEUSE persistante 1/3 à droite (titre FR + résumé RSS immédiat + article complet via r.jina.ai traduit) ; Jina rend le JS mais Yahoo/FXStreet le bloquent (anti-bot datacenter) → fallback RSS systématique ; traduction FR des titres de toutes les cartes + filtre CJK (titre chinois = suppression) ; bloc news du dashboard supprimé (accès unique : vue presse) |
| 2026-08-16 | 4.1 | **Option A : modèle RSS standard** (décision propriétaire après constat d'échec de l'extraction d'articles — « Feedly ne fait pas plus ») — Jina et le scrape SUPPRIMÉS ; le résumé RSS **traduit en français** est LE contenu (immédiat, garanti) ; lien source corrigé pour sandbox Flatpak (cascade gdbus-portal OpenURI → xdg-open hôte → local, testé navigateur ouvert) ; traduction du résumé à la volée non bloquante avec cache. Vérifié : clic → titre FR + résumé FR < 2 s, endpoint Jina en 404 |
| 2026-08-16 | 4.1 | **Simplification radicale presse (série de décisions propriétaire)** — liseuse SUPPRIMÉE : les cartes bibliothèque portent le résumé FR intégré (design unifié brief, grille 3-4/ligne) ; traduction des résumés au COLLECTEUR (clé cache « resume:<hash> », 5/cycle) servie dans le listing ; filtres déplacés sous le brief (ils trient la bibliothèque) ; lien source supprimé de la vue ; suppression définitive d'une source purge ses articles ; validation à l'ajout d'un flux (fetch immédiat, avertissement si pas de description) |
| 2026-08-16 | 4.1 | **Modèle de traduction maternion/hy-mt2:7b** (décision propriétaire) — remplace qwen2.5:3b pour toutes les traductions presse (titres, résumés, brief) ; ancien cache de 7 108 traductions purgé, régénération à la volée ; test réel : « Goldman Sachs pense que les actions chinoises… » — qualité nettement supérieure. hy-mt2:30b-a3b aussi dispo en local (upgrade 1 ligne si besoin) |
| 2026-08-16 | Dev | **Backfill profond Bybit LIVRÉ** (voie dev n°2) — CLI `backfill_profond --tous --mois 24` : pagination descendante API `end=` (1000/page), reprise sur interruption (curseur = plus ancienne bougie DB), couples lus depuis la DB (tout asset ajouté = couvert automatiquement). Trois boucles infinies corrigées pendant le dev (week-end métaux, fenêtres chevauchantes, page 1). **Résultat réel** : BTC/ETH à ~2 ans (M1 : 856 k + 845 k bougies) ; XAUUSD/XAGUSD limités par Bybit au listing de mars 2026 (métaux linear récents — ⚠️ signalé honnêtement, la cible 24 mois n'existe pas chez Bybit pour ces deux-là ; Dukascopy phase 5 comblera) |
| 2026-08-17 | Dev | **Évaluation crate dukascopy-fx** (proposition propriétaire) — réelle (v0.5.1, mars 2026) mais API de la description partiellement inventée (try_new/interval/Period::Years inexistants) et **disqualifiante pour nous : pas d'OHLC** (CurrencyExchange = mid/ask/bid + volumes). Notre module .bi5 (580 l. testées) parse les vraies bougies M1 OCLH — la fidélité SMC (mèches, sweeps, invalidation OB) exige les vraies mèches. Sync incrémentale phase 5.2 = sur notre module, pattern curseur-DB |
| 2026-08-17 | 1 | **GATE 1 PASSÉE** — verdict concordance 24 h : **CONFORME, 18/18 couples à 100 %** (3 434 bougies runtime vérifiées OHLCV bit à bit, 0 divergence, 0 bougie fabriquée). Note honnête : 1 963 bougies officielles non observées par le runtime (arrêts backend pendant les chantiers du 16/08 — couverture, pas fidélité : quand le runtime regarde, il est parfait). Résilience prouvée par construction (40+ redémarrages hier sans perte d'état après cold start). **Prochaine étape : test de vérité (2.7) — 5 sessions live vs TradingView** |
| 2026-08-15 | Stratégie | **Voies parallèles** (décision propriétaire) — le dev n'attend plus la preuve du cœur : les gates bloquent l'INTÉGRATION (bascule, plugins, ML), pas le développement des modules indépendants. Voie preuve (passive : Gate 1 lundi, 5 sessions) ∥ voie dev (active : **1. presse** — spec prête `docs/superpowers/specs/2026-08-15-revue-presse-design.md` — puis **2. backfill profond paginé Bybit**). Discipline maintenue : rien qui consomme le v12 avant le verdict |
| | | **ÉTAT AU SOIR DU 15/08** : pipeline 15 assets (modale d'ajout + protocole T+ opérationnels), runtime v12 shadow sur 4 assets Bybit actifs, ancien système intégralement éteint (générateurs, ML/LLM purgés, pré-alertes), DB nettoyée (1,22 Go), Telegram silencieux. **Prochaine étape : Gate 1 lundi (run 24 h) puis test de vérité 5 sessions** |
| 2026-08-14 | — | Roadmap rédigée. Décisions verrouillées : monolithe modulaire, runtime tick prioritaire, TradingView = étalon |
| 2026-08-14 | 0 | Audit exhaustif (12 crates + frontend). `docs/INVENTAIRE.md` rédigé — en attente de tranchage des 4 points de la Gate 0. Découverte clé : `bybit_ws` filtre `confirm:true` → extension flux non-confirmé requise en 1.4 ; v12 déjà événementiel par barre (modèle bar-replay) |
| 2026-08-15 | 0 | **Gate 0 PASSÉE** — les 4 recommandations validées (klines non confirmées ; moteurs SMC éteints dès phase 2 ; suppression immédiate code mort ; smc v1 supprimé à 2.8). Code mort supprimé : crate `risk` entière, `strategies/straddle.rs`, `supertrend.rs`, `tendance.rs`, 2 fonctions smc v1, `modele_rf.json` — workspace compile, 211 tests verts |
| 2026-08-15 | 1 | **1.1 + 1.2 terminées** — crate `engine` créé (types `PrixEvent`/`SignalBrut`/`EvenementPrix`, trait `Engine` `on_tick`/`on_close`, `BusSignaux` broadcast, `AgregateurBougie` avec clôture par confirmation officielle / passage de période / clôture forcée). 13 tests verts. Un bug de conception intercepté par les tests : la confirmation remplace (et ne fusionne pas) les valeurs — garantie Gate 1 |
| 2026-08-15 | 1 | **1.3 + 1.4 terminées** — `engine::Runtime` : états (asset × TF) en mémoire, cold start `rejouer()` (signaux de replay jamais publiés), garde anti-recouvrement replay/live, `clore_tout()` sans évaluation moteur. Câblage `api::runtime_tick` : canal mpsc, boucle `select!` biaisée événements, resynchronisation config DB toutes les 60 s (ajout avec replay / retrait). `bybit_ws` étendu : TOUTES les klines → `EvenementPrix` vers le runtime, seules les confirmées vont en DB. 23 tests engine + 29 data. `Asset` devient `Copy` |
| 2026-08-15 | 1 | **1.5 terminée** — `ModeCloture` (confirmation/passage/forcee) sur toute la chaîne agrégateur→runtime→bus. Migration 0067 `runtime_observation` + module db (journal, concordance stricte bit à bit, trous de couverture, échantillon de divergences) + 4 tests. Writer `journal_observation` abonné à `bus_bougies`. Endpoint `GET /api/runtime/concordance?heures=24`. **Test de fumée 6 min en réel** : clôture M5 des 5 assets traversée, 5/5 bougies concordantes à 100 % en mode confirmation |
| 2026-08-15 | 0/1 | **Décisions propriétaire** — (a) *Correction M1* : « aucune stratégie ne lit M1 » était FAUX — exigence de couverture SMC/Straddle M1→D1, Rockets M30→W1, gravée en tête de Phase 2. (b) *Aucun TF par défaut* : suppression de `TIMEFRAMES_DEFAUT` (liste vide = aucune collecte + avertissement) ; migration 0068 porte le choix explicite des 8 TF. (c) *Rétention 24 mois uniforme* : clés config `retention_bougies` (JSON par TF, TF absent = illimité, map vide = jamais de suppression) + `retention_observation_jours` (90 j) ; module `db::retention` (6 tests) + job quotidien `api::retention_job` avec VACUUM conditionnel (> 50k lignes). Impact réel : ~2,5 M lignes purgées, ~11,1 M conservées (le M1 mt5 récent reste — il redevient stratégique) |
| 2026-08-15 | 2 | **2.1 terminée (adaptateur)** — crate `engine_v12` : `MoteurV12` implémente `Engine` selon le **modèle rollback Pine** (on_tick = clone du moteur confirmé + évaluation complète de la bougie en formation, clone jeté ; on_close = commit autoritaire). Anti-ré-émission par clé (barre, sens, source, entrée bit à bit), cache borné (50 barres). `Clone` ajouté aux 23 détecteurs + `SmcV12Engine` + `HtfAggregator`. **3 tests verts dont le test de fidélité** : sur 700 bars XAUUSD M15 réels, chemins tick-par-tick et par-clôtures aboutissent au MÊME état confirmé et le chemin tick couvre tous les signaux du chemin clôture. Calibration v12 vérifiée compatible M1 (swing=3). Gate 1 reportée au lundi (week-end sans marché indices/métaux) |
| 2026-08-15 | 2 | **2.2/2.3/2.4 terminées (lifecycle intrabar)** — trait `Engine` retourne `SortieMoteur` (signaux + événements) ; `EvenementTrade`/`TypeEvenementTrade` (Fill/Be/Tp1/Tp2/Tp3/Cloture) ; `BusEvenements` sur le runtime. `MoteurV12` : diff d'états lifecycle (`EtatVu`) entre évaluations successives → détection AU TICK des fills/BE/TP/clôtures, phantoms intrabar retirés sans rétraction (R5). **Bug réel intercepté par l'invariant fort** : l'élagage du cache `emis` par fenêtre de 50 barres réémettait en boucle les trades longévifs (bar_created=124 réémis dès la barre 175) — corrigé par élagage par présence dans le carnet. **4 tests verts** dont : clôture n'ajoute RIEN après évaluation tick complète (0/0 sur 700 bars) ; fills/clôtures détectés intrabar sans aucune transition dupliquée. 283 tests workspace |
| 2026-08-15 | 2 | **2.5 terminée (replay harness)** — `engine_v12::replay` (2 modes : clôtures = parité bar-replay TV ; ticks simulés = alertes live, sur-ensemble), verdict `conforme_reference` (carnet plugin == moteur nu, bit à bit), 6 tests. Migration 0069 `runtime_replay` (journal complet JSON archivé). Endpoint `POST/GET /api/runtime/replay[/{id}]` + CLI `replay_v12`. **3 runs réels sur la DB de prod** : BTC M15 (2688 bougies) → 14 signaux, conforme=1 ; XAUUSD M15 (928 bougies, gap mt5/bybit) → 14 signaux, conforme=1 ; BTC M15 ticks → **41 alertes vs 14 clôturées** (sémantique alerte : 27 conditions intrabar évanouies), conforme=1. Point ouvert pour la Gate 2 : zéro TP1 touché sur ces runs — à confronter aux échantillons bar replay TradingView |
| 2026-08-15 | 2 | **2.6 terminée (shadow mode)** — moteurs `MoteurV12` branchés sur les 40 couples du runtime (5 assets × 8 TF), replay de cold start adapté au v12 (~7 jours par TF, plafonné 10 080). Migration 0070 `runtime_emissions` + writer double bus (signaux + événements live persistés à l'émission) + `GET /api/runtime/emissions`. Rétention des émissions alignée sur le journal d'observation (90 j). **Décision Gate 0 n°2 APPLIQUÉE** : chemins SMC timer (signal_engine 5 min + smc_boucle 15 min) éteints au démarrage — remplacés par le runtime. **Fumée 6 min en réel (crypto WE)** : 40 moteurs armés, signaux + événements v12 journalisés en live, concordance conforme. Code des boucles éteintes conservé jusqu'à la bascule 2.8 |
| 2026-08-15 | 1/2 | **Backfill automatique** (décision propriétaire) — `data::backfill` : avant le cold start de chaque couple, les bougies manquantes en queue sont récupérées via REST Bybit (plafond 1 000/appel, v1) — « à l'ouverture, l'historique est là », comme un chart TradingView. Déclenché au démarrage et à chaque nouveau couple coché (UI → runtime ≤ 60 s). Vérifié en prod : nouveaux TF (M1/M30/H4/W1 ajoutés par la migration 0068) kickstartés à 1 000 bougies, queues des anciens TF comblées. Limite v1 documentée : trous *en milieu* d'historique non détectés (seule la queue est vérifiée) — pagination complète avec Dukascopy en phase 5 |
| 2026-08-15 | UI | **Simplification vue Données** (décision propriétaire) — contrôles « Timeframes collectés » et « Historique (backfill) » retirés du pilotage du pipeline : politique fixe (8 TF toujours, rétention 24 mois, backfill auto à l'ouverture) affichée en simple ligne d'information. `MOIS_RETENTION = 24` pilote la profondeur Dukascopy et la référence de couverture DB. Backend inchangé (PUT déjà à champs optionnels — config DB préservée). Frontend build OK |
| 2026-08-15 | UI/DB | **Reclassement forex + indices → Dukascopy** (décision propriétaire) — source `ig` (intégration morte) remplacée par `dukascopy` pour les 15 assets concernés (tous ont leur mapping `datafeed_dukascopy` vérifié en 0066). Badge UI « Dukascopy », libellés de catégories mis à jour, activation par type (crypto/métal → Bybit, forex/indice → Dukascopy), validation backend `binance`\|`dukascopy`. Nuance : Dukascopy = backfill historique aujourd'hui, flux live en phase 5. Nettoyage préalable : suppression XPTUSD/XPDUSD/CAC40/FTSE100/US30/JP225 (741 012 bougies US30, backup CSV 53 Mo) + correction du bug badge GBPJPY (activation qui forçait `binance`) |
| 2026-08-15 | Gel | **ML/LLM remis à zéro et suspendus** (décision propriétaire — modèles entraînés sur les signaux invalides de l'ancien système) — purge des tables d'apprentissage (samples 475, entraînements 1 880, importance 2 404, règles de rejet 8, calibrations), modèles archivés dans `data/backups/modeles_2026-08-15/`. Suspensions : scheduler ML, surveillance 6 h, calibrations ×3, patterns d'échec, **boucles Straddle**, **workers Rockets** (scan/suivi/analyses LLM hebdo SMC+Rockets). Seuls restent actifs : runtime v12 shadow, Bybit WS, journaux, rétention, Telegram, calendrier, pips. Réactivation : après gates 2-3, réentraînement sur signaux validés uniquement. Incident de course intercepté : le scheduler a réentraîné une fois pendant la transition (fichiers supprimés à nouveau) |
| 2026-08-15 | Archi | **Système d'ajout d'assets** (décision propriétaire, exécuté) — `common::Asset` enum → newtype texte (l'asset est une donnée, pas du code) ; maps de symboles → règles génériques (ticker→USDT, contrats linéaires métaux) ; `POST /api/assets` enrichi (symbole Bybit requis / instrument Dukascopy requis, auto-déduit forex, réactivation COALESCE) ; **modale UI** « + Ajouter un asset » (classe → worker imposé, symboles auto-proposés, carte opaque) ; reconnexion WS automatique sur changement de config (empreinte assets×TF vérifiée toutes les 30 s) ; provider Bybit : `retCode` vérifié (erreurs lisibles). **Preuves réelles** : SUI → 1 000 bougies × 8 TF + 8 moteurs armés en ≤ 60 s sans redémarrage ; GBPAUD (Dukascopy) → 960 bougies M15. 375 tests verts. **Protocole T+ « test d'ajout d'asset » ajouté en fin de roadmap** — à exécuter à chaque ajout |
| 2026-08-15 | UI/API | **Suppression de l'import manuel** (décision propriétaire) — badge « MAJ » et bouton « Importer depuis MT5 » retirés du bandeau Données ; `data_csv_handlers.rs` + `data_mt5_handlers.rs` supprimés (~700 lignes : parsing CSV, détection délimiteurs/timestamps, scan MT5) ; routes `POST /api/data/import-csv` et `import-mt5` retirées (404) ; méthode frontend `importerMt5` supprimée. L'entrée de données est désormais exclusivement automatique : WS Bybit (temps réel) + backfill auto (queues) + Dukascopy à la demande. Point ouvert noté : une panique sqlx non fatale (« requires a Tokio context », pool partagé inter-arbiters actix) observée une fois sur un handler — à surveiller |
| 2026-08-15 | Pipeline | **Nettoyage du portefeuille d'assets** (décisions propriétaire) — cryptos réduites à BTC/ETH/SUI (suppression ADA/AVAX/BNB/DOGE/DOT/LINK/SOL/XRP : 355 819 bougies), forex réduit à 6 paires (suppression AUDUSD/CADJPY/EURGBP/GBPUSD/NZDUSD/USDCAD : 2 369 131 bougies), avec les purges précédentes (XPT/XPD + 4 indices : 741 012). **DB : 2,4 Go → 1,22 Go** (VACUUM). Pipeline final : 15 assets (3 crypto + 2 métaux Bybit temps réel · 7 forex + 3 indices Dukascopy). Toutes les suppressions sauvegardées en CSV dans data/backups/ |
| 2026-08-15 | UI | **Compteurs de couverture** — badge 💾 DB (taille réelle via PRAGMA page_count × page_size) + compteur 📈 bougies aujourd'hui (depuis minuit Paris) : les jauges de fonds (% sur 24 mois, Go) sont muettes à l'échelle du jour — le compteur de flux rend le travail journalier visible |
| 2026-08-15 | DB | **Nettoyage signaux toutes stratégies** (avant aujourd'hui, décision propriétaire) — signaux/feedbacks/analyses LLM/positions/snapshots : tables vides, historique repartant de zéro avant le test de vérité. Découverte : `rockets_signaux.cree_le` stocké en texte ISO là où les autres tables utilisent des epochs (dette technique à homogénéiser au prochain passage schéma) |
| 2026-08-15 | Gel | **Suspension des pré-alertes + suivi signaux** — le worker pré-alertes (ancien système : scorer SMC + ATR Straddle sur bougies clôturées) alimentait encore Telegram (**145 messages le 15/08** malgré les suspensions précédentes — rescapé hors de la liste validée, erreur rectifiée). Files Telegram vides ; **la prochaine notification sera un signal v12 validé par le test de vérité** |
| 2026-08-15 | DB | **Purge rockets ouverts** — 7 signaux ouverts (générés à 13h53 avant la suspension du scan) + 390 feedbacks orphelins recréés par le suivi avant sa propre suspension ; tables rockets intégralement vides (vérifié : /api/rockets/actifs → []) |
| 2026-08-15 | Run | **run.sh : fermeture fenêtre (X) = arrêt complet** — watchdog surveillant backend + Tauri + Vite (la mort de l'un arrête tout proprement), diagnostic « 🔍 Cause de l'arrêt » nommant le process décédé avec les dernières lignes de son log, nettoyage des instances résiduelles (Vite fantôme port 1420 par PID, Tauri par nom exact). **Deux incidents pkill -f auto-match corrigés** (motif de chemin matchant l'appelant → terminal tué) — règle désormais inscrite dans le script : jamais `pkill -f` par chemin, uniquement `pkill -x` par nom exact |
