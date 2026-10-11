# ✅ FEUILLE DE ROUTE — Vérification intégrale de l'app (10/10/2026)

> **Méthode** : dérouler dans l'ordre (c'est l'ordre naturel d'utilisation : du démarrage
> à l'extinction). Cocher uniquement si le comportement correspond à l'attendu. Pour
> CHAQUE écart : **décrire avant de corriger** (constat → cause → correctif proposé →
> validation), puis reporter dans le tableau de synthèse en fin de feuille.
> **Référence** : le détail de chaque attendu vit dans le manuel
> (`docs/MANUEL-UTILISATEUR.md`) — la section est citée entre crochets [§ x.y].
> **Durée estimée totale : ~1 h à 1 h 15.**

---

## 0. Préparation (2 min)

- [x] Dépôt propre : `git status` sans surprise, HEAD = `1927852`
- [x] Lire `data/logs/backend.log` : noter l'heure du dernier évènement (base de comparaison)
- [x] Téléphone à portée de main (Telegram) — canal de réception identifié

## 1. Démarrage [§ 2]

- [x] `./scripts/run.sh` démarre sans erreur (build + api + fenêtre)
- [x] Fenêtre ouverte en < 3 min ; page d'accueil affichée
- [x] Logs : `⚡ Runtime tick démarré` présent
- [x] Logs : couples armés — 53 lignes moteurs (9 assets SMC × 3 TF + couples M1/H1)
- [x] Logs : session SPOT saine (11 actifs × 8 TF = 88 topics) — la boucle µs ne concerne que « linear » (écart 4, bruit bénin)
- [x] Logs : `Bybit WS: session prévue pour N actifs × M timeframes` (flux crypto vivant)
- [x] Aucun log `🩹 Rattrapage` inattendu au boot

## 2. Tableau de bord [§ 3]

- [x] Bloc STRATÉGIES : 4 colonnes avec capital, Σ R, WR, compteur de positions, état
- [x] Les 4 capitaux correspondent aux valeurs d'hier (vécu immuable — pas de saut)
- [x] Planche MÉTÉO animée (jauges/ticker)
- [x] Horloges SESSIONS MONDIALES à l'heure
- [x] Bandeau : les 3 badges cliquables → 3 modales s'ouvrent (calendrier vide ce WE — voir écart 3, affiné)
- [x] Modale calendrier : vide ce samedi — NORMAL (ForexFactory publie « nextweek » plus tard ; worker retry 3 h — écart 3 = message 503 trompeur, correctif proposé)
- [x] Fenêtre macro : silencieuse (aucune annonce High à venir — normal un samedi)
- [x] Pedestal COMMS : fenêtres par source vivantes
- [x] Cartes positions à risque / neutralisées cohérentes (2 KDJ LINK/SOL visibles)
- [x] Toggle Telegram d'une stratégie : réagit au clic (état conservé après rechargement)

## 3. Stratégie SMC [§ 4.1]

- [x] Page SMC : positions en cours + historique + bloc moteur affichés
- [x] Table en cours : colonnes R Latent / P/L Latent alimentées (prix vivants)
- [x] Historique : tri par colonne fonctionnel ; paliers max et $ présents
- [x] Aucun trade fermé n'affiche « ⏳ En cours » (vocabulaire verdicts — régression du 09/10)
- [x] ⚙️ Paramètres : registre (capital 1000, risque 1 %) et niveaux (TP1 0,6 · TP2 2,0 · TP3 lointaine · trailing ON 0,1 · fractions 100/0/0)
- [x] Bloc « Réglage par asset » : sélectionner un asset → placeholder grisés (aucune surcharge en base au 10/10 — les tester en parcours 8/H)
- [x] 🕐 Timeframes/Assets : grille conforme (9 assets × M5/M15/M30 ; M1 et H1 désarmés)
- [x] Page Définition SMC : lexique complet charge
- [x] Page Scanner SMC : bandeau 📌 présent bandeau 📍 Zones à l'approche présent + cartes du vivier

## 4. Stratégie Straddle [§ 4.2]

- [x] Page straddle : agenda (épuré — matrice passée dans la modale) des annonces + table des passes + bloc moteur
- [x] Matrice : REMPLACÉE par la matrice croisée unique 19 lignes (écarts 14-15) — validée owner, case cliquable (armer/désarmer une case test puis la remettre)
- [x] ⚙️ Paramètres : placement 3 s · SL 0,5 × ATR · trailing 1,0 (réglage owner confirmé), SL × ATR, trailing — valeurs conformes
- [x] Surcharge par asset straddle : sélecteur opérationnel : sélecteur + badges opérationnels
- [x] Page Définition straddle : charge : charge

## 5. Stratégie Rockets [§ 4.3]

- [x] Page rockets : positions + historique (verdicts ✓) + historique (verdicts 🏁/⏹/👤/❌ correctement libellés)
- [x] Scanner : vivier de candidats de candidats avec scores par pilier
- [x] Mention règle des 30 honnête (9 clôtures → rien de significatif affiché comme tel)
- [x] Page Définition rockets : charge : charge

## 6. Stratégie KDJ [§ 4.4]

- [x] Page KDJ : 2 positions en cours (LINK/SOL, 09/10 22h00) + historique
- [x] Historique = variante KDJ : sans SL/TP1-3/Stratégie ni Score, verdict lisible dans Palier max (✅ TP / ❌ SL), colonnes Gain/Perte + Évolution du capital, colonne Position (unités) — écarts 5-7 validés owner
- [x] XPTUSD fermée TP (+2,24 R) — Σ R ≈ +0,18
- [x] ⚙️ Paramètres : period 20, signal 7, amplitude 2, ratio 2, ADM min off
- [x] Modale assets KDJ : 6 cochés (BNB, XAGUSD, XPTUSD, XRP, LINK, SOL)
- [x] Scanner KDJ : ADX H1/D1, direction, Prêt — lignes présentes (XAGUSD/XPTUSD figés : samedi = normal)
- [x] Page Définition KDJ : charge

## 7. Graphiques & surveillance [§ 5]

- [x] [validé § 7]
- [x] Changer asset/TF d'une cellule : bougies se chargent, voyant **flux** vert
- [x] Plein écran (double-clic) puis Échap puis Échap : fonctionnels
- [x] Panneau indicateurs : OB + structure + tendance — OK (owner 11/10) activer OB + structure + tendance → Appliquer → dessins corrects
- [x] Indicateur OB Institutionnels : bandes OTE visibles, confluence dorée marquée
- [x] Dessin : tracer (lignes, rectangles, fibo) → persiste après changement de TF → changer de TF → le dessin persiste
- [x] Alerte prix : 🔔 pose au clic → liste → suppression pose au clic → liste avec l'alerte → supprimer
- [x] Trade ouvert visible sur le graphique de son TF : visible sur le graphique de son TF
- [x] **Décaler le graphique** : le trade RESTE visible — correctif du 09/10 VALIDÉ en production (espace entre dernière bougie et axe) : le trade RESTE visible jusqu'à la bougie en cours — **régression du 09/10, point critique**
- [x] Multi-TF : même trade visible avec badge sur un autre TF le même trade apparaît sur un autre TF du même asset avec badge
- [x] [reporté lundi — marchés fermés dimanche]
- [x] Test nuit — REPORTÉ lundi (marchés fermés dimanche) : régler temporairement `smc_alertes_zones_nuit_debut/fin` sur l'heure courante → silence total → remettre 23/7
- [x] Éco-cal sur graphique : annonces + tooltips : annonces visibles, tooltip au survol

## 8. Laboratoire [§ 6]

- [x] Page Simulation : badge jaune « jamais les chiffres officiels »
- [x] Accès labos : sans onglets (mode verrouillé) via carte labo du dashboard — voulu owner fonctionnels
- [x] Chips « Paires simulées » : exactement les 9 assets armés (régression 17/09) (régression du 17/09)
- [x] Chips « Timeframes simulés » : M1/M5/M15/M30 : M1/M5/M15/M30
- [x] ▶ Lancer une simulation (~35 s) → comparatif (~35 s) → comparatif vécu/sim s'affiche
- [x] 📊 Balayer les fractions → table + essais (~qq min) → table + essais créés
- [x] Advisory 🎯 : meilleur essai / effectif insuffisant honnête meilleur essai OU « effectif insuffisant » (honnêteté)
- [x] Badge PAR ASSET : s'affiche à 1 chip — L'ADVISARY NÉCESSITE des essais TAGUÉS [asset] (balayage lancé avec la seule chip) — expliqué owner, essais en base globaux à l'essai sur l'advisory
- [x] ⚡ Activer PAR ASSET : expliqué owner (écrit la SURCHARGE de l'asset seul) — test ⚡ exécuté par IA via API → modale ⚙️ : badge surchargé sur cet asset SEUL
- [x] [SIMULÉ IA 11/10] Surcharge BTC trailing 0,15 → 45 s après : EXACTEMENT BTC M5/M15/M30 réinscrits, aucun autre
- [x] [SIMULÉ IA 11/10] DELETE → 40 s après : mêmes 3 couples BTC réinscrits (retour défaut), table surcharges vide → retour au défaut + log de réinscription
- [x] [SIMULÉ IA 11/10] Aucune autre ligne de réinscription dans le log — uniquement BTC

## 9. Analyses & IA [§ 7 + refonte 11/10 — écarts 20-21]

**Vue d'ensemble (page /analyses par défaut — refonte 11/10)**

- [x] « Les stratégies » : liste DYNAMIQUE — les 4 présentes (📐 ⚡ 🚀 📈), plus de « trois » figé
- [x] Ligne **TOTALES** : clôtures cumulées · WR pondéré · Σ R total · capital total (départ → actuel) · hier
- [x] Clic sur une ligne du tableau → rapport DÉTAILLÉ de la stratégie
- [x] Plus d'onglets : bandeau = titre + bouton « ← Vue d'ensemble » quand on est en détail
- [x] Accès direct ?strategie=X depuis la carte « Rapport d'activité » de chaque stratégie du dashboard
- [x] 🤖 **Synthèse IA du portefeuille** : ⚡ Générer (~1 min) → état + ce qui porte / ce qui coince / où concentrer l'attention + confiance + cache du jour + ↻ Régénérer
- [x] L'analyste cite des chiffres réels (stratégies, $, R) et signale les non-jugeables (règle des 30)

**Rapport par stratégie (refonte 3 colonnes — écart 21)**

- [x] ① Verdict 5 s en pleine largeur : capital, Σ R distance, WR, hier + badge < 30
- [x] Colonne GAUCHE : période — doublon de titre corrigé (tabs Jour/Semaine/Mois + graphique) — visible sans déplier
- [x] Colonne CENTRE : classements ✓ · edge ✓ · TF corrigés (positifs seuls, SMC seulement) (TF ou événements + assets avec 🤖) + ⚖️ Où vit l'edge (verdicts + assets barres) + ⏱️ Timeframes par asset (SMC)
- [x] Colonne DROITE : heatmap + snapshots — doublons de titres corrigés heure × jour + 📈 Snapshots (courbe + avis archivés)
- [x] 🎯 Tranches de score : SMC seulement (straddle/rockets/KDJ exclus) — tableau restauré (3 tranches servies : 6-8, 9-11, 12-19)
- [x] 🤖 Analyse IA toujours visible sous la grille : ⚡/↻ + confiance + les 4 blocs (état / forts / faibles / pistes)
- [x] Aucun bloc replié ne reste : tout est visible d'emblée
- [x] Straddle : timeframes/score exclus (v-if SMC), pas de timeframes/score — le squelette tient
- [ ] KDJ : le squelette tient (classements + edge + heatmap)
- [x] Rockets : timeframes/score exclus (v-if SMC) (peu de données)
- [ ] 🤖 Conseil par asset (clic sur une ligne du classement assets) : badges JUGEABLE/confiance + chiffres expliqués un à un + cache du jour

**Dashboard**

- [ ] Cartes stratégies : blocs Paramètres/Labo/Assets compactés sur UNE ligne chacun (« 1 092 $ · 1 % », « +109 R · 44 % »)
- [x] NOUVELLE carte « Rapport d'activité » sous les 3 blocs : « N cl. · ±X $ · hier ±Y $ » → ouvre LE rapport de la stratégie
- [x] L'ancienne carte « Rapport d'activité » (pedestal COMMUNICATIONS) ouvre bien la VUE D'ENSEMBLE

**PAGE IA — POINT COMPLET SUR LES PROMPTS (owner 11/10)**

- [ ] Onglet ML : features par permutation + monitoring par stratégie + retrain — visibles
- [ ] Onglet Prompts : liste complète éditable — vérifier la présence de TOUS :
  - [ ] `analyse_rapport` (avis par stratégie)
  - [ ] `analyse_rapport_asset` (conseil 🤖 par asset)
  - [ ] `analyse_creneaux` (synthèse matrice straddle)
  - [ ] `analyse_portefeuille` (synthèse globale — NOUVEAU 11/10)
  - [ ] `conviction_signal` (notation à l'émission)
  - [ ] définitions (`smc_definition`, `straddle_definition`, `rockets_definition`)
  - [ ] `rockets_catalyseur` + `rockets_ranker` + `unlock_detection` (présents en code — vérifier visibles et éditables)
- [ ] Pour CHAQUE prompt : ouvrir → texte complet lisible → annuler sans sauvegarder (aucun changement accidentel)
- [ ] Tester un prompt édité : modifier → l'analyse suivante reflète le changement → rétablir le défaut
- [ ] Revue de fond (owner) : chaque prompt fait-il son job ? à affiner ?
  - [ ] `conviction_signal` : étalonnage encore correct ? (re-test conviction à ~500 = 467/500 ce jour)
  - [ ] `analyse_rapport` : pistes actionnables ou verbeux ?
  - [ ] `analyse_rapport_asset` : les conseils citent-ils les bons chiffres ?
  - [ ] `analyse_creneaux` : les croisements asset×événement sont-ils pertinents ?
  - [ ] `analyse_portefeuille` : qui porte / qui coince — utile et concret ?
- [ ] Colonne IA des signaux : conviction 0-100 + raison au survol

## 10. Presse & calendrier [§ 8]

- [ ] Page Presse : brief en tête (contexte + articles)
- [ ] Cartes avec barre de score (rouge/jaune/gris) et lien source
- [ ] Traduction d'un article : bascule FR/VO
- [ ] Calendrier économique : 10 jours, impacts visibles

## 11. Données & santé [§ 9]

- [ ] Page Données : workers affichés avec statut (Bybit, MT5, news)
- [ ] Fraîcheur par asset : < 2 min pour tous les assets armés
- [ ] Cases assets : état conforme ; « N / total activés »
- [ ] Décocher un asset NON armé (ex. DOGE) → exclu ≤ 60 s (fraîcheur cesse de croître)
- [ ] Le recocher → le flux reprend
- [ ] Décocher un asset ARMÉ (ex. SOL) → **réactivé automatiquement 🩹 dans la minute** (garde-fou) — case reste cochée au rechargement
- [ ] + Ajouter un asset : modale s'ouvre, annuler (pas d'ajout fantôme)

## 12. Synchronisation & robustesse [§ 10]

- [ ] Modifier un réglage global (ex. trailing SMC 0,1 → 0,15) → log de réinscription de TOUS les couples ≤ 60 s
- [ ] Remettre 0,1 → nouveau cycle de réinscription
- [ ] Modifier la surcharge d'UN asset → log pour SES couples SEULS
- [ ] Un trade ouvert avant modification : ses niveaux/lignes sur graphique inchangés
- [ ] L'historique d'hier : identique bit à bit (vécu immuable)

## 13. Extinction & redémarrage [§ 2.3]

- [ ] Fermeture propre de l'app (tout s'arrête — comportement attendu)
- [ ] Relance run.sh → tous les réglages d'aujourd'hui conservés (surcharge test, armements)
- [ ] Surcharge testée en § 8 : toujours présente (ou réinitialisée si c'était le choix)
- [ ] Les positions ouvertes survivent (rattrapage cohérent, aucune clôture fantôme)

## 14. Clôture de la vérification

- [ ] Chaque case non cochée = un écart documenté ci-dessous
- [ ] Décisions à prendre listées (ex. sort de la page heatmap orpheline — D3)

---

## 📋 Synthèse des écarts trouvés

| # | Parcours | Constat (description) | Cause suspectée | Correctif proposé | Statut |
|---|---|---|---|---|---|
| 1 | 1. Démarrage | Actifs MT5 ~12 h de retard (dernière M1 XAUUSD 22:56 vendredi) | **NORMAL — samedi, forex fermé** (confié owner 10/10) | préciser « week-end = normal » dans le manuel § dépannage | à faire (doc) |
| 2 | 1. Démarrage | Bannières boot trompeuses : « Boucles Straddle SUSPENDUES » / « Worker Rockets scan SUSPENDU » alors que 81 créneaux armés + scan rockets frais (11:23:26) | vieux logs legacy d'une phase passée | purger/clarifier ces lignes | à décrire puis corriger |
| 3 | 1. Démarrage | ForexFactory JSON semaine suivante → HTTP 404 | URL distante changée ou transient | vérifier la modale calendrier (parcours 2) ; corriger l'URL si persistant | en attente |
| 4 | 1. Démarrage | Boucle « Bybit linear session fermée ~µs » (bruit log) | aucun actif linear Bybit par design (XAU/XAG = MT5) | descendre ce cas en log debug | cosmétique |
| 8 | 3. SMC | AUTO-ARM : est_arme() considérait tout asset ABSENT de la map comme armé (M1-M30) — 15 assets réactivés hier soir → 62 trades hors périmètre (dont positions M1 remplies). Scanner corrigé le 27/09 mais pas l'armement runtime | sémantique historique « tout armé » (décision 01/09) jamais alignée | CODÉ : None => false (aligné scanner) + test ; PURGE 62 trades + scoring (backup dédié) sur ordre owner ; vérifié en prod au boot 16h02 (27 couples légitimes, LINK désarmé, 0 rescapé). 156 trades anciens (14-28/09) PURGÉS sur ordre owner (avec scoring, couverts par le même backup) — vécu SMC final : 499 trades · Σ R +89,75 | **résolu, vérifié prod** |
| 9 | 3. SMC | En-tête historique (173 trades · +13,56 R) contredisait la carte dashboard (654 · +112,5 R) : fenêtre « 500 derniers signaux toutes stratégies » + somme FRONT sur la fenêtre | la promesse 16/09 « Σ R servi par le backend » violée sur l'en-tête | CODÉ : totaux de l'en-tête servis par /api/analyses (nb_trades + r_distance_total — même source que le dashboard), repli fenêtre si analyse indisponible | à valider à l'écran |
| 10 | 2. Dashboard | Jauge « Positioning BTC/ETH L/S » (Bybit) retirée sur demande owner + résumé assets « X assets » sans « · Y couples » | — | CODÉ : suppression bout-en-bout (PlancheMeteo bloc/calculs/textes, types marché, backend Positioning/fetchers/cache/payload), zéro résidu | à valider à l'écran |
| 11 | 3-5. Pages | État « Officielle/Observation » divergent : StraddleView hardcodait « Observation » et RocketsView aussi (registre = Officielle pour les deux) — les cartes dashboard contredisaient les pages | états hardcodés | CODÉ : composable useEtatStrategie (source unique /api/strategies, cache partagé) branché sur les 4 pages | à valider à l'écran |
| 12 | 3-5. Historiques | Doctrine owner : niveaux figés (SL/TP), Sortie et Stratégie sortent de TOUS les historiques ; Gain/Perte + Évolution du capital généralisés (SMC, straddle, KDJ) ; rockets : Invalidation + Montant retirés (Qté gardée), Évolution du capital ajoutée | tables chargées de colonnes sans valeur de relecture | CODÉ : HistoryTable refondu (variantes smc/straddle/kdj), rockets idem (cumul chronologique pl_dollars depuis capital_depart /api/analyses) | à valider à l'écran |
| 13 | 4. Straddle | Section « 📜 Créneaux événements » de la page dupliquée avec la modale « Assets & créneaux » du dashboard (même matrice 81 cases, mêmes boutons) — owner : un seul poste d'armement | duplication 28/09 | CODÉ : modale complétée (seuils de la boucle + verdicts 7 j + archive déplacés dedans), page Straddle épurée (agenda seul), composable partagé inchangé, zéro code mort | à valider à l'écran |
| 14 | 4. Straddle (VALIDÉ owner 10/10 soir) | Modale « Assets & créneaux » : titre trompeur (volet gauche ne gérait que les assets, créneaux à droite) — owner : UN SEUL grand tableau | information éclatée en 2 volets | CODÉ (option A) : matrice croisée UNIQUE — lignes = 9 créneaux (toggle de ligne + heure + totaux), colonnes = assets (toggle = périmètre, compteur x/9, ⚠ hors périmètre), cellules = cases cliquables (✅/❌/⏳/⚠/◉ + t·ΣR) ; bandeau bas = seuils + verdicts validés + archive ; nouvel endpoint POST /armement/evenement (ligne entière, armer = test à zéro) ; ajout d'assets par chips « + » ; RETRAIT d'un asset = suppression de ses cases (colonne disparaît, chip revient — réajouter = nouveau test à zéro) ; bannière « Validés » supprimée (tout est dans la matrice) | à valider à l'écran |
| 15 | 4. Straddle (VALIDÉ owner 10/10 soir) | Créneaux enrichis (owner 10/10) : + Ouverture Tokyo (fixe, DST — 2h Paris été/1h hiver) et surtout les TYPES CALENDAIRE (NFP, CPI, FOMC, PCE, GDP, Retail, Jobless, Dépenses perso + filet « autres High ») — l'annonce datée change chaque semaine, son type non ; le rail tier 1 inconditionnel SUPPRIMÉ : chaque passe passe par une case armée, stats/verdicts s'accumulent par (asset × type) | annonces calendrier non contrôlées ni mesurées | CODÉ : taxonomie 19 lignes (champ calendrier), classificateur titre→type (11 tests), annonces_evenements unifié (heure réelle de l'annonce classée), annonces_tier1 supprimée (zéro dette), fenêtres hot-reload alignées, prochaine occurrence réelle affichée (jour/mois), marqueurs 📅/📯 dans la matrice | à valider à l'écran |
| 16 | 4. Straddle (VALIDÉ owner 10/10 soir) | Matrice 📅 vide d'historique (lignes nées à zéro) — owner : remplir avec les trades passés | compteurs neufs | FAIT : les 9 fixes comptaient DÉJÀ en live depuis le 28/09 (1er backfill les avait doublés — restauré depuis backup, leçon retenue) ; correctif chirurgical = AJOUT cal_fomc 7 passes (rail tier 1 d'alors, compté nulle part) + TRANSFERT des 8 passes jeudi 14:30 de annonces_us_0830 vers cal_jobless (sémantique du nouveau rail) ; déduplication slot/ligne classée codée + testée (une NFP à 14:30 = UNE passe) | à valider à l'écran |
| 17 | 4. Straddle (VALIDÉ owner 10/10 soir) | Cellules 📅 vides (CPI/PCE/PIB/Retail/perso/nfp = 0 tirage — normal, titres historiques non conservés) + owner : analyse IA sous la matrice (meilleurs événements/assets/croisements) + prompt éditable | — | CODÉ : POST /api/evenements/ia (Ollama, cache du jour, contexte = compteurs réels par événement/asset/CASE + règle des 30 par case), prompt « analyse_creneaux » ajouté aux prompts éditables, bloc 🤖 sous le tableau (état + événements à travailler + meilleurs croisements + confiance) | à valider à l'écran |
| 18 | 7. Graphiques (VALIDÉ owner 11/10) | Fenêtre « Graphiques » (tuile dashboard) OUVERTE avec SA barre native + celle de l'app (2 barres, ─ □ ✕ de la barre native inopérants) ; bouton « Voir » d'une position naviguait DANS la fenêtre principale (remplaçait le dashboard) — 2 comportements incohérents | WebviewWindow créée sans decorations:false + router.push | CODÉ : ouvrirFenetreGraphiques() partagé (tuile + Voir) avec decorations:false (une seule barre, contrôles de l'app), fenêtre existante → setFocus + re-ciblage du slot 1 par événement storage (ChartsView écoute) ; + capabilities Tauri : fenêtre « Graphiques » ajoutée aux permissions (─ □ ✕ refusés en silence — windows: [main] uniquement) | à valider à l'écran |
| 19 | 7. Graphiques (VALIDÉ owner 11/10 après relance) | Dessins : fibo se trace, LIGNE et RECTANGLE « impossibles à tracer » (owner 11/10) | en investigation — cycle pointeur/rendu partagé par les 3 outils, aucune asymétrie visible au code ; symptôme précis demandé | — | en discussion |
| 20 | 9. Analyses | Vue d'ensemble enrichie (owner 11/10) : « les TROIS stratégies » alors qu'elles sont 4, pas de totaux, pas de lecture IA ; onglets redondants (accès par cartes dashboard) | vue d'ensemble squelettique d'avant KDJ | CODÉ : liste DYNAMIQUE (icônes 4/4) + ligne TOTALES (clôtures, WR pondéré, ΣR, $ cumulés, hier) + synthèse IA du portefeuille (POST /api/analyses/portefeuille/ia, prompt éditable analyse_portefeuille, cache du jour) ; ONGLETS SUPPRIMÉS (bouton ← Vue d'ensemble à la place) ; cartes dashboard : blocs compactés 1 ligne + NOUVELLE carte « Rapport d'activité » par stratégie (clôtures · $ total · hier → /analyses?strategie=X) | à valider à l'écran |
| 21 | 9. Analyses | Rapport par stratégie : 7 blocs REPLIÉS (invisible par défaut) empilés verticalement, pas de fil de lecture | historique <details> | CODÉ (owner 11/10, design validé) : 3 colonnes pleine largeur (GAUCHE bilan : verdict + périodes · CENTRE leviers : classements + edge + timeframes · DROITE temps : heatmap + snapshots) + analyse IA TOUJOURS VISIBLE pleine largeur sous la grille + score SMC en pleine largeur ; contenus internes inchangés (zéro logique) ; styles orphelins des <details> purgés | à valider à l'écran |
| 5 | 6. KDJ | Tableaux KDJ à ajuster (owner 10/10) : en cours = TP1 seul renommé TP, masquer Score ; historique = masquer Score, supprimer Verdict (redondant palier max), colonnes **Gain/Perte** puis **Évolution du capital** après palier max | variante KDJ incomplète | SignauxTableau (TP unique, sans Score) + HistoryTable (sans Score/Verdict, + Gain/Perte, + Évolution du capital) | **validé owner 10/10** |
| 6 | 6. KDJ | Colonne Lot vide pour les 3 premiers trades KDJ | calcul du lot : repli capital 0 quand aucune clôture n'existe avant l'émission | repli sur capital_depart (premiers trades de toute stratégie) ; enchaînement capital→lot vérifié sur données (LINK 12,99 ≈ 1 002 $ × 1 % / 0,77) | **validé owner 10/10** |
| 7 | 6. KDJ | Lots divergents entre « en cours » et historique (3 causes : capital statique front vs composé backend, sl_pips conventionnel vs distance réelle, clamp) + logique sizing : KDJ doit s'exprimer en **unités + % capital**, pas en lot forex (owner 10/10 — SMC/straddle inchangés) | deux calculs distincts | source unique backend (capital à l'émission) dans les DEUX tables via /api/signaux/lots ; colonne « Position » KDJ = **N unités** + « X $ risqués · 1 % » + ≈ Y $ engagés | **validé owner 10/10** |

> Rappel : **décrire avant de corriger** — chaque écart passe par ce tableau avant
> toute modification. Les correctifs seront validés un à un.
