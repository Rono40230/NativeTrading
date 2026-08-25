# ROADMAP — Native Trading AI

> État au 25 août 2026. Cette roadmap ne contient que **ce qu'il reste à faire**.
> L'historique complet des phases livrées est archivé dans `docs/JOURNAL.md`.

---

## Ce qui est en place (résumé — non exhaustif)

L'app est une coquille d'orchestration + 3 verticales stratégiques complètes :

| Verticale | État | Moteur | Source données | IA |
|---|---|---|---|---|
| **SMC** | Officielle | v12 fidèle Pine (24 couples, replays + amorce MTF) | XAU/XAG : MT5/Axi · BTC : Bybit | Analyse (bouton) |
| **Straddle** | Observation | v2 redéfini (2 jambes à E, T-10s, trailing tick) | XAU/BTC/NAS/SP : MT5+Axi · annonces US | À construire (étape 2) |
| **Rockets** | Observation | Scanner D1 /10 + gestion journal (R1→50 %+trailing) | Binance (crypto) | Catalyseur news + ranker ✅ |

**Infrastructure** : runtime tick (bus signaux/événements/bougies) · EA MT5/Axi (temps réel + historique 24 mois en delta) · Bybit WS (BTC) · analyste IA qwen3:32b local · Telegram (annonces d'imminence seules) · presse traduite FR · vue Données (pipeline complet).

**Pages Définition** : SMC (6 onglets), Straddle (5), Rockets (6) — toutes avec Enrichissement IA documenté.

---

## À faire — par ordre de priorité

### 1. Gate 3 — Straddle en conditions réelles

**Quand** : passes du 26-28/08 (PCE/PIB mercredi 14h30 UTC, Warsh/Payrolls vendredi 16h00 UTC).

- [ ] Laisser les moteurs jouer les 4 annonces (XAU, BTC, NAS100, SP500 sur prix Axi)
- [ ] Après chaque passe : vérifier la journalisation (table signaux, verdict SL/BE/TS/TimeStop + R)
- [ ] Après les 4 passes : bilan gate 3 avec le propriétaire (verdicts, R cumulé, comportement trailing)
- [ ] Décision propriétaire : passage Officielle ou ajustements
- [ ] Si Officielle : activer le son Telegram (le template est prêt, dormant)

### 2. Agenda intelligent — Créneaux de volatilité récurrents (Straddle IA)

**Prérequis** : historique 24 mois en base ✅ (l'analyse peut tourner dès maintenant).

**Ce que c'est** : l'analyste découvre les créneaux où la volatilité explose de façon récurrente (fixing Londres, ouvertures de session, données économiques sans badge « High », effets hebdomadaires) — au-delà du calendrier actuel.

**Architecture** (constitution respectée : calcul → IA lit → propriétaire règle) :

- [ ] **Backend** : calcul statistique des créneaux (volatilité par heure×jour sur 24 mois M1, détection de pics récurrents, corrélation avec le calendrier existant)
- [ ] **Backend** : endpoint `GET /api/straddle/creneaux-detectes` (résultats du calcul)
- [ ] **IA** : l'analyste lit les créneaux détectés + le calendrier, identifie les causes probables, propose armer/ignorer avec justification
- [ ] **Front** : page « Straddle › Agenda intelligent » — liste des créneaux (nom, horaire, volatilité médiane ×N, mouvement typique, constat IA, recommandation) avec bouton armer/ignorer
- [ ] **Moteurs** : les créneaux armés deviennent des annonces synthétiques (même structure que l'ouverture européenne DAX)
- [ ] **Dashboard** : le bloc Straddle affiche tous les créneaux armés (calendrier + découverts) avec badge distinctif
- [ ] Les créneaux découverts démarrent en **Observation** (journalisés, silencieux) — passage Officielle sur preuve

### 3. Test de vérité au centime

**Prérequis** : MT5/Axi alimente l'app en continu ✅ (même source, même horloge UTC).

- [ ] Comparer bougie par bougie (OHLCV) nos bougies M1 Axi vs le graphique MT5 sur une session complète
- [ ] Comparer les signaux SMC sur XAU (même source → aucun écart attendu)
- [ ] Si écarts : les tracer et les corriger
- [ ] Documenter le verdict au journal

### 4. Rôles IA Straddle (après gate 3)

- [ ] **Analyse des passes** : l'analyste lit les passes journalisées (annonce → range → fill → verdict R) et explique ce qui marche / coince
- [ ] **Recommandation agenda + minutage** : sur les données accumulées, proposer quels événements valent le coup et à quelle minute entrer — propositions que le propriétaire valide dans les réglages

### 5. Rôles IA SMC (après accumulation de trades propres)

- [ ] **Voile des setups** : l'analyste lit la donnée annonces vs confirmés vs dissipés (le panneau existe) et identifie les caractéristiques des setups qui tiennent
- [ ] **Analyse rebranchée** : le bouton Analyse SMC reconnecté sur les données maintenant propres (remplis/jamais remplis/dissipés/verdicts)
- [ ] **Décision sur preuve — filtre temps réel** : après ~2 semaines de données, regarder si les setups dissipés partagent des caractéristiques que l'IA saurait repérer → décider ensemble si un filtre vaut le coup
- [ ] **Journalisation du détail scoring** (préalable si analyse par confluence) : stocker le détail point par point à l'émission du signal

### 6. Rockets — Extensions

- [ ] **Véto unlocks** : chercher une source libre (API ou scraping d'un calendrier public de déverrouillages de tokens) → intégrer au scanner (éliminatoire si unlock majeur < 30 jours)
- [ ] **Actions/ETF via MT5** : Axi les propose — l'ajout passe par la modale 📦 Données (source MT5, symbole broker) ; le classement ETF a ses profils 2/3/4 %
- [ ] **Analyse par pilier** : l'analyste relie les critères du /10 aux verdicts des positions clôturées (le détail est déjà journalisé) → propositions de recalibrage chiffrées

### 7. Maintenance & dette technique

- [ ] Graphique XAU : vérifier l'affichage des zones SMC sur l'historique Axi profond (2 ans de contexte — si des artefacts apparaissent au changement de source, les tracer)
- [ ] ETH : réactiver et re-backfiller si souhaité (purgé par le job de rétention 24 mois — il était inactif)
- [ ] Config `worker_historique_mois` : 6 mois mais l'historique MT5 en couvre 24 — harmoniser la config avec la rétention
- [ ] Leçons L1-L11 : à relire avant toute intervention (archivées au journal)

---

## Règles transverses (non négociables)

| Règle | Détail |
|---|---|
| **Pine = étalon** | Le moteur v12 est figé sur le Pine (md5 vérifié). Toute déviation = bug Rust, jamais « amélioration » |
| **Discussion avant construction** | Chaque étape fait l'objet d'une discussion de définition avant le code |
| **Vocabulaire français** | États : Officielle / Observation / Construction. Pas d'anglicismes dans l'app |
| **Telegram = imminence seule** | Pas de clôture/fill/BE/TP. Le lot s'affiche avec le montant risqué |
| **L'IA n'exécute jamais** | « L'IA lit, juge, propose, explique — les moteurs décident, toi seul règles » (constitution gravée 24/08) |
| **Un actif = une série** | Une seule source de vérité par actif (MT5/Axi pour XAU/XAG/NAS/SP/DAX, Bybit pour BTC) |
| **30 trades minimum** | Aucune proposition de recalibrage sous 30 trades remplis par tranche |
| **Fichier ≤ 600 lignes** | Pré-commit bloquant. Extraire vers un module dédié |
| **Pas de .unwrap()/.expect() hors tests** | Pré-commit bloquant |

---

## Gates restantes

| Gate | Preuve | Statut |
|---|---|---|
| **3 — Plugins fidèles** | Straddle : passes réelles 26-28/08 journalisées + verdicts R + bilan propriétaire. Rockets : premières cassures qualifiées vécues par le ranker | ⬜ en cours |
| **4 — Extensions isolées** | Kill de l'EA MT5 sans impact backend + presse résiliente | ⬜ (prouvée en partie : kill api pendant les tests du jour) |
| **5 — Sources additionnelles** | MT5/Axi : couverture 24 mois ✅ + bougies temps réel ✅. Test de vérité au centime = dernière preuve | ⬜ |

---

## Leçons durables (nées des erreurs — à lire avant toute intervention)

1. **pkill par nom exact uniquement** (`pkill -x`) — jamais par motif de chemin, sinon on tue son propre terminal.
2. **Vérifier chaque édition appliquée** — un remplacement silencieux peut rater sa cible (apostrophe courante vs droite, etc.).
3. **Vérifier que le binaire tourne** après un build — un échec silencieux laisse l'ancien process en place.
4. **Ne jamais continuer sur un build en échec** — arrêter, corriger, relancer.
5. **Timestamps en secondes Unix partout** — jamais de ISO 8601 en DB (hétérogénéité = bugs silencieux).
6. **Mapper tous les écrivains et lecteurs d'une table avant de toucher au schéma.**
7. **Une seule série de prix par (asset × TF)** — deux sources qui écrivent la même bougie = corruption.
8. **Pas d'analyse d'image sans vérification humaine** — la vision IA invente des détails.
9. **Migrations = embarquées à la compilation** — toucher `db/src/lib.rs` pour forcer la re-embed (récidive du 25/08 : migration 0087 non appliquée au premier déploiement).
10. **MT5 WebRequest : gros corps POST = crash** (morceaux ≤ 2 000 bougies ≈ 130 Ko) — et StringSplit compte une sous-chaîne vide finale absente du tableau (borner par ArraySize, jamais par le retour).
11. **Horloge serveur broker ≠ UTC** (Axi = GMT+2/+3 avec DST US) — convertir serveur→UTC à la source, sinon tout est décalé de 2-3 heures.
| 2026-08-25 | Phase 5.2 | **GRAPHIQUE MT5 RÉPARÉ (définitif)** — le prix vivait mais l'historique disparaissait au connect du stream. Cause : le flush du cache (ajouté comme protection GMT+3) effaçait l'historique REST, puis le batch vide MT5 laissait une seule bougie. Fix : flush retiré (le try/catch sur series.update suffit), garde anti-crash conservée. Vérifié propriétaire : « tout est ok ». La Phase 5.2 est TERMINÉE — MT5/Axi alimente l'app en continu (prix + historique + moteurs), le graphique vit sur tous les actifs. Prochain rendez-vous : gate 3 demain 14h30 UTC (PCE/PIB) |
