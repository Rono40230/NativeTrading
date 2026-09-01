# ROADMAP — Native Trading AI

> État au 29 août 2026. Cette roadmap ne contient que **ce qu'il reste à faire**.
> Le détail des phases livrées SMC v12 vit dans `docs/AMELIORATIONS_SMC_V12.md`,
> les études des étapes 3-4 dans `docs/ETAPE3_*.md` et `docs/ETAPE4_CALCUL_TRADES.md`.

---

## État des lieux (bref — pour situer le travail restant)

| Verticale | État | Moteur | Source données | IA |
|---|---|---|---|---|
| **SMC** | Officielle | v12 figé : étapes 3-4-5 closes (TP1=0.6R + SL×0.75 = +239R · miroir MQL5 livré) | XAU/XAG/NAS/SP/DAX : MT5/Axi · BTC : Bybit | Analyse (bouton) |
| **Straddle** | Observation | v2 redéfini (2 jambes à E, T-10s, trailing tick) | XAU/BTC/NAS/SP : MT5+Axi · annonces US | À construire (après gate 3) |
| **Rockets** | Observation | Scanner D1 /10 + gestion journal (R1→50 %+trailing) — 2 univers : crypto (Binance) + actions US Tiingo/QQQ (Observation silencieuse depuis 01/09) | Binance + Tiingo | Catalyseur news + ranker ✅ |

**Miroir v12 terminé (29/08)** : Pine étalon (`docs/reference/`) = Rust (`smc` 226 tests ✓)
= MQL5 indicateur + EA (`mt5/`, commité `12515b8`). Parité vérifiée : TP1=0.6R, SL×0.75,
TP3 DoL cappé 3R, BPR, BSZones, London HL, Asian HL, mega-vol, prevLiq.
Les 3 exemplaires MQL5 (source `Applis Nono`, dossier MT5, dépôt git) sont synchronisés.

**Infrastructure** : runtime tick (bus signaux/événements/bougies) · EA MT5/Axi (temps réel +
historique 24 mois en delta) · Bybit WS (BTC) · analyste IA qwen3:32b local · Telegram
(annonces d'imminence seules) · presse traduite FR · vue Données (pipeline complet).

**Pages Définition** : SMC (6 onglets), Straddle (5), Rockets (6) — Enrichissement IA documenté.

---

## À faire — par ordre de priorité

### 1. Étape 5 — Résiduel : validation numérique du miroir MQL5

La parité est prouvée **par construction** (mêmes règles, constantes, ordre d'exécution).
Reste à la prouver **par les nombres** :

- [ ] **EA dans le Strategy Tester** : backtest `smc_ea_v12.mq5` sur une paire/période de
      référence (ex. XAU M15) et comparaison aux chiffres du replay Rust sur les mêmes bornes
      (nombre de signaux, verdicts, R cumulé — écart attendu ≈ 0)
- [ ] **Écart ≠ 0** : le tracer règle par règle jusqu'à la divergence (le miroir est la base
      de l'automatisation future des ordres — il doit être exact)
- [ ] **Pine dans TV** (action propriétaire) : coller le Pine de `docs/reference/`
      dans TradingView sous « Scalp à Nono »

### 2. Gate 3 — Straddle en conditions réelles

Les passes des 26-28/08 (PCE/PIB mercredi, Warsh/Payrolls vendredi) ont été jouées.

- [ ] Vérifier la journalisation des passes (table signaux, verdicts SL/BE/TS/TimeStop + R)
- [ ] Bilan gate 3 : verdicts, R cumulé, comportement trailing sur les 4 annonces
- [ ] Décision propriétaire : passage Officielle ou ajustements
- [ ] Si Officielle : activer le son Telegram (template prêt, dormant)

### 3. SMC v12 — Surveillance production

Le programme d'améliorations est **clos**. Reste = **vérifier que la production confirme
le replay** :

- [ ] **SP500 live** : après ~2 semaines, comparer la production réelle au replay (fréquence
      M15/M5, verdicts — règle 30 trades). Divergence marquée → étude calibration dédiée
      (profil actuel = miroir NAS100)
- [ ] **Mega-orders live** : confirmer l'apport +21.3R en réel (delta replay concentré BTC M5 —
      réserve documentée)
- [ ] Tout réglage ne bouge que sur preuve ≥ 30 trades remplis par tranche (anti-overfitting)

### 4. Agenda intelligent — Créneaux de volatilité récurrents (Straddle IA)

**Prérequis** : historique 24 mois en base ✅ (l'analyse peut tourner dès maintenant).

**Ce que c'est** : l'analyste découvre les créneaux où la volatilité explose de façon
récurrente (fixing Londres, ouvertures de session, données sans badge « High », effets
hebdomadaires) — au-delà du calendrier actuel.

**Architecture** (constitution respectée : calcul → IA lit → propriétaire règle) :

- [ ] **Backend** : calcul statistique des créneaux (volatilité heure×jour sur 24 mois M1,
      pics récurrents, corrélation calendrier existant)
- [ ] **Backend** : endpoint `GET /api/straddle/creneaux-detectes`
- [ ] **IA** : l'analyste lit créneaux + calendrier, propose armer/ignorer avec justification
- [ ] **Front** : page « Straddle › Agenda intelligent » (nom, horaire, volatilité ×N,
      mouvement typique, constat IA, bouton armer/ignorer)
- [ ] **Moteurs** : créneaux armés = annonces synthétiques (même structure que l'ouverture
      européenne DAX)
- [ ] **Dashboard** : bloc Straddle affiche tous les créneaux armés avec badge distinctif
- [ ] Les créneaux découverts démarrent en **Observation** (journalisés, silencieux)

### 5. Test de vérité au centime

**Prérequis** : MT5/Axi alimente l'app en continu ✅ (même source, même horloge UTC).

- [ ] Comparer bougie par bougie (OHLCV) nos M1 Axi vs le graphique MT5 sur une session complète
- [ ] Comparer les signaux SMC sur XAU (même source → aucun écart attendu)
- [ ] Si écarts : les tracer et les corriger
- [ ] Documenter le verdict au journal

### 6. Rôles IA Straddle (après gate 3)

- [ ] **Analyse des passes** : l'analyste lit les passes journalisées (annonce → range → fill →
      verdict R) et explique ce qui marche / coince
- [ ] **Recommandation agenda + minutage** : proposer quels événements valent le coup et à
      quelle minute entrer — propositions validées par le propriétaire dans les réglages

### 7. Rôles IA SMC (après accumulation de trades propres)

- [ ] **Voile des setups** : l'analyste lit annonces vs confirmés vs dissipés (le panneau
      existe) et identifie les caractéristiques des setups qui tiennent
- [ ] **Analyse rebranchée** : bouton Analyse SMC reconnecté sur les données propres
      (remplis/jamais remplis/dissipés/verdicts)
- [ ] **Décision sur preuve — filtre temps réel** : après ~2 semaines, regarder si les setups
      dissipés partagent des caractéristiques repérables → décider si un filtre vaut le coup
- [ ] **Journalisation du détail scoring** (préalable si analyse par confluence) : stocker le
      détail point par point à l'émission du signal

### 8. Rockets — Extensions

- [ ] **Véto unlocks** : source libre (API/scraping d'un calendrier public de déverrouillages
      de tokens) → intégrer au scanner (éliminatoire si unlock majeur < 30 jours)
- [ ] **ETF via Tiingo** : le rail actions (livré 01/09 — Axi n'expose aucun
      share CFD, source Tiingo retenue) se généralise aux ETF du répertoire
      NASDAQ Trader (lever l'exclusion ETF + profils 2/3/4 % dédiés) — même
      pipeline, après validation de l'Observation actions
- [ ] **Analyse par pilier** : l'analyste relie les critères du /10 aux verdicts des positions
      clôturées (déjà journalisés) → propositions de recalibrage chiffrées

### 9. Maintenance & dette technique

- [ ] **Revue complète des prompts IA** : purge des prompts morts (`smc_signal`, `smc_filtre`,
      `rockets_opportunites` — endpoints consommateurs supprimés, à confirmer), audit des
      actifs, alignement sur les mécaniques actées (étapes 3-4 déjà répercutées)
- [ ] Graphique XAU : vérifier l'affichage des zones SMC sur l'historique Axi profond (2 ans —
      artefacts au changement de source à tracer)
- [ ] ETH : réactiver et re-backfiller si souhaité (purgé par le job de rétention 24 mois)
- [ ] Config `worker_historique_mois` : 6 mois mais l'historique MT5 en couvre 24 — harmoniser
      avec la rétention
- [ ] Leçons L1-L11 : à relire avant toute intervention (archivées au journal)

---

## Règles transverses (non négociables)

| Règle | Détail |
|---|---|
| **Pine = étalon** | Le moteur v12 est figé sur le Pine (md5 vérifié). Toute déviation = bug Rust/MQL5, jamais « amélioration » |
| **Discussion avant construction** | Chaque étape fait l'objet d'une discussion de définition avant le code |
| **Vocabulaire français** | États : Officielle / Observation / Construction. Pas d'anglicismes dans l'app |
| **Telegram = imminence seule** | Pas de clôture/fill/BE/TP. Le lot s'affiche avec le montant risqué |
| **L'IA n'exécute jamais** | « L'IA lit, juge, propose, explique — les moteurs décident, toi seul règles » (constitution gravée 24/08) |
| **Un actif = une série** | Une seule source de vérité par actif (MT5/Axi pour XAU/XAG/NAS/SP/DAX, Bybit pour BTC) |
| **30 trades minimum** | Aucune proposition de recalibrage sous 30 trades remplis par tranche |
| **Fichier ≤ 600 lignes** | Pré-commit bloquant. Extraire vers un module dédié |
| **Pas de .unwrap()/.expect() hors tests** | Pré-commit bloquant |
| **Chirurgie uniquement** | Toute correction/amélioration est CHIRURGICALE : additive, sans modifier le comportement existant, vérifiée par tests + build avant commit. Zéro casse, zéro régression (règle propriétaire 27/08) |
| **Stratégie changée = prompt changé** | Tout changement de logique, de calcul ou de mécanique dans une stratégie est porté IMMÉDIATEMENT dans les prompts IA de l'analyste (page Prompts IA — `llm/prompts.rs`) |
| **MQL5 : 3 exemplaires synchrones** | Source `Applis Nono` = dossier MT5 `~/.mt5/.../Advisors/` = dépôt `mt5/` — toute modification est copiée partout avant compilation (leçon du 29/08 : MetaEditor compile le dossier MT5) |

---

## Gates restantes

| Gate | Preuve | Statut |
|---|---|---|
| **3 — Plugins fidèles** | Straddle : passes réelles 26-28/08 journalisées + bilan propriétaire. Rockets : premières cassures qualifiées vécues par le ranker | ⬜ passes jouées, bilan à faire |
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
12. **MetaEditor compile son tampon, pas le disque** — après édition externe : fermer/rouvrir l'onglet. Et il ouvre le dossier MT5 (`~/.mt5`), pas le dossier source — toujours synchroniser les 3 exemplaires (voir règle transverse).
