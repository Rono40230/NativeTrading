# ROADMAP AUDIT — OCTOBRE 2026

Issue de l'audit complet du 05/10/2026 (audit-app-2026-09.md pour l'état antérieur).
Complète docs/ROADMAP.md (chantiers produits) — ce fichier ne couvre que la
remise en état et les corrections issues de l'audit.

---

## RÈGLE 1 — VERROU D'ÉTAPE (propriétaire, absolue)

**On ne démarre pas l'étape suivante tant que la précédente n'est pas VÉRIFIÉE.**
Chaque étape ci-dessous se termine par un critère de vérification explicite :
un test automatisé qui passe, ou une constatation dans l'app décrite noir sur
blanc. Aucun cumul d'étapes non vérifiées. Chaque étape vérifiée = un commit
(checkpoint) ; aucun push sans ordre explicite du propriétaire.

Légende vérification : **[TEST]** = cargo test / vitest vert · **[APP]** =
constat visuel ou mesurable dans l'app par le propriétaire · **[DB]** =
requête SQL de preuve.

---

## RÈGLE 2 — ZÉRO RÉGRESSION, ZÉRO DETTE (propriétaire, absolue)

**Interdiction formelle de casser ou de faire régresser le code.** Chaque
correction doit vérifier qu'aucune dépendance n'a été touchée et que SEULE
la cible de la modification a été améliorée — rien ne bouge « à côté ».
Cette roadmap améliore le code sans créer aucune dette technique.

Protocole obligatoire, chaque étape, sans exception :

1. **AVANT — cartographie des dépendances** : lister tout ce qui touche la
   cible (importeurs, appelants, tables, routes) ; toute dépendance
   impactée est soit mise à jour dans la même étape, soit la modification
   est repensée. Décrire avant de corriger.
2. **PENDANT — modification minimale** : la plus petite diff qui corrige la
   cible ; pas de refactor opportuniste « pendant qu'on y est » (toute
   amélioration est une étape à part).
3. **APRÈS — filet de régression complet** :
   - `cargo test` (workspace entier) vert ;
   - `vue-tsc` + `vitest` + `vite build` verts si le front est touché ;
   - `cargo build --release` SANS NOUVEAU WARNING (un warning = dette) ;
   - grep de preuve : zéro référence orpheline aux symboles supprimés ;
   - le bug corrigé est verrouillé par un test de régression dédié.
4. **Commit atomique par étape** : une étape = un commit indépendant et
   révertable — si une régression apparaît, on sait exactement d'où.

---

## PHASE 0 — CORRIGER CE QUI FAUSSE LA MESURE

> Tout ce qui suit trompe l'owner aujourd'hui : une stratégie muette qu'on
> croit vivante, des réglages qui ne règlent rien, un ML nourri de deux
> règnes différents.

### Étape 1 — KDJ : pourquoi zéro signal

**Constat** : 0 signal depuis toujours, alors que le scanner voit des
candidats (« Tendance franche » LINK/SOL le 05/10) et que le labo 7.G a
rejoué des setups gagnants (XRP +1,35 R/tr). Parité rejeur ↔ moteur live
suspecte.

**Actions**
1. Écrire un test de parité : rejouer 90 jours de bougies H1 des 6 assets
   armés (BNB, XAGUSD, XPTUSD, XRP, LINK, SOL) via le rejeur ET via le
   moteur live (`kdj_halftrend::moteur_live` alimenté en clôtures), mêmes
   paramètres — comparer le compte de signaux événement par événement.
2. Si divergence : isoler la première divergence (condition, seuil, bug
   de câblage runtime, fenêtre d'amorce) et corriger.
3. Si concordance (0 setup ne satisfait les conditions) : chiffrer quelle
   condition élimine tout (histogramme des conditions sur la période),
   présenter au propriétaire le compromis (assouplir ADX/amplitude ou
   désarmer la stratégie).

**Vérification** : **[TEST]** test de parité vert, comptes identiques ·
puis **[APP]** soit un premier signal KDJ apparaît (journal + Telegram),
soit décision propriétaire documentée (assouplissement ou désarmement).

> **État 05/10 — CORRIGÉ, en attente de vérification owner.** Diagnostic
> (`cargo test -p api -- --ignored diagnostic_kdj --nocapture`) : parité
> rejeur/live PARFAITE sur données réelles (17-21 signaux/asset sur 5 mois
> H1, ±1 sur LINK par comptage retournement) — le moteur n'était PAS en
> cause. Cause racine : le replay runtime apporte 7 jours = 168 barres H1,
> le warm-up du moteur en exige 260 → ~4 jours de mutisme après CHAQUE
> redémarrage, setups 3-4/mois jamais vus. Correctif : `avec_chauffe`
> (préchauffe 600 barres H1 depuis la base au montage) + garde
> anti-doublon (chevauch chauffe/replay). 293 tests workspace verts,
> release sans warning, test de régression dédié. **Vérification en
> attente : restart + premier signal KDJ dans l'app (ou compteur santé
> étape 4).**

### Étape 1-bis — Préchauffage des caches froids au boot (incident 05/10)

**Constat** : après redémarrage, les deux gros calculs (patterns-jour 24 mois,
matrice événements) se lançaient à l'ouverture de la fenêtre et bloquaient
tous les fetchs du dashboard pendant leurs premières minutes — écran « tout
à 0 » (incident du propriétaire 05/10, recovered après calcul). run.sh
attendait déjà le front et l'API : la course au démarrage n'était pas en
cause.

**Actions** : caches hisés au niveau module + `prechauffer_matrice` /
`prechauffer_patterns_jour` lancés en tâche de fond DÈS le boot (main.rs) —
le même cache est rempli, l'endpoint sert instantanément.

**Vérification** : **[TEST]** 89 + workspace verts, release 0 warning ·
**[APP]** au prochain redémarrage : lignes « 🔥 Préchauffage … » dans le log
boot, et le dashboard affiche ses valeurs immédiatement sans minute morte.

> **État 05/10 — POSÉ, en attente de vérification owner (redémarrage).**
> Validation rétro KDJ livrée le même jour : le moteur chauffé aurait émis
> **9 signaux sur les 7 derniers jours** (BNB ×2 Short, XPTUSD ×2 Short,
> XRP ×2 Long, XAGUSD Short, LINK Long, SOL Long) — preuve que la
> stratégie vivait et que seul le warm-up manquait.

### Étape 2 — Seuils de confiance : rebrancher ou retirer

**Constat** : `seuil_confiance_smc/straddle/rockets` éditables dans l'UI,
lus par aucun code (lecteurs disparus avec la purge ML du 09/09).

**Actions**
1. Décision propriétaire : (A) rebrancher comme vrai filtre à l'émission
   dans `signaux_officiels` (signal dont la confiance ML est sous seuil =
   journalisé latent, non annoncé), ou (B) retirer les trois clés de la
   config et de l'UI (réglage fantôme supprimé).
2. Implémenter la décision. Si (A) : test avec un faux signal sous le
   seuil → bloqué ; au-dessus → annoncé.

**Vérification** : **[TEST]** filtre testé (option A) · **[APP]** les clés
n'apparaissent plus dans la modale config (option B). Plus aucun réglage
mort dans l'UI — grep des clés de config exposées vs lues.

> **État 05/10 — OPTION B POSÉE, en attente de vérification owner.**
> Cartographie : 3 lignes whitelist backend, ZÉRO référence frontend
> (l'UI ne les affichait même pas), 3 lignes en base. Retrait : whitelist
> + migration 0116 (DELETE). 39 suites vertes, release 0 warning, grep
> zéro référence orpheline. Le vrai filtre de confiance reviendra avec
> l'étape 12 (advisory sur données réelles).

### Étape 3 — Rockets : silence et positions dormantes

**Constat** : aucun signal depuis le 22/09 (vérifier si rareté normale —
9 signaux en un mois) ; 3 positions ouvertes depuis 13 jours sans décoller
(sortie = −1R ou R1+trailing uniquement : aucune sortie de stagnation).

**Actions**
1. Diagnostic du silence : logs des passages du scanner D1, candidats
   journalisés depuis le 22/09, seuils de classement — dire si c'est la
   rareté (rien ne casse) ou une panne (scanner bloqué).
2. Ajouter une sortie de stagnation : paramètre cartes Rockets « time-stop
   stagnation N jours » (défaut proposé : 10 jours sans R1 → clôture au
   prix courant, verdict `stagnation`).
3. Traiter les 3 positions dormantes existantes selon la nouvelle règle.

**Vérification** : **[TEST]** gestion testée (position > N jours → clôturée
avec le bon verdict/R) · **[APP]** les 3 positions dormantes fermées ·
**[DB]** `SELECT COUNT(*) FROM signaux WHERE strategie='rockets' AND statut='Actif'` = 0.

**Complément 05/10 soir (retour owner)** : AMD close **STAG +0,34R** au
premier cycle ✓, mais l'historique l'affichait « ❌ SL » (ternaire en dur
ne connaissant pas STAG) et WBTC/WIF restaient ouvertes : leur `ts_entree`
était en **millisecondes** (bug d'émission crypto d'avant octobre) → âge
négatif → stagnation jamais déclenchée. Correctifs : normalisation
d'unité à la lecture + badge/libellé STAG (historique + formatage des
signaux) + warn visible si klines indisponibles (plus de saut silencieux).
Vérification attendue : WBTC/WIF closes STAG au premier cycle après
redémarrage.

> **État 05/10 — POSÉE, en attente de vérification owner.** Diagnostic du
> silence : le scanner EST vivant (73 candidats scannés le 05/10 à 10:22,
> IA news notée, « 0 signal(s) ») — rareté normale de la stratégie
> (cassure ≥ 3 % + conviction ≥ 40), pas une panne. Sortie de stagnation :
> `stagnation_max_jours` (défaut 10, carte Paramètres › Rockets), règle
> dans `pas_gestion` (ordre journal : invalidation → R1 → trailing →
> stagnation ; jamais une position neutralisée), verdict STAG, R latent
> capté au prix courant. Migration 0117. 39 suites + 16/16 rockets,
> release 0 warning, vue-tsc/vitest/build verts.

### Étape 4 — Télémétrie de silence des moteurs

**Constat** : une stratégie peut être muette des semaines sans alerte.

**Actions**
1. Endpoint `/api/sante/moteurs` : par stratégie — armée (oui/non), jours
   depuis le dernier signal émis, nombre de signaux 7 j, état des boucles
   (dernier passage scanner/gestion/collecte).
2. Carte dashboard (pedestal ou bloc Données) : « jours depuis dernier
   signal » par stratégie, pastille rouge au-delà d'un seuil par stratégie
   (SMC 3 j, straddle 2 j, rockets 15 j, KDJ 30 j — réglable).

**Vérification** : **[APP]** la carte affiche KDJ en rouge le jour même
(preuve immédiate que la détection fonctionne), SMC/straddle en vert.

> **État 05/10 — POSÉE, en attente de vérification owner.** Endpoint
> `/api/sante/moteurs` (armée, jours de silence, signaux 7 j, seuil,
> alerte — une stratégie armée muette au-delà de son seuil, ou depuis
> toujours, passe en rouge) + ligne « Moteurs » dans la fenêtre Données
> du pedestal (rafraîchie au chargement). Seuils : SMC 3 j, straddle 2 j,
> rockets 15 j, KDJ 30 j. Test dédié sur base mémoire (silence, seuils,
> jamais-alerte si désarmée). 39 suites, release 0 warning, front vert.
> L'état fin des boucles (dernier passage scanner/collecte) est différé
> à la page santé complète (P2).

### Étape 5 — ML straddle : purger l'avant-pivot

**Constat** : `ml_training_samples` contient 290 échantillons straddle dont
une partie d'avant la suppression du vécu (28/09) — deux règnes mélangés,
contraire à la décision « nouvelles bases ».

**Actions** : purge des échantillons straddle antérieurs au 28/09 19:00
UTC ; vérification du compte restant (les ~189 passes post-pivot).

**Vérification** : **[DB]** `SELECT COUNT(*) FROM ml_training_samples WHERE
strategie='straddle' AND cree_le < pivot` = 0, et total = passes post-pivot.

> **État 05/10 — FAITE et VÉRIFIÉE [DB].** 101 échantillons avant-pivot
> purgés (migration 0118, frontière 17:30 UTC — trou net de 6 h entre les
> deux règnes). Après purge : 216 échantillons straddle = EXACTEMENT les
> 216 signaux fermés post-pivot — collecte complète et mono-régime.
> 39 suites, release 0 warning.

---

## PHASE 1 — HYGIÈNE ET COHÉRENCE

### Étape 6 — Nettoyage frontend mort

**Actions** : supprimer `HoraireHeatmapPrecisionPanel.vue`,
`RocketsAnalyseLlm.vue`, `useProbaHeatmap.ts`, le répertoire vide
`components/reglages/`, les 3 méthodes API orphelines
(`analyserPrecisionHoraire`, `getDerniereAnalyseLlmRockets`,
`lancerAnalyseLlmRockets`) et leurs routes si plus rien ne les sert.

**Vérification** : **[TEST]** vue-tsc + vitest + vite build verts · grep
zéro référence aux symboles supprimés.

> **État 05/10 — FAITE et VÉRIFIÉE [TEST].** Supprimé : 2 composants, 1
> composable, répertoire vide reglages/, 3 méthodes API + leurs types
> (PrecisionHoraire, RocketAnalyseLlm), 2 routes + 2 handlers, le module
> strategies::straddle_precision (203 l.), la chaîne LLM rockets_analyse
> (module llm + prompt catalogue + persistance db + module db::rockets
> vidé puis supprimé). La table rockets_analyses_llm rejoint la purge de
> l'étape 8. Découverte en passant : le module LLM smc_analyse n'a PLUS
> de déclencheur vivant (aucune route / page ne l'appelle) — décision
> propriétaire à l'étape 7 (retirer ou recâbler un bouton Analyse SMC).
> 39 suites, release 0 warning, front 12/12 + build verts, grep 0
> référence orpheline.

### Étape 7 — Routes backend sans consommateur (9)

**Actions** — décision par route, avec le propriétaire :
- `GET/POST /api/straddle/analyste*` (2) : câbler dans l'agenda Straddle
  (avis de l'analyste LLM) ou supprimer ;
- runtime replay/concordance (5) : outils de diagnostic — garder si
  utilisés en CLI, sinon supprimer ;
- `GET /api/creneaux-volatilite` (1) : supprimer (doublon de
  patterns-jour) ;
- `PUT /api/worker/config` (1) : recâbler dans Données ou supprimer.

**Vérification** : **[APP]** chaque route restante est appelable depuis
l'UI ; grep final routes ↔ appels sans orphelin des deux côtés.

> **État 05/10 — FAITE et VÉRIFIÉE [TEST].** Décisions propriétaire :
> runtime ×5 gardé ; PUT worker/config supprimé (5a, lecture seule) ;
> smc_analyse supprimé (doublon pré-fusion du générique /api/analyses/
> {strategie}/ia — le bouton « Analyse » des graphiques appelle
> /api/smc/v12/analyse, technique, indépendant) ; analyste straddle
> dédié supprimé (décision déléguée : doublon stratégique sans
> consommateur, git le conserve) ; chaîne creneaux-volatilite complète
> supprimée (endpoint + job + module + spawn + table, migration 0119) +
> prompt creneaux_proposition orphelin depuis la phase 3. Découverte :
> l'analyse IA par stratégie EXISTE déjà (Rapport d'activité, onglet par
> stratégie, testée en direct sur les 4). 39 suites, release 0 warning,
> grep 0 orphelin.

### Étape 8 — Tables mortes + incohérence fetch

**Actions** : migration 0116 — `DROP TABLE straddle_creneaux`,
`DROP TABLE sentiment_historique` (write-only) ; remplacer le
`fetch('/api/whale')` relatif de SmcScannerView par le client axios.

**Vérification** : **[DB]** migration appliquée, tables absentes ·
**[APP]** badge 🐋 toujours vivant dans le scanner SMC.

> **État 05/10 — FAITE et VÉRIFIÉE [TEST+DB].** Migration 0120 (DROP
> straddle_creneaux + sentiment_historique — vérifié : zéro lecteur, la
> « veille » servie vient de figer_veille_marche, décision 18/08) ;
> l'écriture sentiment_historique retirée avec la table ; fetch whale →
> axios partagé. 39 suites, release 0 warning, front vert.

### Étape 9 — Fichiers et artefacts

**Actions** : purger `data/` (comparatif\*.txt, etape4\*.txt, dev.db,
CSV 24 Mo, backups d'août), `A_faire.txt`, `scripts/test.sh`,
`scripts/backup.sh` (copie 4,4 Go sans limite — le remplacer par un backup
WAL-checkpointé limité si besoin) ; archiver
`backup_straddle_suppression_2026-09-28.sql.gz`.

**Vérification** : `run.sh` complet vert après purge ; `git status` propre.

> **État 05/10 — FAITE.** Études d'août (comparatif/etape4/passe_finale/
> validation_spx), dev.db, CSV référence 24 Mo, A_faire.txt,
> scripts/test.sh + backup.sh (mars) supprimés ; artefacts 15/08 dans
> backups/ purgés ; sauvegarde straddle 28/09 ARCHIVÉE dans backups/.
> Zéro référence dans les scripts. **Découverte : run.sh sauvegarde la
> base à CHAQUE démarrage — 34 sauvegardes de ~2,2 Go = 71 Go
> d'accumulation** (rétention à 30, jamais appliquée aux anciens noms
> soulignés). Décision owner 05/10 : rétention 5 + pièces historiques
> (corruption, secours, avant_reset) dans run.sh + purge immédiate →
> **71 Go → 17 Go** (54 Go récupérés).

### Étape 10 — Fraîcheur des sources de prix

**Actions** : dans le bloc Données du pedestal, une ligne par source
(mt5, bybit_ws, binance) : « dernière bougie il y a X min », pastille
rouge au-delà de 5 min ; décision propriétaire sur l'unification
Binance → Bybit (les 11 actifs binance en retard de ~20 min).

**Vérification** : **[APP]** la carte reflète le retard Binance en direct ;
après unification, plus qu'une source crypto, fraîche.

> **État 05/10 — POSÉE, en attente de vérification owner.** GET
> /api/sante/sources (fraîcheur M1 par source vivante, seuil 5 min) +
> ligne « Sources » dans la fenêtre Données. Mesure du jour : MT5 ✓ 2 min,
> Bybit ✓ 2 min, **Binance morte depuis 11 h** — et vérifié : ses 11
> actifs sont TOUS couverts par Bybit — puis CORRECTION après vérification
> owner : binance n'est PAS un flux mais le COMBLEMENT de trous (décision
> 15/08, n'écrit qu'au montage d'un couple quand un trou existe). Son âge
> = temps depuis le dernier trou comblé, pas une santé : la carte le
> montre en informatif (« comblement, dernier trou il y a X h »), seuls
> les flux continus (MT5, Bybit) alertent en rouge à 5 min. NE PAS
> supprimer — fonction vivante.

### Étape 11 — Stats ML branchées sur les vraies données

**Actions** : les statistiques feedback (`/api/ml/feedback/stats`,
monitoring-ML par stratégie) lisent les tables `*_feedback` vides — les
faire lire `ml_training_samples` (réelles) ; ou alimentons les tables
feedback depuis les clôtures (une des deux, pas les deux).

**Vérification** : **[APP]** monitoring-ML affiche des chiffres non nuls
cohérents avec les signaux fermés.

> **État 05/10 — CORRIGÉE, en attente de vérification owner.** Constat
> affiné : les globales SMC lisaient DÉJÀ ml_training_samples (438
> trades réels) ; rockets/straddle renvoyaient null — SQL cassé
> silencieux depuis septembre (colonnes gagnant/pnl_r inexistantes).
> Alignées sur le patron SMC (rr_realise) + test de régression dédié
> (exclusion expire/invalide, WR, R moyen). Les ventilations par score/
> kill zone restent vides (tables sans écrivain — vides honnêtes, pas
> de fiction).

---

## PHASE 2 — FAIRE FRANCHIR UN PALIER

### Étape 12 — Advisory ML honnête (sur le labo existant)

**Actions** : dans la page Labo de chaque stratégie, un bloc
« Recommandation » : le meilleur essai de balayage vs la configuration
courante (ΔR sur la fenêtre, robustesse), avec bouton « Activer cette
config » (clic propriétaire uniquement). Pas de suggestion inventée :
seulement ce que les essais ont mesuré.

**Vérification** : **[APP]** recommandation visible par stratégie, ΔR
chiffré, activation = réglages réellement écrits (relecture = valeurs
nouvelles — le bug du SELECT ne doit pas pouvoir revenir, un test
d'aller-retour existe déjà).

### Étape 13 — Rockets : détection intrajournalière

**Actions** : cassure du pivot D1 détectée en M15 (au lieu d'attendre la
clôture D1) ; backtest comparatif D1 vs M15 sur l'historique avant
bascule live.

**Vérification** : **[TEST/APP]** backtest chiffré (fréquence, R/tr,
drawdown) présenté au propriétaire ; bascule seulement sur décision.

### Étape 14 — Rétention de la base

**Constat** : trading.db = 2,2 Go + WAL 2,2 Go.

**Actions** : politique de rétention par TF (M1 > 12 mois archivé ou
supprimé, propore au labo de conserver l'agrégé) ; job de purge +
`PRAGMA wal_checkpoint(TRUNCATE)` ; mesurer avant/après.

**Vérification** : **[DB]** taille mesurée avant/après ; **[APP]** labo et
matrice réactivité toujours alimentés (la réactivité n'a besoin que de
120 jours de M1).

### Étape 15 — (Optionnel) Entrée straddle à l'horloge garantie

**Actions** : timer runtime armé à T-N s par créneau : la passe s'ouvre au
dernier prix connu même sans print dans la fenêtre (utile uniquement pour
les créneaux calmés — les jours d'annonce réelle, les prints suffisent).

**Vérification** : **[TEST]** entrée à T-N s sans tick dans la fenêtre ·
**[APP]** passes à 14:29:57 le jour PCE/NFP.

---

## SUIVI PASSIF (données qui mûrissent, aucune action)

- Re-test conviction à ~500 signaux notés (prompt recalibré, daff4fe).
- 2.3 M30 SMC — ~fin septembre (à relire).
- 1.1 straddle vécu (nouveau régime événementiel) — première lecture ~11/10.
- 2.1 SP500 flat — ~3 semaines.
- Labo whale : accumulation des signaux enrichis.

## ORDRE ET VERROUS

1 → 2 → 3 → 4 → 5 (Phase 0, chaque étape vérifiée avant la suivante),
puis 6 → 11 (Phase 1, ordre indicatif mais verrou identique), puis
12 → 15 (Phase 2). Un commit par étape vérifiée ; push sur ordre only.
