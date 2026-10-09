# 📘 MANUEL DE L'UTILISATEUR — Native Trading AI

> **À qui s'adresse ce document** : à toute personne qui découvre l'application et veut se
> servir de toute sa puissance — la paramétrer, lire ses données brutes comme celles de
> l'IA, et modifier des réglages **en comprenant les conséquences**.
> **Double usage** : ce manuel sert aussi de **guide de test intégral** — chaque bloc
> « 🖥️ À l'écran » décrit l'attendu vérifiable ; si l'écran ne montre pas cela, quelque
> chose est cassé ou dé synchronisé.
> **Date** : 09/10/2026 — valeurs de production citées à jour de cette date.
> **🔊 À venir (décision propriétaire 09/10)** : une page « 📘 Manuel » intégrée à
> l'app servira ce fichier en lecture seule — petit chantier séparé, à poser après
> validation du contenu.

---

## Comment lire ce manuel

Chaque fonctionnalité est décrite avec **quatre blocs constants** :

| Marqueur | Contenu |
|---|---|
| 🔧 **Workflow** | le chemin précis, clic par clic |
| 🖥️ **À l'écran** | ce qui doit s'afficher — critère de test |
| 📖 **Lecture** | quel chiffre décrit quoi, avec les conventions de l'app |
| ⚡ **Réglages & conséquences** | chaque paramètre → effet → **délai d'application réel** |

Les délais d'application possibles, à retenir une fois pour toutes :

| Délai | Signifié |
|---|---|
| **Immédiat** | effet visible au prochain affichage/chargement |
| **Tick 60 s** | le runtime relit la configuration toutes les 60 secondes |
| **Hot-reload moteur** | le couple (asset × TF) concerné est réinscrit sous 60 s — voir les logs 🩹 |
| **Relance requise** | uniquement pour un changement de CODE (run.sh reconstruit tout) |

---

# CHAPITRE 1 — Bienvenue & philosophie

## 1.1 Ce qu'est cette application

Native Trading AI est une **salle de marché personnelle de paper trading** : quatre
stratégies de trading déterministes tournent en continu sur des données réelles (MT5 pour
les instruments du broker, Bybit pour les cryptos), émettent des signaux, gèrent des
positions virtuelles et composent un capital simulé. Une couche d'intelligence
artificielle **locale** lit, note et conseille — mais ne décide jamais.

Les quatre stratégies :

| Stratégie | Idée | Unité de temps |
|---|---|---|
| **SMC v12** | Smart Money Concepts : entrées sur order blocks institutionnels, structure HH/HL/LH/LL, scoring 16 composantes | M5, M15, M30 |
| **Straddle** | News trading : deux jambes (achat + vente) posées à T−10 s d'une annonce à fort impact | M1, autour d'événements |
| **Rockets** | Cassures de consolidation (VCP × Rocket Hunter) sur crypto et actions US | D1, gestion à 30 s |
| **KDJ/Halftrend** | Suivi de tendance H1 : croisement KDJ dans le sens des moyennes, arrêt sur l'EMA200 | H1 |

## 1.2 Les cinq règles d'or (la constitution de l'app)

1. **L'IA n'exécute jamais.** Elle lit, juge, note, propose. Chaque activation d'un
   réglage est un clic humain. Telegram reçoit des messages d'imminence, pas des ordres.
2. **Le vécu est immuable.** Un trade ouvert garde pour toujours les niveaux (entrée, SL,
   TP) figés **à son émission**, et les fractions de vente figées **à sa clôture**.
   Modifier un réglage n'affecte que les trades **futurs** — jamais l'historique, jamais
   le capital déjà composé. Les contre-factuels vivent au laboratoire uniquement.
3. **La règle des 30.** Aucune conclusion statistique tant qu'une tranche n'a pas
   **30 clôtures** (par stratégie, par asset, par source d'événement). L'app affiche
   honnêtement « non significatif » en dessous.
4. **Pine = étalon.** Les moteurs sont des miroirs vérifiés d'indicateurs Pine
   (TradingView). Toute déviation est votée explicitement — jamais subie.
5. **Deux voix, jamais de moyenne.** Le **R DISTANCE** juge la stratégie (entrées +
   placement des TP) ; le **$ réel composé** juge le résultat (réglages de sortie
   compris). L'app n'affiche jamais de « R moyen ».

## 1.3 Le cycle de vie d'un trade (à comprendre absolument)

```
Détection (moteur, bougie en formation)
   → ANNONCE D'IMMINENCE (Telegram + app, intrabar — décision 23/08)
   → FILL au prix d'entrée (le trade existe au marché)
   → GESTION (ventes partielles aux TP, BE, trailing, time-stop)
   → CLÔTURE (verdict) → le trade entre au VÉCU (capital composé, analyses)
```

- **Avant le fill** : un ordre SMC « posé » est invisible sur les graphiques et ne compte
  dans aucune statistique (décision 11/09).
- **Après la clôture** : le trade est figé. Les pages « Analyses » le rejouent, le
  laboratoire le simule avec d'autres réglages, mais sa ligne officielle ne bouge plus.

## 1.4 Les conventions de lecture (partout dans l'app)

| Vous voyez | Ça veut dire |
|---|---|
| **+12,4 R** (Σ R distance) | Les trades de cette catégorie ont parcouru, cumulés, 12,4 fois le risque engagé — la qualité des entrées et des objectifs |
| **+850 $** | L'argent réellement gagné, ventes partielles et réglages compris, capital composé |
| **44 % ok / WR** | Part des clôtures gagnantes en dollars ($ > 0) — pas en R |
| **⏳/🟢 En cours** | Trade vivant (ou verdict inconnu — voir glossaire) |
| **✅ TP / ❌ SL / 🔄 Retournement** | Verdicts KDJ |
| **TP3 / TS / TP2+BE / TP1+BE / SL / BE / Expire** | Verdicts SMC & straddle |
| **🏁 TS / ⏹ STAG / 👤 Manuel / ❌ SL** | Verdicts rockets |
| **464/30** | Effectif vs règle des 30 : en dessous de 30, tout est « descriptif » |
| **JUGEABLE / NON JUGEABLE** | Règle des 30 appliquée à un asset précis |

> 💡 **Pourquoi mon R affiché et mon $ ne « collent » pas ?** C'est voulu. Le R distance
> dit où le prix est allé ; le $ dit ce que la mécanique de sortie a encaissé. Leur ÉCART
> mesure la conversion (ventes partielles, break-even, trailing) — pas un défaut de
> sélection des trades.

---

# CHAPITRE 2 — Démarrer l'app

## 2.1 Lancer

🔧 **Workflow** : terminal à la racine du dépôt → `./scripts/run.sh`. Le script
construit le front (dist), compile et lance le backend (api), puis la fenêtre Tauri.

🖥️ **À l'écran** : après ~1-2 minutes ( compilation + cold start), la fenêtre
s'ouvre sur le tableau de bord. Le bandeau du graphique doit afficher **flux** (point
vert) — sinon voir § 2.4.

📖 **Ce qui se passe au boot** (dans l'ordre) :
1. **Migrations** de la base (aucune donnée n'est transformée sans backup dédié).
2. **Cold start du runtime** : pour chaque couple armé, le moteur est reconstruit et
   **rejoue l'historique** (SMC : ~700 bougies du TF ; KDJ : chauffe de 600 barres H1 ≈
   25 jours — sans elle, le moteur resterait muet 4 jours).
3. **Rattrapage des orphelines** : tout trade resté « Actif » alors que sa condition de
   sortie est atteinte (redémarrages) est clôturé au premier tick — log `🩹 Rattrapage`.
4. **Workers de données** : Bybit WS (cryptos) et EA MT5 (instruments du broker)
   reprennent les flux de bougies.
5. **Garde-fou d'alimentation** : tout asset armé trouvé inactif est réactivé — log
   `🩹 Asset X réactivé automatiquement`.

## 2.2 Les logs — la radio interne

🔧 **Workflow** : fichiers dans `data/logs/` — `backend.log` (le principal),
`tauri.log`, `news_collector.log`.

🖥️ **À l'écran / dans le fichier** — les lignes qui font foi :

| Ligne de log | Ce qu'elle prouve |
|---|---|
| `⚡ Runtime tick démarré` | le cœur temps réel tourne |
| `Runtime tick: XAUUSD M15 MT5 moteurs armés (SMC armé), … (replay 672 bougies, 3 signaux historiques)` | le couple est armé et initialisé |
| `Bybit WS: session prévue pour N actifs × M timeframes = K topics` | le flux crypto a de la matière |
| `Bybit WS … session fermée proprement après ~500 µs` **en boucle** | ⚠️ AUCUN actif à suivre — alimentation coupée (voir § 9) |
| `📍 Approche zone …` | une alerte de zone est partie |
| `🔔 Alerte prix …` | une alerte de prix a franchi son niveau |
| `🩹 Rattrapage …` | un trade orphelin a été réparé |
| `XAUUSD … réinscrit (paramètres modifiés — hot-reload)` | un réglage a été appliqué au moteur |

## 2.3 Arrêter

🔧 **Workflow** : fermer la fenêtre Tauri ou tuer le process. **Tout s'arrête**
(aucun redémarrage automatique) — c'est le comportement attendu.

> ⚠️ **Règle de changement** : un changement de **code** (front ou back) exige une
> **relance complète** de run.sh. Un changement de **paramètre** (réglages, armement)
> s'applique par hot-reload en ≤ 60 s — la relance n'est PAS nécessaire.

## 2.4 Vérifier que tout est vivant (30 secondes)

1. Bandeau d'un graphique : point vert **flux**.
2. Page **Données** : fraîcheur des sources — chaque actif doit avoir des bougies de
   moins de 2 h (moins de 2 minutes pour les actifs du périmètre armé).
3. Tableau de bord : les cartes STRATÉGIES affichent des capitaux et la carte
   positions à risque reflète les trades ouverts.

---

# CHAPITRE 3 — Le tableau de bord (page d'accueil)

> La page d'accueil est un **cockpit** : tout y est lecture, presque rien n'y est
> réglage. Les réglages vivent dans les pages stratégies (chapitre 4) et le panneau
> d'indicateurs (chapitre 5).

## 3.1 LE PANNEAU — bloc STRATÉGIES (4 colonnes-instruments)

🔧 **Workflow** : page d'accueil, bande horizontal gravée « STRATÉGIES ».

🖥️ **À l'écran** : quatre colonnes (SMC, Straddle, Rockets, KDJ), chacune avec :
capital actuel, Σ R, taux de réussite, compteur de positions, état (Officielle /
Observation), jauge, et un bouton Telegram par stratégie.

📖 **Lecture** :
- **Capital** : le $ composé de la stratégie (départ 1 000 $ par défaut). Chaque
  clôture y ajoute son profit/perte réel.
- **Σ R** : la voix « stratégie » (chapitre 1.4).
- **Le bouton Telegram** : active/désactive les messages **de cette stratégie** vers
  votre canal. Effet immédiat sur le prochain message.

⚡ **Réglages & conséquences** : le capital et le risque **ne se règlent pas ici** —
voir ⚙️ Paramètres de chaque stratégie (chapitre 4). Le toggle Telegram ne coupe jamais
les alertes de zones ni d'alertes prix (elles ont leurs propres interrupteurs).

## 3.2 La planche MÉTÉO DU MARCHÉ

🖥️ **À l'écran** : jauges linéaires + ticker — la température générale des marchés
(volatilité, tendance) calculée par le backend.

📖 **Lecture** : contexte global pour juger si l'activité des moteurs est cohérente
(pas de signaux SMC en marché mort = normal).

## 3.3 Les SESSIONS MONDIALES

🖥️ **À l'écran** : horloges des sessions (Asie, Londres, New York). Les moteurs SMC
sont sensibles aux kill-zones de session — une lecture utile avant de s'étonner
d'un silence ou d'une salve.

## 3.4 Le bandeau des 3 badges → modales

🔧 **Workflow** : cliquer chaque badge du bandeau supérieur.

🖥️ **À l'écran + lecture** :
1. **Calendrier** : les annonces économiques à venir (l'app recharge le calendrier
   au démarrage — lundi/jeudi : jours de forte densité d'annonces US).
2. **Créneaux** : les créneaux de volatilité statistiques connus.
3. **Macro** : la **fenêtre macro ±30 min** autour des annonces à impact High.

> ⚠️ **Fenêtre macro — conséquence importante** : pendant ±30 min autour d'une
> annonce High, la stratégie **SMC n'émet plus** (le straddle, lui, est conçu POUR ces
> moments et reste actif). Si SMC semble muet un après-midi de CPI/FOMC : c'est voulu.

## 3.5 Le pedestal COMMS

🖥️ **À l'écran** : fenêtres par source + toiles — le flux des communications
(dépêches, annonces) qui nourrit presse et calendrier.

## 3.6 Les blocs du bas

- **Rapport d'activité** : condensé du chapitre 7 (une carte par stratégie).
- **Positions à risque / neutralisées** : les trades ouverts proches de leur SL /
  les positions dont le risque a été neutralisé (BE).
- **Journal de bord** : vos notes par trade (📝 dans les tableaux) — le carnet
  personnel qui survit aux redémarrages.

📖 **Lecture des positions à risque** : distance au SL en R et en % — plus la
distance est petite, plus l'alerte visuelle est justifiée.

---

# CHAPITRE 4 — Les quatre stratégies

> Les pages stratégies partagent la même anatomie (composant StrategyShell) :
> **🟢 positions en cours** (haut) + **📜 historique des trades** (bas) + un bloc
> **moteur**. Les réglages vivent dans les modales ⚙️ de chaque page.

## 4.0 L'anatomie commune (une fois pour toutes)

### La table des positions en cours

🖥️ **À l'écran** : une ligne par trade **rempli et vivant** — asset, TF, direction,
score, lot, entrée, SL, TP, prix actuel, **R Latent**, **P/L Latent**, badge d'état,
conviction IA.

📖 **Lecture** :
- **Lot** : taille recalculée = capital de la stratégie × risque % / risque en pips.
- **R Latent** : la DISTANCE du prix à l'entrée, en multiples du risque — le palier
  atteint par le mouvement, pas ce qui est encaissable.
- **P/L Latent** : R latent × montant risqué, en $.
- **Badge** : « ⏳ En attente » = ordre posé non rempli (invisible sur les graphiques,
  hors stats) ; « 🟢 En cours » = position au marché.
- **Colonne IA** : la conviction LLM (0-100) notée à l'émission — voir § 7.4.

### La table d'historique

🖥️ **À l'écran** : trades **clôturés uniquement** — sortie, verdict (palier max),
$ réalisé, MFE des perdants, dates, durée. Tri par colonne.

📖 **Lecture** :
- **Palier max** : le meilleur niveau atteint avant la clôture — juge le placement
  des TP (« frôlé TP2 puis SL » se lit ici).
- **MFE** (amber, sur les SL) : l'excursion favorable maximale avant la perte —
  un SL avec MFE élevé signale un TP trop lointain, pas une mauvaise entrée.
- **$** : la voix résultat (ventes partielles comprises).

⚡ **Réglages & conséquences** : aucune action sur ces tables ne touche les moteurs —
elles sont des vues. Le tri et le journal 📝 sont locaux.

---

## 4.1 SMC v12 — la stratégie structurelle

### 4.1.1 La page SMC

🖥️ **À l'écran** : positions SMC en cours + historique + bloc moteur (état, couples
armés, télémetrie santé).

📖 **Spécificités SMC** : les verdicts possibles sont TP3 / TS (trailing stop) /
TP2+BE / TP1+BE / SL / BE / Expire. Un trade vit jusqu'à son verdict — jamais de
clôture forcée sur dégradation de zone (décision 26/08).

### 4.1.2 ⚙️ Paramètres — le registre

🔧 **Workflow** : page SMC → bouton Paramètres → « ⚙️ Paramètres — SMC (registre +
niveaux de profits) » → bloc du haut.

🖥️ **À l'écran** : **Capital alloué ($)** et **Risque par trade (%)**.

⚡ **Réglages & conséquences** :

| Paramètre | Effet | Délai | Ne change PAS |
|---|---|---|---|
| Capital alloué | base du calcul des lots pour les trades FUTURS ; le $ composé affiché suit le capital de la stratégie | immédiat (prochain signal) | les trades déjà ouverts, l'historique |
| Risque par trade (1-3 %) | part du capital risquée par trade | immédiat | le vécu |

> ⚠️ **Piège classique** : augmenter le risque ne change PAS le R des trades
> existants — seulement le $ des futurs. Le R est une distance, pas une mise.

### 4.1.3 ⚙️ Paramètres — les niveaux de profits

🔧 **Workflow** : même modale, bloc « niveaux ».

🖥️ **À l'écran** : TP1 (× R), TP2 (× R), TP3 — mode (Liquidité lointaine / R fixe),
R fixe / repli (× R), **Trailing stop après TP2** (toggle + distance × R), et les
**fractions** : Vente à TP1 / Vente à TP2 / Solde à TP3 (%).

📖 **Valeurs de production au 09/10** : TP1 0,6 · TP2 2,0 · TP3 lointaine ·
trailing **ON à 0,1 × R** · fractions **100 / 0 / 0** (configuration « tout-TP1 »
activée le 06/10 via l'advisory du laboratoire).

⚡ **Réglages & conséquences** :

| Paramètre | Effet sur les trades FUTURS | Délai |
|---|---|---|
| TP1 / TP2 (× R) | place les premiers objectifs — plus court = plus de TP touchés, moins de R par palier | hot-reload ≤ 60 s |
| TP3 mode | **lointaine** : cible la liquidité la plus lointaine (EQH/PDH…), repli croisé si absente ; **R fixe** : cible directe 3-10 R | idem |
| Trailing ON/OFF + k × R | après TP2, le stop suit l'extrême à k × R — k petit = sécurise tôt, coupe les mouvements longs | idem |
| Fractions f1/f2/f3 | répartition du lot aux paliers — **les 3 valeurs ensemble, somme = 100 %** (refus sinon) | idem |

> ⚠️ **Règle d'or du vécu** : un trade déjà ouvert garde SES fractions et SES niveaux.
> Changer 100/0/0 → 50/30/20 ne transformer aucun trade existant : seuls les nouveaux
> signaux naîtront avec. Le laboratoire (chapitre 6) est le seul endroit pour rejouer
> le passé avec d'autres valeurs.

> 💡 **Lecture de « 100/0/0 + trailing 0,1 »** : tout se vend à TP1, le solde… il n'y
> en a pas — profil « encaisser vite » choisi après mesure (+154 R sur 959 trades en
> étude). C'est un profil, pas une vérité éternelle : le labo permet de le remettre en
> question périodiquement.

### 4.1.4 🕐 Timeframes / Assets (l'armement SMC)

🔧 **Workflow** : page SMC → bouton Timeframes → cocher/décocher les cases
asset × TF (M1, M5, M15, M30, H1).

🖥️ **À l'écran** : la grille des couples ; l'état actuel (production 09/10 :
M5+M15+M30 armés sur 9 assets ; M1 et H1 désarmés).

⚡ **Réglages & conséquences** — l'armement est LE périmètre :

| Vous armez/désarmez | Ce qui suit (≤ 60 s) |
|---|---|
| un couple asset × TF | le moteur (signaux), les données (l'asset est garanti alimenté — garde-fou), le scanner SMC, les zones d'approche, le laboratoire |
| décocher | le couple disparaît du runtime, du scanner et du labo ; ses trades ouverts restent gérés jusqu'à leur clôture |

> ⚠️ **H1 désarmé** (décision 04/09) : le cocher réactive des moteurs H1 — coût de
> calcul et historique de signaux à reconsidérer. M1 : très bruyant, désarmé par
> défaut.

### 4.1.5 Réglages PAR ASSET (surcharge ⊕ défaut)

🔧 **Workflow** : modale ⚙️ Paramètres SMC → bloc « Réglage par asset » → choisir un
asset dans le sélecteur (ex. XAUUSD) → modifier les champs → Enregistrer. Badge
« surchargé » sur les champs concernés ; **Réinitialiser** revient au défaut global.

🖥️ **À l'écran** : chaque champ vide affiche le défaut global en placeholder grisé ;
seuls les champs remplis deviennent des surcharges.

📖 **Lecture** : XAUUSD peut avoir trailing 0,2 pendant que tous les autres assets
restent à 0,1 — la modale le montre par les badges.

⚡ **Réglages & conséquences** :

| Action | Effet | Délai |
|---|---|---|
| Surcharger un champ d'un asset | SEULS les couples de cet asset se réinscrivent (log hot-reload ciblé) ; les trades futurs de l'asset naissent avec | ≤ 60 s |
| Réinitialiser | l'asset revient au défaut global | ≤ 60 s |
| n'importe laquelle | rien sur les autres assets, rien sur le vécu | — |

> 💡 **Où voir l'état des surcharges** : la modale (badges) et le conseil 🤖 par asset
> (chapitre 7) qui cite chaque écart au défaut.

### 4.1.6 Définition & scanner

- **Page Définition SMC** : le lexique complet (BOS, MSS, CHoCH, order block, FVG,
  OTE, premium/discount, kill-zones…) — la référence vocabulaire.
- **Page Scanner SMC** : le vivier — une carte par asset armé, meilleur setup par TF,
  tendance H1/H4, whale 🐋. Le bandeau **📍 Zones à l'approche** y vit (chapitre 5.6).

---

## 4.2 Straddle — la stratégie événementielle

### 4.2.1 Comment elle travaille (à lire d'abord)

Le straddle ne regarde pas les graphiques : il attend des **annonces à impact High**
(PCE, GDP, FOMC…). À **T−10 s**, les DEUX jambes (achat ET vente) sont posées au même
prix E. Le premier mouvement ne « choisit » rien : les deux vivent en parallèle.
R = sl_mult × ATR H1. Verdicts possibles par passe : TP3 / TS / TP2+BE / TP1+BE /
SL / BE / Expire — **une passe peut coûter jusqu'à −1,5 R** (la jambe perdante paie
son SL), c'est assumé.

### 4.2.2 L'agenda et la matrice des événements

🔧 **Workflow** : page Straddle → agenda (annonces à venir) → matrice **81 cases** :
9 assets × 9 types d'événements, chaque case s'arme/se désarme indépendamment.

🖥️ **À l'écran** : l'agenda liste les annonces avec compte à rebours ; la matrice
montre l'état armé de chaque couple (asset, événement). Production 09/10 : TOUT ARMÉ.

⚡ **Réglages & conséquences** :

| Action | Effet | Délai |
|---|---|---|
| Armer/désarmer une case | des passes seront (ou non) posées pour CET asset sur CE type d'événement | immédiat (prochaine annonce) |
| Retirer un asset du périmètre | toutes ses cases cessent | idem |
| Modifier un réglage pendant une fenêtre active | **différé** : une passe dans sa fenêtre [T−30 min ; T+8 h] garde ses paramètres — la modif s'applique à la suivante | à la prochaine passe |

> ⚠️ **La garde de fenêtre** est une protection : on ne change pas les règles d'une
> passe en cours. Le log hot-replay le dit explicitement.

### 4.2.3 ⚙️ Paramètres straddle

🔧 **Workflow** : page Straddle → ⚙️ Paramètres.

| Paramètre | Effet | Délai |
|---|---|---|
| Placement (secondes avant T) | quand les jambes se posent (défaut 10 s) | prochaine passe |
| SL (× ATR H1) | l'échelle de risque de la passe | idem |
| Trailing (× R) | suivi après TP1 de la jambe gagnante | idem |
| **Surcharge par asset** | mêmes champs, pour un seul asset (ex. DAX plus nerveux) | idem, couple seul |

📖 **Lecture de la table des passes** : le R net d'une passe = somme des jambes avec
comptabilité « TP acquis » : un TP touché reste acquis même si la jambe revient.

---

## 4.3 Rockets — les cassures de consolidation

### 4.3.1 Comment elle travaille

Scanne D1 l'univers crypto (top 300 Binance) + actions US liquides. Classement /10 sur
4 piliers : Fondamental (3), Technique (3), Chartisme (2), Pilotage (2). **Alpha ≥ 9**,
**Rocket ≥ 7** ; les candidats ≥ 5 sont journalisés en attente de cassure de pivot.
Gestion : R1 touché → 50 % vendus puis trailing % ; **STAG** = sortie de stagnation ;
veto « unlocks » actif (les déverrouillages de tokens tuent la candidature).

### 4.3.2 Pages et réglages

🖥️ **À l'écran** : page Rockets (positions + historique avec verdicts 🏁 TS / ⏹ STAG /
👤 Manuel / ❌ SL), Scanner (vivier de candidats avec score par pilier), Définition.

⚡ **Réglages & conséquences** : profil de risque (PeuRisque/Neutre/Risque) et
trailing % dans ⚙️ Paramètres — effet sur les positions futures uniquement. **Pas de
réglage par asset** (choix de conception : la stratégie vit sur son univers, pas sur
des actifs individuels).

📖 **Lecture** : 9 clôtures seulement au 09/10 — tout est sous la règle des 30,
l'app le dit et n'en tire aucune conclusion. C'est le comportement attendu.

---

## 4.4 KDJ/Halftrend — la tendance H1

### 4.4.1 Comment elle travaille

À chaque clôture H1 : si SMA100 > EMA200 (contexte haussier), K croise D au-dessus,
close au-dessus de l'EMA200 et (optionnel) ADX suffisant, alors une flèche Halftrend
déclenche l'entrée à l'open suivant. **SL = EMA200 figée à l'entrée** ; **TP unique =
entrée + ratio_risk × distance à l'EMA200**. Sorties : TP, SL, ou **Retournement**
(signal inverse). Niveaux FIGÉS à l'émission — pas de BE, pas de trailing.

### 4.4.2 ⚙️ Paramètres

🔧 **Workflow** : page KDJ → ⚙️ Paramètres (period, signal, amplitude, ratio risque,
ADX min) + bloc de surcharge par asset (mêmes champs par asset).

| Paramètre | Effet | Délai |
|---|---|---|
| period / signal (défauts 20 / 7) | la vitesse du KDJ — plus court = plus de signaux, plus de bruit | hot-reload ≤ 60 s |
| amplitude (2) | la réactivité de Halftrend | idem |
| ratio risque (2) | le multiple du TP — 2 = cible à 2× la distance EMA200 | idem |
| ADX min (−1 = off) | filtre de tendance — monte le seuil pour ne trader que les tendances franches | idem |
| surcharge par asset | ex. adx_min spécifique à BNB | idem, couple seul |

### 4.4.3 L'armement KDJ

🔧 **Workflow** : modale « Choix des assets » — cocher les assets (production :
BNB, XAGUSD, XPTUSD, XRP, LINK, SOL).

> ⚠️ **Armer un asset KDJ garantit son alimentation en bougies** (garde-fou du 09/10).
> Avant cette protection, 5 des 6 assets étaient privés de données pendant 49 h sans
> qu'aucune alarme ne sonne.

### 4.4.4 Le scanner KDJ & la table d'historique spéciale

🖥️ **À l'écran** : le scanner liste les actifs avec ADX H1/D1, direction, K·D·J,
**Prêt** (toutes conditions réunies sauf la flèche). L'historique KDJ est une
**variante dédiée** : sans colonnes SL/TP/Stratégie, avec colonne **Verdict**
(✅ TP / ❌ SL / 🔄 Retournement).

📖 **Lecture** : les 3 premières clôtures de l'histoire (06-07/10) : XPTUSD TP +2,24 R,
BNB SL −0,91, SOL SL −1,15 — Σ +0,18 R. Tout effectif < 30 reste descriptif.

---

---

# CHAPITRE 5 — Graphiques & surveillance

> Page **Graphiques** (menu SMC → Graphiques). C'est la salle de lecture des marchés :
> bougies temps réel, tout l'outillage SMC, dessins, alertes — et les trades vivants.

## 5.1 Les cellules

🔧 **Workflow** : la page propose une grille de cellules (layout réglable). Chaque
cellule : cliquer pour l'activer (cadre cyan), choisir asset + TF dans son bandeau,
**double-clic** n'importe où = plein écran (**Échap** pour revenir).

🖥️ **À l'écran** : bandeau par cellule (asset, TF, prix, variation, voyant **flux**
vert = WebSocket connectée) ; bouton ⛶ plein écran.

📖 **Lecture** : prix et variation = dernière bougie en formation ; « silence » rouge =
flux coupé (voir § 9).

⚡ **Réglages** : aucun réglage moteur ici — changer de TF/asset est une vue. Les
sous-graphiques RSI/MACD/ATR n'apparaissent que sur les layouts ≤ 4 cellules (lisibilité).

## 5.2 Les indicateurs (le panneau)

🔧 **Workflow** : bouton indicateurs → cocher/décocher → **Appliquer**.

🖥️ **À l'écran** : ~30 cases réparties en familles : **base SMC** (tendance bgcolor,
structure HH/HL/LH/LL, BOS, MSS, CHoCH, sweeps, OB, FVG, signaux) ; **étendus**
(sessions Asie/Londres/NY, EQH/EQL, Asian HL, niveaux clés, NDOG/NWOG, breaker,
propulsion, imbalance, BPR, premium/discount, équilibre, volume fort, impulsions,
OB multi-TF H1/H4/W1/MN) ; **institutionnel** (bande OTE swing + ancres) ; **Kasper**
(tendance multi-TF, périodes rapide/lente) ; **RSI / MACD / ATR** (sous-graphes).

📖 **Lecture** : chaque famille dessine la couche correspondante du moteur — les OB
montrent force x/10 et état (vierge/touchée), les liquidités PDH/PDL leurs niveaux.

> ⚠️ **Principe fondamental** : les indicateurs sont de l'**AFFICHAGE**. Aucune case
> ne change le moteur, les signaux ou les alertes. Tout est reversible instantanément.

## 5.3 L'indicateur OB Institutionnels (OTE)

🖥️ **À l'écran** : bandes OTE swing par TF (seuil de vague propre à chaque TF) +
marquage **doré** des OB en confluence avec l'OTE.

📖 **Lecture** : rétro-ingénierie de « SMC Institutional Scalper » — parité vérifiée
avec TradingView. La confluence dorée est **informative uniquement** (étude C1 du 09/10 :
z = +1,61 < 2 → aucun filtre, aucun poids moteur).

## 5.4 Les dessins personnels

🔧 **Workflow** : barre d'outils bas-centre → outil (╱ tendance, ▭ rectangle, ƒ fibo,
⌫ gomme) → dessiner sur le graphique. 🗑 efface TOUT les dessins de l'asset.

🖥️ **À l'écran** : les dessins **survivent** aux changements de TF et aux redémarrages
(persistés par asset).

## 5.5 Les alertes de prix

🔧 **Workflow** : bouton 🔔 → mode pose → cliquer sur le graphique au niveau voulu →
choisir le sens (au-dessus / en-dessous). La cloche affiche le compteur ; rouvrir la
liste pour supprimer (🗑).

🖥️ **À l'écran + lecture** : chaque alerte = un niveau, un sens, une note. Le watcher
examine **chaque prix live** ; au franchissement : désarmage automatique + notification
app (son + notification OS) + message Telegram.

⚡ **Réglages & conséquences** : une alerte est **one-shot** (se désarme en touchant).
Effet immédiat. Aucun lien avec les moteurs.

## 5.6 Les zones d'approche SMC (le watcher 📍)

🔧 **Workflow** : rien à activer — le watcher tourne en continu sur les couples SMC
**armés**. Sa face visible : le bandeau « 📍 Zones à l'approche » en tête de la page
**Scanner SMC** (rafraîchi 30 s).

🖥️ **À l'écran** : le bandeau liste les zones **fraîches** (order blocks jamais
touchés) que le prix live approche — chip verte (▲ achat) ou rouge (▼ vente) avec
asset, TF, zone [bas–haut], **distance en × ATR** (ambre si ≤ 0,10 = imminence), 🔔 si
l'alerte est déjà partie. Ligne du bas : les alertes des 2 dernières heures.

📖 **Comment ça marche** : la zone la plus proche de chaque sens est guettée ; bande
d'approche = 0,25 × ATR **du TF du couple** ; **une alerte par zone et par épisode**
(ré-armement seulement si le prix recule de 1 × ATR supplémentaire) ; zone touchée =
fin de surveillance. Telegram reçoit « 📍 Approche zone ACHAT — XAUUSD M5 · [..] · à
0,12 × ATR ». Anti-flood 20 alertes/heure.

> ⚠️ **Nuit 23h–7h (heure de Paris)** : le watcher ne fait STRICTEMENT rien — aucun
> état, aucune notification. Au réveil 7h, seules les zones **encore** en bande
> alertent (pas de rattrapage bruyant).

⚡ **Réglages (clés de configuration, tick 60 s)** : `smc_alertes_zones_actif` (1/0),
`…_seuil_atr` (0,25), `…_hysteresis_atr` (1,0), `…_nuit_debut` (23), `…_nuit_fin` (7),
`…_telegram` (1/0). Désactiver n'affecte ni les alertes de prix ni les moteurs.

## 5.7 Les trades vivants sur les graphiques

🖥️ **À l'écran** : un trade **rempli et ouvert** = box TP (entry↔TP3, vert), box SL
(rouge, disparaît au BE), lignes TP1/TP2, entrée pointillée, label force/lot/niveaux.
Les trades des **autres TF** du même asset apparaissent avec un badge (M5, KDJ…).
**Tant qu'il est ouvert, le trade vit de sa bougie d'origine jusqu'à la bougie en
cours — décaler ou zoomer le graphique ne le fait jamais disparaître** (correctif
09/10). À la clôture, tout disparaît (fidélité Pine).

> ⚠️ **Les ordres SMC posés non remplis sont invisibles** (décision 11/09) : seuls les
> trades déclenchés se dessinent.

## 5.8 Le calendrier économique sur graphique

🖥️ **À l'écran** : les annonces à venir apparaissent sur l'axe temps ; survol =
tooltip (titre, heure, impact). Lien direct avec la fenêtre macro (§ 3.4).

---

# CHAPITRE 6 — Le laboratoire de simulation

> Page **Simulation** — badge jaune permanent : « SIMULATION — jamais les chiffres
> officiels ». Le labo **rejoue les trades réellement pris** avec d'autres réglages :
> il ne peut pas inventer des trades que la stratégie n'a pas pris.

## 6.1 Onglet SMC — rejouer une configuration

🔧 **Workflow** : onglet SMC → régler la grille de paramètres **virtuels** (TP1, TP2,
mode TP3, trailing, fractions f1/f2/f3) → **▶ Lancer la simulation** (~35 s) →
comparatif vécu officiel vs simulation.

🖥️ **À l'écran** : KPI côte à côte (R, $, WR, effectif) — la colonne « vécu » est
l'officielle, l'autre le contre-factuel.

⚡ **Conséquences** : **aucune** — tout est virtuel tant qu'on n'active pas (§ 6.4).
Les paramètres de la grille ne touchent pas la production.

## 6.2 Le périmètre de simulation (chips)

🔧 **Workflow** : sous la grille — chips **« Paires simulées »** (les assets armés ;
vide = toutes) et **« Timeframes simulés »** (M1/M5/M15/M30).

🖥️ **À l'écran** : cliquer une chip la sélectionne (teal). **Une seule chip asset** =
mode PAR ASSET — l'advisory (§ 6.4) passe sur le périmètre de cet asset.

📖 **Lecture** : le périmètre est **virtuel** — il ne touche pas à l'armement.

## 6.3 Les balayages

🔧 **Workflow** : **📊 Balayer les fractions** (toutes les combinaisons f1/f2/f2,
quelques minutes) ou **balayage du trailing** (k de 0,05 à 1,0, ~3 min — re-jeu exact).

🖥️ **À l'écran** : le tableau des configurations testées, trié par R total, avec la
ligne « moteur actuel » pour comparer. Chaque balayage devient un **essai** stocké.

## 6.4 L'advisory honnête 🎯

🔧 **Workflow** : carte « 🎯 Recommandation » en haut du labo → lit les essais →
**⚡ Activer cette config** (clic direct, pas de confirmation) quand un essai battant
existe.

🖥️ **À l'écran** : le meilleur essai (R total maximal, **effectif ≥ 30 — règle des 30,
par asset en mode PAR ASSET**), l'essai correspondant à la configuration actuelle,
l'écart en R ; ou honnêtement « Effectif insuffisant — les données s'accumulent ».

📖 **Lecture** : « activable » signifie que les paramètres de l'essai existent dans
les réglages réels. L'espace straddle est virtuel : info seule, pas de bouton.

⚡ **Conséquences de ⚡ Activer** — le geste le plus puissant de l'app :

| Mode | Ce qui est écrit | Effet |
|---|---|---|
| global (aucune chip) | la **configuration globale** de la stratégie | tous les assets, hot-reload ≤ 60 s |
| **PAR ASSET** (1 chip) | la **SURCHARGE de cet asset** | seuls ses couples se réinscrivent ; les autres assets ne bougent pas |

Dans les deux cas : écriture **relue et vérifiée** après sauvegarde (une écriture non
confirmée est signalée), et **aucun effet sur les trades ouverts** (vécu immuable).

## 6.5 Onglets straddle et KDJ

- **Straddle** : re-jeu des passes vécues avec d'autres sl/trailing/time-stop —
  périmètre chips des passes par asset ; advisory en lecture seule (espace virtuel).
- **KDJ** : re-jeu H1 par assets avec period/signal/amplitude/adx, classement par
  actif (r_net) — même advisory PAR ASSET avec activation en surcharge.

---

# CHAPITRE 7 — Analyses & IA

## 7.1 Le rapport d'activité (page Analyses)

🔧 **Workflow** : page **Analyses** — vue d'ensemble ou onglet par stratégie (toutes
les pages « /analyse » des stratégies y redirigent — c'est LA page d'analyse).

🖥️ **À l'écran** : en-tête (effectif, source, badge ⚠️ sous la règle des 30) →
**bloc verdict « la réponse en 5 s »** (KPI de tête : capital, Σ R distance, WR, hier)
→ classements → performance par période (jour/semaine/mois) → détail verdicts & assets
→ timeframes par asset → tranches de score (SMC) → **heatmap heure × jour** →
évolution jour après jour (snapshots + avis IA archivés).

📖 **Lecture transversale** :
- le tri des classements = **Σ R distance** (la contribution réelle, volume compris) ;
- gris = effectif < 30 (non significatif) ;
- la heatmap répond à « quand le $ se gagne-t-il ? » — les créneaux horaires locaux.

## 7.2 Les classements → conseil 🤖 par asset

🔧 **Workflow** : dans l'onglet d'une stratégie, bloc « 🏆 Classement des assets » →
bouton **🤖** en fin de ligne d'un asset → la modale de conseil se génère (~1 min).

🖥️ **À l'écran** : badge **JUGEABLE · N clôtures** (ou NON JUGEABLE · N/30), badge
confiance /100, **état de l'asset** (2-3 phrases), **conseil proposé** (« tu décides
seul »), **chiffres clés** — chaque chiffre suivi de SA phrase d'explication (fabriqués
par le moteur, pas par le LLM) — et le chemin d'application (Simulation → chip →
balayer → ⚡).

📖 **Ce que l'IA a lu** pour conseiller : le vécu de l'asset (R distance, $, par TF),
ses réglages actuels (surcharge ou défaut — chaque écart cité), l'écart mesuré au
balayage du labo. Cache du jour — redemander le lendemain ou après un nouveau
balayage. **Rockets exclu** (pas de réglages par asset).

⚡ **Conséquences : AUCUNE** — la modale est de la lecture pure. L'application d'un
conseil passe toujours par le labo (garde-fous de la § 6.4).

## 7.3 La conviction LLM (colonne IA)

📖 **Lecture** : à CHAQUE émission de signal, le LLM local note sa conviction a priori
(0-100, étalonnage strict — survol pour la raison en une phrase). Étude du 24/09 :
une conviction basse prédit les SL, mais l'AUC (0,572) est sous le seuil (0,65) →
**pas de filtre** : la note informe, ne bloque rien. Re-test protocolaire prévu à
~500 signaux notés.

## 7.4 L'analyse IA du rapport

🔧 **Workflow** : bas de l'onglet stratégie → « ⚡ Générer l'avis » (~1 min, Ollama
local) → « ↻ Régénérer ».

🖥️ **À l'écran** : état général, ce qui marche, ce qui coince, pistes à étudier,
confiance. Cache du jour ; l'avis du jour est archivé avec le snapshot quotidien
(l'historique survit aux redémarrages).

## 7.5 La page IA (ML + prompts)

🔧 **Workflow** : page **IA** → onglets **ML** et **Prompts**.

🖥️ **À l'écran + lecture** :
- **ML** : importance des features par permutation OOS (« ce qui distingue les
  gagnants » — nourrit l'analyste IA), monitoring par stratégie, panneau de retrain
  fine-tuning (lance un réentraînement local — ne touche JAMAIS les moteurs de
  décision, seulement les couches d'analyse).
- **Prompts** : la liste des prompts **éditables** — `conviction_signal`,
  `analyse_rapport`, `analyse_rapport_asset`, définitions des stratégies, catalyseur
  rockets… Chaque édition s'applique au prochain appel ; le bouton de repli restaure
  le défaut du code.

> ⚠️ **Modifier un prompt change le tonneau d'où l'IA tire** — pas les chiffres : le
> contexte chiffré est TOUJOURS fabriqué par les moteurs. Une consigne trop laxiste
> donnera des avis verbeux, jamais des chiffres inventés (et les chiffres clés par
> asset sont produits côté moteur, hors LLM).

---

# CHAPITRE 8 — Presse & calendrier économique

## 8.1 La revue de presse

🔧 **Workflow** : page **Presse** → filtres (source, langue) → cartes articles ;
**+ Ajouter** une source (flux RSS).

🖥️ **À l'écran** : un **brief** en tête (contexte marché rédigé par l'IA locale +
résumé des articles), puis les cartes enrichies — barre colorée = score d'impact
(rouge ≥ 60, jaune ≥ 40), traduction française à la demande, lien vers la source.

📖 **Lecture** : le score pondère la matière première des couches IA (sentiment,
catalyseur rockets). Les briefs sont mis en cache — rafraîchissement périodique.

## 8.2 Contexte marché & sentiment

🖥️ **À l'écran** : bloc de synthèse du sentiment de veille — l'humeur des marchés
vue par l'IA à partir des dépêches notées. Contexte, pas signal.

## 8.3 Calendrier & radar

🖥️ **À l'écran** (aussi sur le dashboard, badge → modale) : « 📅 Calendrier
économique — 10 prochains jours » (annonces avec impact), « ⏰ Créneaux moyens de
volatilité » et « 🌡️ Radar ATR temps réel ». C'est ici que se préparent les fenêtres
macro (SMC muet ±30 min autour des High) et les passes straddle à venir.

---

# CHAPITRE 9 — Données & santé

## 9.1 Les workers d'ingestion

🔧 **Workflow** : page **Données** → section « Workers d'ingestion » → interrupteurs
+ statuts par worker.

🖥️ **À l'écran** : cartes par worker (Bybit WS cryptos, EA MT5 instruments du broker,
collector de news) avec état, dernier cycle, compteur.

⚡ **Réglages & conséquences** :

| Action | Effet | Délai |
|---|---|---|
| Couper un worker | les bougies de ses assets cessent d'arriver — moteurs correspondants **aveugles** (plus de signaux NI de clôtures) | ≤ 60 s |
| Rallumer | le flux reprend ; **le trou reste** dans l'historique (les moteurs reprennent sur les nouvelles barres) | ≤ 60 s |

> ⚠️ **Leçon de l'incident du 07/10** : 15 assets privés de données pendant 49 h =
   3 trades jamais clôturés + 5 assets KDJ muets. **Couper l'alimentation coupe
   TOUT.** Symptôme dans les logs : « session fermée proprement après ~500 µs » en
   boucle.

## 9.2 La gestion des assets

🔧 **Workflow** : section assets → cocher/décocher → « + Ajouter un asset » (modale :
symbole, type, source MT5/Binance, paramètres pip/point).

🖥️ **À l'écran** : « N / total activés ». Chaque case = le drapeau d'alimentation.

⚡ **Réglages & conséquences** :
- **Décocher** un asset l'exclut des workers à leur prochaine session (≤ 60 s) ;
  l'historique est conservé.
- **Sauf s'il est armé** dans une stratégie (SMC/KDJ/straddle) : le **garde-fou le
  réactive automatiquement** dans la minute (log 🩹) — armé ⇒ alimenté, par n'importe
  quel chemin. C'est voulu : on ne peut pas désamorcer une stratégie en retirant
  ses données par accident.
- Ajouter un asset MT5 : il apparaîtra dans la liste de l'EA au prochain cycle —
  si rien n'arrive, vérifier que le symbole existe côté MT5 (recompilation EA si
  besoin).

## 9.3 La santé des sources

🖥️ **À l'écran** : fraîcheur par actif/source (dernière bougie). Réflexe de
diagnostic : tout asset du périmètre armé doit avoir des bougies < 2 min (barres en
formation). Le détail complet : § 2.4.

## 9.4 Les paramètres par asset (pips/points)

🔧 **Workflow** : panneau paramètres d'asset → taille du pip, valeur du point, etc.

⚡ **Conséquences** : ces valeurs servent au **calcul des lots** et à l'affichage
« points MT5 » — une erreur ici fausse la taille des positions futures de CET asset
(le R en multiple du risque reste juste ; le $ bouge). Effet au prochain signal.

---

# CHAPITRE 10 — Réglages transversaux : la carte complète

## 10.1 Les trois niveaux de réglage (hiérarchie)

```
NIVEAU 1 — DÉFAUT GLOBAL par stratégie   (modales ⚙️, clés smc_* / kdj_params / strategies_params)
NIVEAU 2 — SURCHARGE PAR ASSET           (bloc « Réglage par asset » — prime champ à champ)
NIVEAU 3 — LE VÉCU                       (FIGÉ : niveaux à l'émission, fractions à la clôture)
```

Toutes les lectures de l'app (moteurs, labo, advisory, IA, affichage) passent par la
**fusion Niveau 1 ⊕ Niveau 2** — une seule source de calcul. Le Niveau 3 ne bouge
jamais, quoi que vous fassiez.

## 10.2 La table maîtresse des réglages

| Réglage | Où | Effet | Délai |
|---|---|---|---|
| Capital / risque % | ⚙️ Registre (×4 stratégies) | lots et $ des trades futurs | immédiat |
| TP1/TP2/TP3/trailing/fractions SMC | ⚙️ Niveaux | profil de sortie des futurs signaux | hot-reload ≤ 60 s |
| Surcharge par asset (SMC/straddle/KDJ) | bloc par asset | SEUL l'asset concerné | hot-reload couple ≤ 60 s |
| Couples SMC armés | 🕐 Timeframes/Assets | périmètre moteurs + données + scanner + labo | ≤ 60 s |
| Assets KDJ armés | modale KDJ | idem, périmètre H1 | ≤ 60 s |
| Périmètre straddle | modale straddle | passes futures | ≤ 60 s |
| Matrice 81 cases | page straddle | par (asset × événement) | immédiat (prochaine annonce) |
| Alertes zones (6 clés) | configuration | watcher d'approche | ≤ 60 s |
| Alertes prix | sur graphique | one-shot par niveau | immédiat |
| Telegram par stratégie | dashboard | messages de la stratégie | immédiat |
| Workers | page Données | alimentation en bougies | ≤ 60 s |
| Indicateurs graphiques | panneau indicateurs | **affichage seul** | immédiat |
| Prompts IA | page IA › Prompts | ton des analyses/notations | prochain appel |
| ML retrain | page IA | couches d'analyse seulement | — |

> ⚠️ **Ce qui n'agit JAMAIS par réglage** : le vécu (trades passés et ouverts), le
> R distance des trades existants, l'historique des clôtures, les snapshots.

## 10.3 Armement = périmètre (le graphe des conséquences)

Armer un couple asset × TF détermine D'UN SEUL geste : les moteurs qui tournent, les
données garanties (garde-fou), le scanner, les zones d'approche, les chips du labo,
et la matière des classements. Le désarmer retire tout cela — mais jamais les trades
ouverts, qui vivent jusqu'à leur verdict.

---

# CHAPITRE 11 — Annexes

## 11.1 Glossaire express

| Terme | Sens |
|---|---|
| **Fill** | le prix a touché l'entrée : le trade existe au marché |
| **R distance** | multiple du risque parcouru — juge entrées + TP |
| **$ composé** | résultat réel réinvesti trade après trade |
| **BE (break-even)** | stop ramené à l'entrée (après TP1) |
| **Trailing (k × R)** | stop suivant l'extrême à distance k |
| **MFE** | excursion favorable maximale (avant un SL) |
| **OB** | order block — zone institutionnelle d'origine du mouvement |
| **OTE** | zone de retracement optimal (0,62-0,79 du dernier pied) |
| **FVG / BPR / Imbalance** | déséquilibres de prix à combler |
| **BOS / MSS / CHoCH** | cassures de structure (suite / déplacement / changement) |
| **Sweep** | mèche qui prend la liquidité avant de repartir |
| **EQH/EQL, PDH/PDL** | plus hauts/bas égaux, précédents du jour/semaine |
| **Kill zone** | fenêtre de session à forte activité institutionnelle |
| **Passe** | un événement straddle complet (deux jambes) |
| **STAG** | clôture rockets pour stagnation |
| **Vécu** | l'historique officiel figé d'une stratégie |
| **Essai** | une configuration mesurée au labo |
| **Advisory** | la recommandation honnête sur les essais (règle des 30) |
| **Surcharge** | réglage propre à un asset, prime sur le défaut |
| **Hot-reload** | réinscription automatique des moteurs après changement |

## 11.2 LA CHECKLIST DE TEST INTÉGRAL (~45 min)

> À dérouler dans l'ordre après une relance. Chaque point se coche si l'écran
> correspond au « 🖥️ À l'écran » du chapitre cité.

**A. Boot (§ 2)** — ☐ run.sh démarre sans erreur ☐ fenêtre + « flux » vert ☐ logs :
moteurs armés par couple ☐ aucune boucle « session fermée 500 µs » ☐ fraîcheur
bougies < 2 min sur les assets armés (§ 9.3)

**B. Dashboard (§ 3)** — ☐ 4 colonnes STRATÉGIES avec capitaux ☐ météo animée ☐
horloges sessions ☐ 3 badges → 3 modales s'ouvrent ☐ fenêtre macro visible si
annonce High à venir ☐ positions à risque cohérentes avec la page stratégie

**C. Stratégies (§ 4)** — ☐ SMC : positions + historique peuplés ☐ ⚙️ registre et
niveaux affichent les valeurs en vigueur ☐ surcharge XAUUSD visible (badge) ☐ 🕐
grille d'armement = état réel ☐ straddle : agenda + matrice 81 cases ☐ rockets :
scanner vivier ☐ KDJ : scanner ADX + historique variante (colonne Verdict, pas de
SL/TP/Stratégie) ☐ verdicts bien libellés (✅ TP, ❌ SL — jamais « En cours » sur un
trade fermé)

**D. Graphiques & surveillance (§ 5)** — ☐ cellules actives, plein écran Échap ☐
indicateurs s'appliquent (tester OB + structure) ☐ un dessin persiste après changement
de TF ☐ alerte prix posée au clic apparaît dans la liste ☐ bandeau 📍 Zones à
l'approche présent sur le Scanner (vide si rien en bande — normal) ☐ trade ouvert
visible ET restant après décalage du graphique ☐ multi-TF : le même trade apparaît
avec badge sur un autre TF

**E. Laboratoire (§ 6)** — ☐ onglets SMC/straddle/KDJ ☐ chips Paires = les assets
armés ☐ lancer une simulation (~35 s) → comparatif s'affiche ☐ balayage fractions →
table + essais ☐ advisory : meilleur essai ou « effectif insuffisant » honnête ☐
tester ⚡ Activer en PAR ASSET sur un asset secondaire → badge surchargé dans la
modale + log hot-reload CIBLÉ sur cet asset seul → Réinitialiser → retour au défaut

**F. Analyses & IA (§ 7)** — ☐ rapport par stratégie (verdict 5 s, classements,
heatmap) ☐ gris = effectif < 30 ☐ 🤖 sur un asset → modale complète avec chiffres
expliqués ☐ Générer l'avis (~1 min) ☐ page IA : ML + prompts éditables

**G. Presse & données (§ 8-9)** — ☐ presse : brief + cartes notées ☐ calendrier
10 jours ☐ Données : workers actifs, cases assets = état réel ☐ décocher un asset
NON armé → exclu ≤ 60 s ☐ décocher un asset ARMÉ → réactivé 🩹 dans la minute
(garde-fou)

**H. Synchronisation des changements (§ 10)** — ☐ modifier trailing SMC global →
log de réinscription de TOUS les couples ≤ 60 s ☐ modifier surcharge 1 asset →
log pour SES couples seuls ☐ redémarrer → tous les réglages conservés ☐ un trade
fermé avant modification garde ses niveaux d'origine dans l'historique

## 11.3 Dépannage — symptômes réels et remèdes

| Symptôme | Cause probable | Remède |
|---|---|---|
| « session fermée proprement après ~500 µs » en boucle | worker sans aucun actif (alimentation coupée) | § 9.2 — réactiver les assets ; le garde-fou couvre les armés |
| Un trade reste « En cours » alors que SL/TP est touché | moteur reconstruit sans la position (redémarrage) + rattrapage exclu | le rattrapage agit au tick suivant ; sinon vérifier que l'asset est alimenté |
| Un trade ouvert disparaît quand on décale le graphique | (corrigé 09/10) | signalez-le — ne devrait plus jamais survenir |
| Chips « Paires simulées » vides ou absurdes | chargeur cassé ou aucun couple armé | vérifier l'armement 🕐 |
| Asset MT5 muet alors qu'il est activé | symbole absent de la liste EA | recompiler/mettre à jour l'EA (v1.35+) |
| « silence » rouge sur un graphique | WS coupée | § 2.4 puis § 9 |
| Avis IA indisponible | serveur Ollama local éteint | démarrer Ollama, puis Régénérer |
| R et $ « ne collent pas » | c'est la conception (deux voix, § 1.4) | lire l'écart comme la conversion |

## 11.4 Feuille de route du manuel

- 🔊 **À intégrer** : page « 📘 Manuel » dans l'app (lecteur du présent fichier,
  lecture seule) — décision propriétaire du 09/10, chantier séparé à venir.
- Maintenir ce fichier à chaque nouveau chantier (une section par fonctionnalité
  livrée) — il est le miroir testable de l'app.

> **— FIN DU MANUEL (P1 + P2 + P3) —**
