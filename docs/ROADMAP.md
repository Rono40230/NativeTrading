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

> ### 📚 Références canoniques (recherche web 2026-08-17 — avant toute implémentation)
>
> **ROCKETS = VCP (Volatility Contraction Pattern, Mark Minervini)** — fiche complète : `docs/superpowers/research/2026-08-17-rockets-straddle-recherche.md`
> - **Contexte requis** (Trend Template) : prix > MA150 > MA200, MA200 montante ≥ 1 mois, prix ≥ 30 % au-dessus du bas 52 semaines, ≤ 25 % sous le haut 52 s, RS ranking > 70
> - **Le pattern** : 2-6 contractions **strictement décroissantes** (séries types 25→15→8→4 %), base 3-12 semaines, volumes qui s'assèchent (VDU : 40-60 % de la moyenne 50 j), ATR final ≈ 1/3 de sa moyenne
> - **Le pivot** : haut de la contraction finale ; entrée buy-stop +1-2 % au-dessus, stop sous le bas de la dernière contraction
> - **Confirmation breakout** : volume ≥ 140-150 % de la moyenne (sources divergent 130-150 %)
> - **Adaptation crypto à définir** : pas de méthodologie VCP crypto formelle publiée — le projet doit trancher : RS vs BTC, pondération week-end 24/7, profondeurs élargies
>
> **STRADDLE = news trading par ordres stop** — même fiche
> - **Mécanique** : buy-stop + sell-stop 10-30 pips au-delà du range pré-annonce (15-30 min), posés 1-5 min avant l'annonce, **OCO forcé à la seconde du fill**
> - **Annonces tier 1** : NFP + CPI (08:30 ET), FOMC (14:00 ET + conférence 14:30 = **double vague**) ; bougie d'annonce = 3-10× ATR normal
> - **Gestion** : SL 15-30 pips, TP 3-5× SL, breakeven dès profit = SL, time-stop 60 min
> - **Risques chiffrés** : spread EUR/USD 1→5-10 pips à l'annonce, slippage 5-15 pips, whipsaw/chasse-liquidité sur straddle naïf trop serré
> - **Honnêteté** : aucune validation académique — les win-rates publiés viennent de vendeurs ; la gate 3 (replay + sessions live) tranche

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
| L8 | **Vite résout `.js` AVANT `.ts` sur import nu** : aucun fichier `.js` compilé ne doit vivre dans `src/` (`noEmit: true` verrouillé dans tsconfig). Toute vérification de code servi doit passer par ce que le navigateur charge réellement (module résolu, requête réseau), pas par la lecture du `.ts` | Chart « 3 jours » : 208 sorties tsc périmées shadowaient les `.ts` corrigés — chaque correctif était vérifié dans le `.ts` mais jamais servi à l'app |
| L9 | **Ajouter un fichier SQL dans `db/migrations` ne recompile PAS le crate porteur** : `sqlx::migrate!` embarque la liste au compile-time et cargo ne voit pas l'ajout de fichier. Après TOUTE nouvelle migration : recompiler le crate db (`touch` si besoin) ET vérifier `SELECT version FROM _sqlx_migrations` après boot — « migrations OK » au boot ne prouve rien | Bloc sentiment vide : migration 0073 absente du binaire (rlib db du 11/08), `run()` réussissait silencieusement sans rien appliquer |
| L10 | **Une seule série de prix par couple asset×TF — vérifiée contre l'API officielle** : plusieurs writers (WS, backfill, fallbacks) peuvent écrire la même table avec des séries DIFFÉRENTES (spot vs perp ~30 $ d'écart, étiquettes mensongères). La concordance interne (runtime↔DB) ne prouve rien sur la cohérence AVEC LE MONDE : auditer DB↔API officielle avant toute confrontation de fidélité | Échantillon 2.7 n°1 : 194 barres BTC M15 = perp linear Bybit (WS runtime), 190 = spot (backfill REST) entremêlées → moteur sur un patchwork, signaux divergents de TV dans les deux sens malgré un port sain |
| L11 | **Aucune lecture d'image présentée comme fait sans contre-vérification humaine** : l'analyse d'image peut « lire » ce que le contexte attend (hallucination plausible — elle a retranscrit une table avec exactement nos valeurs attendues, fausses). Pour toute donnée décisive : dictée humaine ou extraction programmatique vérifiable | Confrontation OB : une lecture de table TV inventée a produit un faux verdict de concordance ; les valeurs dictées par le propriétaire ont révélé la vérité (offset de source ~60 $) |

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
| 2 | v12 fidèle | **Pyramide de vérification** (0 bougies → 1 zones SMC → 2 scoring → 3 logique d'entrée → 4 signaux) — chaque étape validée avant la suivante, décision propriétaire 2026-08-18 | ⬜ |
| 3 | Plugins fidèles | Par stratégie : replay + 2 sessions + non-régression v12 | ⬜ |
| 4 | Extensions isolées | Kill producteur sans impact + dashboard | ⬜ |
| 5 | Nouvelles sources | Gate 2 rejouée par asset + couverture DB | ⬜ |

# Prochaines étapes (état au soir du 2026-08-18)

> Voies parallèles actives : la preuve du cœur (2.x) et le dev des modules
> indépendants avancent séparément — les gates bloquent l'intégration, pas le dev.

| # | Étape | Nature | Prérequis |
|---|---|---|---|
| 1 | **Investigation queue D1 ETH** — D1 figé au 26/03 en DB (BTC à jour) ; les moteurs v12 shadow ETH s'appuient sur la queue au cold start. Vérifier pourquoi le backfill auto/WS ne l'alimente plus | Autonome (agent) | aucun |
| 2 | **Phase 2.7 — Test de vérité** : 5 sessions live vs TradingView — le propriétaire note ses alertes TV (bar replay), confrontation au journal `GET /api/runtime/emissions` | **Participation proprio** | sessions de marché |
| 3 | **Phase 2.7-bis — Audit indicateur par indicateur** v12 vs TradingView (proposition propriétaire) : chaque détecteur vérifié sur des cas connus (BOS/CHoCH/OB/FVG/sweeps…) | Semi-autonome | échantillons TV |
| 4 | **Phase 2.8 — Bascule officielle** : signaux v12 officiels (Telegram sur le bus < 1 s), suppression de l'ancien chemin (smc boucles, signal_engine) | Autonome | **Gate 2 (2.7)** |
| 5 | **Phase 3 — Plugin Straddle** (définition canonique sourcée en tête de Phase 3) puis **Rockets** (VCP Minervini) | Autonome | définitions ✅ |
| 6 | **Phase 5.2 — Sync Dukascopy incrémentale** : les 10 assets Dukascopy n'ont que leur backfill initial (~410 bougies D1, figées) | Autonome | pattern curseur-DB ✅ |
| 7 | Maintenance consignée : UX fine presse, panique sqlx (« requires a Tokio context »), timestamps hétérogènes (ISO vs epoch), troncature UTF-8 | Autonome | — |

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
| 2026-08-17 | 4.2 | **Sentiment = référence veille** (décision propriétaire — le flux tendu 30 min ne convenait pas) — le composite servi (`GET /api/sentiment/composite`) est désormais la **moyenne du J-1** par classe (5 composites + composantes rsi/fg/vix moyennées), figée avant l'ouverture de J ; persistance densifiée à chaque cycle 30 min (avant : 1 snapshot/jour) ; fallback live le premier jour sans historique. **Affichage strictement inchangé** (carte, pastilles, cours, F&G) — seule la source des scores change. Vérifié : 17/08 → moyennes du 16/08 servies |
| 2026-08-17 | UI | **Dashboard réorganisé** (décision propriétaire) — gauche : surveillance assets + setups en formation (pleine hauteur, ex-emplacement presse) ; droite : sentiment (référence veille) + calendrier économique ; grille stratégies élargie au centre |
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
| 2026-08-17 | UI | **Chart aligné TradingView Basic — incident « ~3 jours de bougies » résolu** (plusieurs correctifs successifs sans effet). Cause racine : **208 fichiers `.js` compilés résiduels** dans `src/` (sortie d'un `tsc` sans `noEmit`) — Vite résout `.js` AVANT `.ts` sur import nu, donc `useChartLimite.js` (ancienne logique par jours : M15=384 bougies ≈ 4 j) shadowait le `.ts` « aligné » : chaque correctif était vérifié dans le `.ts` mais jamais servi. Correctifs : suppression des 208 `.js` (non trackés git), `noEmit: true` verrouillé dans tsconfig (leçon **L8**), `useChartLimite.ts` simplifié en **5000 bougies pour tous les TF** (parité TV Basic réelle). Preuves : API via proxy Vite = 5000 bougies (04/03→17/08), compteur DOM « Bougies : 5001 », axe du chart défilé jusqu'à **03/04/2026**, `vue-tsc && vite build` OK |
| 2026-08-18 | UI/API | **Bloc Sentiment 100 % « référence veille »** (décision propriétaire) — diagnostic : les jauges composites étaient déjà figées (moyenne J-1) mais les listes USA/Europe/matières/cryptos/VIX venaient de `/api/sentiment/marche` en **flux live** (Yahoo séance en cours + Binance 24 h, refresh front 60 s). Refonte : migration `0073_sentiment_marche_veille` (clôtures par entité/datée, `INSERT OR REPLACE` idempotent) ; collecteur branché au worker 30 min — Yahoo `interval=1d` **barres clôturées uniquement** (jamais la séance en cours) pour indices/matières/VIX + **DB locale D1 Bybit** pour BTC/ETH ; `GET /api/sentiment/marche` ne fait plus AUCUN fetch externe (sert la dernière référence figée, date = jour de clôture majoritaire ; one-shot du collecteur au premier lancement). Front : libellé « Référence veille · date », refresh 60 s → 5 min. Tri des sources proposées par le propriétaire : Alternative.me (déjà intégré) et Yahoo daily retenues ; FRED VIXCLS = **plan B noté** si l'API non officielle Yahoo casse ; Adanos/ConvexTrade/motosan-finance écartés (probables hallucinations) ; yfinance écarté (Python, hors stack) |
| 2026-08-18 | Run | **Incident bloc vide au premier lancement** — la migration 0073 n'était PAS embarquée dans le binaire : `sqlx::migrate!` fige la liste au compile-time et l'ajout d'un fichier `.sql` ne recompile pas le crate `db` (rlib daté du 11/08) — `run()` « réussissait » sans rien appliquer (leçon **L9** : recompiler le crate db + vérifier `_sqlx_migrations` après boot). Correctifs : recompilation forcée (0073 vérifiée embarquée via `strings`), table créée en direct (idempotent, sqlx la marquera proprement au prochain boot), one-shot re-figé. Diagnostic complémentaire : les indices cash Yahoo servent leur dernière close **publiée** (vendredi 14 — les barres 17/18 ont `close=null` : dégradation correcte à la dernière close dispo) ; futures (or/argent/pétrole) et cryptos au 17. **Découverte données : bougies D1 ETH figées au 26/03 en DB** (BTC à jour) → garde d'ancienneté ajoutée dans `cloture_db` (> 4 jours = entité ignorée + warn, 5 tests verts) ; investigation de la queue D1 ETH à faire |
| 2026-08-18 | Pipeline | **Réparation complète d'ETH + comblement de trous dans backfill_profond** (chantier n°1 des prochaines étapes) — cause racine : **ETH `actif=0` dans assets** (désactivé vraisemblablement à la purge crypto du 15/08 ; TFs intraday alimentés jusqu'au 14/08, TFs longs jamais rattrapés depuis l'import initial de mars/avril — et les 6 assets Dukascopy du composite sentiment étaient aussi figés début avril, inactifs par conception). Réparation : réactivation → le runtime a déployé les 8 moteurs v12 ETH avec replay (≤ 60 s, vérifié logs) + backfill auto de queue ; **backfill_profond apprend à combler les trous** : détection SQL exhaustive (plus grande bougie sans successeur ET non-dernière), pages montantes clampées sur la fin du trou, boucle multi-trous — 3 bugs interceptés en route (plancher écrasé par le haut comblé ; la dernière bougie masquait les trous via MAX ; fenêtre initiale sautait les trous < 1000 bougies). **Résultat : 226 000+ bougies insérées, 0 gap sur les 8 TF** (M1 contigu 28/08/2024→maintenant, 1,03 M bougies). Garde d'ancienneté partagée avec le composite (`trop_ancienne`, pub(crate)) : les assets D1 figés > 4 jours sont exclus du score technique (renormalisation `combine_dispo`) au lieu de fausser les jauges. 66 tests verts |
| 2026-08-18 | 2.7 | **Protocole de test de vérité REFONDU en rétrospectif** (objection propriétaire fondée : surveiller 4 assets × 8 TF en simultané impossible + TV gratuit ne logge pas les alertes + l'indicateur Pine efface ses signaux en fin de vie → aucune trace historique lisible). Nouveau protocole : confrontation par échantillons — l'agent produit les listes de signaux (journal `runtime_emissions` ou replay), le propriétaire vérifie sur TV qu'il voit les mêmes marqueurs aux mêmes barres, y compris des **fenêtres blanches** (aucun signal attendu). Condition débloquante : **patch « Mode audit » appliqué à l'indicateur Pine** (`smc_indicateur_v12.pine`, off par défaut) — input `Mode audit`, losanges de CRÉATION (comparateur 1:1 des émissions), flèches d'alerte, croix de break-even, posés sur les MÊMES transitions que les `alert()` existantes → le recalcul historique TV reproduit la séquence exacte qu'un live aurait émise |
| 2026-08-18 | 2.7 | **Échantillon n°1 : NON CONCLUDANT sur le port, DÉCISIF sur les données** — confrontation BTC M15 15-17/08 avec le patch audit (losanges TV du propriétaire vs journal/replay). Phase 1 : le journal live mélange réémissions tick (4 shorts intrabar jamais confirmés en clôture) → le BON comparateur du Pine recalculé = le replay clôtures. Phase 2 : replay 2 semaines → 2 MATCHS EXACTS avec TV (Short 14/08 07:45 UTC = losange 09:45 Paris, Long 17/08 14:45 UTC = losange 16:45 Paris) + élimination de l'hypothèse warm-up (replay 9 semaines : mêmes signaux). Phase 3 : audit des BOUGIES → **cause racine : la table BTC mélange deux séries de prix** — 194 barres `bybit_ws` = **perp linear Bybit** (worker WS souscrit en linear pour tous), 190 barres backfill = **spot** (provider REST : spot pour BTC/ETH) ; écarts ~30 $ entre familles, 51,6 % des barres divergent de l'API spot. XAU/XAG : backfill et WS tous deux linear = cohérents (pas de mélange). Leçon **L10** gravée : audit DB↔API officielle avant toute confrontation (la Gate 1 ne comparait que runtime↔DB, auto-concordance — point aveugle) |
| 2026-08-18 | 2.7 | **Recadrage propriétaire + décision de référence + unification de la série de prix** — le propriétaire valide la **pyramide de vérification** (bougies → indicateurs SMC → scoring → logique d'entrée → signaux ; une variable par étape) comme protocole officiel de la Gate 2. **Série de référence tranchée : `BYBIT:BTCUSDT` (SPOT)** — le chart TV du propriétaire, prix synchro avec l'app. Réparation de l'étape 0 : worker `bybit_ws` scindé en **2 sessions par marché** (spot pour les cryptos, linear pour XAU/XAG — avant : tout en linear), chart de l'app redirigé de stream.binance.com vers **Bybit spot**, étiquette par défaut de `inserer_bougies` corrigée (`binance` mensonger → `bybit_rest`), **4 672 bougies perp purgées** (BTC 4 327 + ETH 345) et re-backfillées en spot (audit : 99 % exactes vs API spot ; le reliquat `bybit_ws` réécrit par le process vivant sera purgé au redémarrage). 99 tests verts, build release OK |
| 2026-08-18 | soir | **Bug post-redémarrage + retraits + sentiment option A** — (1) bug critique : le tick de reconnexion comparait l'empreinte de TOUS les assets au sous-ensemble filtré de la session (spot/linear) → reconnexion en boucle toutes les 60 s **sans aucune bougie ingérée** ; le filtre est appliqué aux deux empreintes. (2) **ETH retiré du pipeline** (décision propriétaire : BTC seul) : soft-delete + purge 1 370 220 bougies (backup CSV) + retrait des sources sentiment + ligne veille purgée ; reliquat perp BTC (28 barres) purgé/re-comblé (329). (3) **Bloc sentiment, option A hybride** (décision propriétaire) : les LISTES passent en **variation du jour live** (Yahoo séance + Bitcoin calculé sur NOTRE série DB Bybit spot — jamais Binance, L10), jauges composites en référence veille inchangées, repli sur la veille figée si le live échoue (< 5 sources), libellé « Jauges : réf. veille · Marchés : aujourd'hui », refresh 2 min. **Échelle des pastilles recalibrée par classe** (avant : ±0,3 % partout — un indice quasi plat s'affichait rouge) : indices ±1 % (ajusté par le propriétaire juste après livraison), matières ±0,75 %, cryptos ±2 % ; la couleur du texte suit la même échelle |
| 2026-08-19 | 2.7 | **Nettoyage chart + étape 1 du miroir OB : détection, naissance, mort, âge ALIGNÉS ; dernier point noir = disparition TV des zones du 17/08 nuit** — ancien moteur SMC v1 supprimé du graphique (8 boutons + ~1 600 lignes de code mort, commit dff2d52) ; filtre d'affichage OB ≥5/10 porté dans l'app (parité Pine) et rendu réglable côté Pine (input, défaut 5) ; fenêtre d'analyse v12 passée de 500 à 5 000 bougies (parité d'âge TV Basic : un OB vit jusqu'au toucher de son bord, FIFO 40/sens — règles identiques dans le port). Confrontation des 5 zones actives : **3/5 concordantes + scores alignés**. Les 2 écarts (nées 17/08 ~03:00/04:15 Paris) éliminés un à un : bougies identiques (audit 21/21 + lecture d'axe), impulsions détectées des deux côtés (mode debug IMP), zones créées des deux côtés (marqueurs NEW), toucheurs antérieurs aux naissances, seuil ROC 5 bps identique, garde identique. Restent deux candidats : score vivant TV (proximité >10×ATR → score forcé 0 ; freshness négative) et surtout **FIFO 40** (éjection silencière non instrumentée) : ~40 impulsions bull depuis le 17/08 → les zones de la nuit éjectées chez TV mais vivantes au port (7 zones actives dont une du 13/07 → le FIFO du port suspecté de ne jamais éjecter). Compteur de zones ajouté au debug Pine (table top-right) — 2 nombres à lire trancheront |
| 2026-08-19 | 2.7 | **ÉTAPE 1 (miroir Order Blocks) : RÉUSSIE à l'écart de source près** — par la table debug dictée par le propriétaire (7 bull + 5 bear TV vs notre analyse 20 000 bougies) : **9/9 zones appariées structurellement** (8 bull + bear 10/08), bornes concordantes à ~10-25 $ près une fois retiré un **offset de source ~−60 $** entre les klines historiques TV et l'API Bybit (prouvé : prix live TV = API à la volatilité près ; nos bougies = API au dollar ; zones TV systématiquement décalées). Détection, naissance, mort, âge, FIFO, population et scoring des zones visibles : **fidèles**. Résidus à instruire (étape suivante) : 4 zones bear TV anciennes (juin, 66,9-71,3k) sans contrepartie à 20k bougies ; score vivant non porté (freshness + proximité ATR : nos zones 17/08 affichées 5/10, TV <1 → invisibles) ; origine de l'offset de source TV. **Leçon L11** : aucune lecture d'image présentée comme fait sans contre-vérification humaine — l'analyse d'image a « lu » une table en produisant exactement nos valeurs attendues (hallucination) ; seules les valeurs dictées par le propriétaire font foi |
| 2026-08-19 | 2.7 | **Étape 2 engagée — alignement lifecycle sur la référence MQL5** — découverte propriétaire : le Pine v12 a un port MQL5 prouvé miroir (`EA MT5/smc_indicateur_v11.mq5`, 4 766 lignes) : troisième source de vérité, structurellement proche du Rust. Analyse croisée Pine↔MQL5↔Rust : le Rust AVAIT déjà le score vivant complet (freshness/proximité −999/ratchet/prune/signaled) ; la différence était le PLACEMENT de la mitigation : le MQL5 documente le bug (« mitigation dans le else → OB ne transitent jamais → ratchet bloqué → sur-scoring, ex OB à 20×ATR gardait 9 ») : `lifecycle_ob_bull/bear` réorganisés (mitigation AVANT suppression, toute barre touchante) + test verrou (`mitigation_avant_suppression_sur_barre_touchante_posterieure`), 188 tests verts. Dettes repérées au passage : Asian High omis dans zn_qual (concern noté), Breaker décoratif (phase 5), diagFlags MQL5 à porter. Reste l'écart de FORCE sur les zones partielles 17/08 (nous 5/10, TV invisible) : mesure par dichotomie du seuil d'affichage TV demandée au propriétaire |
| 2026-08-19 | 2.7 | **Étape 2 outillée et première lecture — écarts de scoring BIDIRECTIONNELS** — diagFlags du MQL5 porté dans le Rust (16 composantes mémorisées au nouveau max du score de chaque zone, exposées via API + binaire `debug_zones`) ; la table debug TV affiche `sc=` par zone. Première confrontation (BTC M15 5 000) : écarts dans les DEUX sens (60524 : TV 10 / nous 6 ; 62946 : nous 9 / TV 7 ; 63564 : 8 = 8 exact) → PAS un biais du port : les composantes (BOS frais, FVG du jour, CHoCH, confluence H4…) ne s'activent pas aux mêmes barres. Prochaine passe : zone 60524 (écart max TV+4) confrontée barre par barre, composante par composante, MQL5 en référence. NOTE de méthode gravée : les observations d'affichage TV du propriétaire priment toujours sur toute lecture automatique d'image (L11) |
| 2026-08-19 | 2.7 | **Vagues 1-2 du miroir indicateur par indicateur : VALIDÉES par inspection directe du propriétaire** — pivots (HH/HL/LH/LL) puis BOS/MSS/CHoCH strictement identiques app ↔ TV en M15. Le socle structurel complet est miroir ; 3 composantes majeures du score (BOS pondéré 5/3/1, MSS +3, CHoCH +4) sortent des suspects de l'étape 2. Ordre de validation établi par vagues de dépendance : 1 pivots ✓ · 2 BOS/MSS/CHoCH ✓ · 3 Sweeps/EQH-EQL · 4 FVG · 5 Sessions/Asian H-L · 6 PDH-PDL-PWH-PWL/NDOG-NWOG · 7 Imbalance/fonds · 8 Prem-Disc/Equilibrium/OTE · 9 OB HTF (+4/+1/+5/+6 : plus gros poids score) · 10 Breakers · 11 BSZones + réglages. Bouton « Structure » renommé « HH/HL/LH/LL » (7e1f64b) |
| 2026-08-20 | Archi | **Décision structurante propriétaire : exécution cible = MT5 chez Axi Corp** — paires : BTC/XAU/XAG/**GER40** (extensibles), terminal sur la MÊME machine que l'app, BTC 24/7. La source de vérité devient le flux Axi (là où les ordres s'exécutent). Plan phase 5 redéfini : EA collecteur MQL5 (pousse bougies+ticks vers l'API locale) → endpoint d'ingestion → provider « axi » (le moteur ne change pas, principe DataProvider) → coexistence Bybit/Axi (BTC chez les deux) → **test de vérité au centime : mêmes bougies Axi dans le MQL5 et le moteur Rust**. RÉSOLUTION VAGUE 3 (leçon majeure) : la série « BYBIT:BTCUSDT » du TV propriétaire a DIVERGÉ de l'API Bybit (~60 $ à l'étape OB → **~6 500 $ le 20/08** : TV 71 000 vs API 64 500) — son Pine est sain (pool figé = tendance continue 63→71k sans pivots égaux, cohérent), notre Rust est fidèle (prouvé par simulation Python du code Pine sur nos bougies : 174 créations, 17 SWP = comportement Rust). CONSÉQUENCE MÉTHODE : les comparaisons visuelles TV↔app sont MORTES pour les indicateurs dépendants des prix (FVG, OB, niveaux) — validation = simulation Pine sur nos données (autonome) puis MQL5 sur flux Axi (provider venu) |
| 2026-08-20 | 2.7 | **Vague 4 (FVG) : VALIDÉE au dollar et à la minute près, du premier coup** — méthode simulation (vague 3) : le code FVG Pine (gap 3 bougies > 0,20×ATR, suppression si close traverse ou âge > 50 barres, partial au toucher, FIFO 10/sens) simulé en Python sur 5 000 bougies Bybit vs export Rust : **4/4 FVG identiques** (bornes au dollar, naissance à la minute, états de mitigation). État des vagues : 1 pivots ✓ · 2 BOS/MSS/CHoCH ✓ · 3 Sweeps ✓ EQH/EQL ✓ (divergence TV documentée, Rust innocenté par simulation) · **4 FVG ✓**. Vagues 5-8 (sessions/AsianHL, PDH-PDL-PWH-PWL/NDOG-NWOG, imbalance/fonds, PremDisc/EQ/OTE) : méthode simulation applicable en autonome |
| 2026-08-20 | 2.7 | **Vagues 5-7 (sessions, Asian HL, PDH/PDL/PWH/PWL, fonds) : vérifiées par inspection croisée Pine↔Rust + 1 bug d'affichage corrigé (bfcab88)** — Kill Zones : constantes UTC IDENTIQUES ✓. Volume fort : SMA20×1.0 ✓. PDH/PDL/PWH/PWL ✓ même définition. NDOG/NWOG : gating TF identique, export vide en M15 BTC (gaps quasi nuls 24/7, attendu). Fond impulsion : collecteur corrigé (RANGE high-low × i_atrSeuil par asset — était corps × seuil_ib ; le scoring était déjà correct). Vague 8 (PD/EQ/OTE) : définitions lues, à comparer |
