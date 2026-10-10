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

- [ ] Bloc STRATÉGIES : 4 colonnes avec capital, Σ R, WR, compteur de positions, état
- [ ] Les 4 capitaux correspondent aux valeurs d'hier (vécu immuable — pas de saut)
- [ ] Planche MÉTÉO animée (jauges/ticker)
- [ ] Horloges SESSIONS MONDIALES à l'heure
- [ ] Bandeau : les 3 badges cliquables → 3 modales s'ouvrent (calendrier, créneaux, macro)
- [ ] Modale calendrier : annonces des 10 prochains jours présentes
- [ ] Fenêtre macro : visible si annonce High ±30 min à venir ; sinon silencieuse (normal)
- [ ] Pedestal COMMS : fenêtres par source vivantes
- [ ] Cartes positions à risque / neutralisées cohérentes avec les pages stratégies
- [ ] Toggle Telegram d'une stratégie : réagit au clic (état conservé après rechargement)

## 3. Stratégie SMC [§ 4.1]

- [ ] Page SMC : positions en cours + historique + bloc moteur affichés
- [ ] Table en cours : colonnes R Latent / P/L Latent alimentées (prix vivants)
- [ ] Historique : tri par colonne fonctionnel ; paliers max et $ présents
- [ ] Aucun trade fermé n'affiche « ⏳ En cours » (vocabulaire verdicts — régression du 09/10)
- [ ] ⚙️ Paramètres : registre (capital 1000, risque 1 %) et niveaux (TP1 0,6 · TP2 2,0 · TP3 lointaine · trailing ON 0,1 · fractions 100/0/0)
- [ ] Bloc « Réglage par asset » : sélectionner XAUUSD → badge « surchargé » sur trailing 0,2 ; placeholder grisé sur les champs au défaut
- [ ] 🕐 Timeframes/Assets : grille conforme (9 assets × M5/M15/M30 ; M1 et H1 désarmés)
- [ ] Page Définition SMC : lexique complet charge
- [ ] Page Scanner SMC : bandeau 📍 Zones à l'approche présent + cartes du vivier

## 4. Stratégie Straddle [§ 4.2]

- [ ] Page straddle : agenda des annonces + table des passes + bloc moteur
- [ ] Matrice 81 cases : état armé complet, case cliquable (armer/désarmer une case test puis la remettre)
- [ ] ⚙️ Paramètres : placement 10 s, SL × ATR, trailing — valeurs conformes
- [ ] Surcharge par asset straddle : sélecteur + badges opérationnels
- [ ] Page Définition straddle : charge

## 5. Stratégie Rockets [§ 4.3]

- [ ] Page rockets : positions + historique (verdicts 🏁/⏹/👤/❌ correctement libellés)
- [ ] Scanner : vivier de candidats avec scores par pilier
- [ ] Mention règle des 30 honnête (9 clôtures → rien de significatif affiché comme tel)
- [ ] Page Définition rockets : charge

## 6. Stratégie KDJ [§ 4.4]

- [x] Page KDJ : 2 positions en cours (LINK/SOL, 09/10 22h00) + historique
- [x] Historique = variante KDJ : sans SL/TP1-3/Stratégie ni Score, verdict lisible dans Palier max (✅ TP / ❌ SL), colonnes Gain/Perte + Évolution du capital, colonne Position (unités) — écarts 5-7 validés owner
- [x] XPTUSD fermée TP (+2,24 R) — Σ R ≈ +0,18
- [x] ⚙️ Paramètres : period 20, signal 7, amplitude 2, ratio 2, ADM min off
- [x] Modale assets KDJ : 6 cochés (BNB, XAGUSD, XPTUSD, XRP, LINK, SOL)
- [x] Scanner KDJ : ADX H1/D1, direction, Prêt — lignes présentes (XAGUSD/XPTUSD figés : samedi = normal)
- [x] Page Définition KDJ : charge

## 7. Graphiques & surveillance [§ 5]

- [ ] Page Graphiques : cellules actives (cadre cyan au clic)
- [ ] Changer asset/TF d'une cellule : bougies se chargent, voyant **flux** vert
- [ ] Plein écran (double-clic) puis Échap : fonctionnels
- [ ] Panneau indicateurs : activer OB + structure + tendance → Appliquer → dessins corrects
- [ ] Indicateur OB Institutionnels : bandes OTE visibles, confluence dorée marquée
- [ ] Dessin : tracer une tendance → changer de TF → le dessin persiste
- [ ] Alerte prix : 🔔 pose au clic → liste avec l'alerte → supprimer
- [ ] Trade ouvert (ou en créer l'attente) : visible sur le graphique de son TF
- [ ] **Décaler le graphique** (espace entre dernière bougie et axe) : le trade RESTE visible jusqu'à la bougie en cours — **régression du 09/10, point critique**
- [ ] Multi-TF : le même trade apparaît sur un autre TF du même asset avec badge
- [ ] Bandeau 📍 Zones à l'approche : se met à jour (30 s) ; vide acceptable si rien en bande
- [ ] Test nuit (si une zone approche) : régler temporairement `smc_alertes_zones_nuit_debut/fin` sur l'heure courante → silence total → remettre 23/7
- [ ] Éco-cal sur graphique : annonces visibles, tooltip au survol

## 8. Laboratoire [§ 6]

- [ ] Page Simulation : badge jaune « jamais les chiffres officiels »
- [ ] Onglets SMC / straddle / KDJ fonctionnels
- [ ] Chips « Paires simulées » : **exactement les 9 assets armés** (régression du 17/09)
- [ ] Chips « Timeframes simulés » : M1/M5/M15/M30
- [ ] ▶ Lancer une simulation (~35 s) → comparatif vécu/sim s'affiche
- [ ] 📊 Balayer les fractions (~qq min) → table + essais créés
- [ ] Advisory 🎯 : meilleur essai OU « effectif insuffisant » (honnêteté)
- [ ] Cliquer une seule chip (ex. XAUUSD) → badge **PAR ASSET XAUUSD** sur l'advisory
- [ ] ⚡ Activer en PAR ASSET sur un asset secondaire → modale ⚙️ : badge surchargé sur cet asset SEUL
- [ ] Log hot-reload : réinscription des SEULS couples de l'asset ≤ 60 s
- [ ] Réinitialiser la surcharge → retour au défaut + log de réinscription
- [ ] Vérifier après coup : les autres assets n'ont PAS bougé (log)

## 9. Analyses & IA [§ 7]

- [ ] Page Analyses : vue d'ensemble + onglets des 4 stratégies
- [ ] Bloc verdict « 5 secondes » : capital, Σ R distance, WR, hier
- [ ] Classements assets/TF/événements : tri Σ R distance, lignes grises < 30
- [ ] Bouton 🤖 sur un asset du classement → modale : badges JUGEABLE/confiance, état, conseil, **chiffres clés avec leur phrase d'explication**
- [ ] « ⚡ Générer l'avis » de l'analyse IA (~1 min, Ollama) → état/forts/faibles/pistes + confiance
- [ ] Historique snapshots : avis IA des jours passés lisibles
- [ ] Page IA : onglet ML (features, monitoring par stratégie) + onglet Prompts
- [ ] Prompts : liste éditable ; ouvrir `analyse_rapport_asset` → texte présent, annuler sans sauvegarder
- [ ] Colonne IA des signaux : conviction 0-100 avec raison au survol

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
| 5 | 6. KDJ | Tableaux KDJ à ajuster (owner 10/10) : en cours = TP1 seul renommé TP, masquer Score ; historique = masquer Score, supprimer Verdict (redondant palier max), colonnes **Gain/Perte** puis **Évolution du capital** après palier max | variante KDJ incomplète | SignauxTableau (TP unique, sans Score) + HistoryTable (sans Score/Verdict, + Gain/Perte, + Évolution du capital) | **validé owner 10/10** |
| 6 | 6. KDJ | Colonne Lot vide pour les 3 premiers trades KDJ | calcul du lot : repli capital 0 quand aucune clôture n'existe avant l'émission | repli sur capital_depart (premiers trades de toute stratégie) ; enchaînement capital→lot vérifié sur données (LINK 12,99 ≈ 1 002 $ × 1 % / 0,77) | **validé owner 10/10** |
| 7 | 6. KDJ | Lots divergents entre « en cours » et historique (3 causes : capital statique front vs composé backend, sl_pips conventionnel vs distance réelle, clamp) + logique sizing : KDJ doit s'exprimer en **unités + % capital**, pas en lot forex (owner 10/10 — SMC/straddle inchangés) | deux calculs distincts | source unique backend (capital à l'émission) dans les DEUX tables via /api/signaux/lots ; colonne « Position » KDJ = **N unités** + « X $ risqués · 1 % » + ≈ Y $ engagés | **validé owner 10/10** |

> Rappel : **décrire avant de corriger** — chaque écart passe par ce tableau avant
> toute modification. Les correctifs seront validés un à un.
