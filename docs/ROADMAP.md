# ROADMAP — Native Trading AI

> État au 8 septembre 2026. Cette roadmap ne contient que **ce qu'il reste à
> faire**. Tout l'historique livré vit dans git et dans `docs/`
> (`AMELIORATIONS_SMC_V12.md`, `ETAPE3_*.md`, `ETAPE4_CALCUL_TRADES.md`,
> `VALIDATION_MQL5.md` — verdict du miroir et réparation des données du 08/09).
>
> **État visé en fin de feuille de route : 0 dette technique, 0 code mort,
> 0 fichier inutile.** Chaque chantier se termine par son propre rangement.

---

## État des verticales (pour situer le travail restant)

| Verticale | État | Moteur | IA |
|---|---|---|---|
| **SMC** | Officielle | v12 figé (miroir Pine/Rust/MQL5 **validé au centime le 08/09**) | Conviction à l'émission (accumulation) + analyse rapport |
| **Straddle** | Observation prolongée (≥ 30 passes, point ~fin sept) | v2 unifié + agenda intelligent §16 + boucle de validation des créneaux | Analyste créneaux (ARMER/IGNORER) + boucle verdicts |
| **Rockets** | Observation | Scanner D1 /10 + gestion 30 s (crypto Binance + actions Tiingo/Yahoo) | Catalyseur news + ranker + ML |

Infrastructure saine : collecteur MT5/Axi **réparé et vérifié au centime**
(08/09), rejeu paramétrique couplé aux réglages, capital simulé composé,
analyste IA local (prompts éditables), Telegram (imminence seule), boucle ML v2
alimentée par les vraies clôtures (107+ samples, XGB OOS 66,7 %).

---

## À faire — par ordre de priorité

### 1. Straddle — passage Officielle (point de décision ~fin septembre)

Décision du 07/09 : Observation prolongée jusqu'à **≥ 30 passes closes** —
le bilan à 20 passes (+4,92 R nets) était dominé par le seul NFP du 04/09
(+4,84 R), le reste à l'équilibre.

- [ ] **Point de décision au seuil de 30 passes** : dossier rafraîchi,
      décomposé par source (annonces tier 1 · créneaux IA · ouvertures DAX).
      La diversification est en route : créneaux IA armés (BTC vendredi 16h ;
      candidats vraies heures : XAU vendredi 15h = fenêtre NFP, NAS/SP
      vendredi 16h, DAX lundi 9h — recalculés sur données réparées).
- [ ] Si Officielle : activer le son Telegram (template prêt, dormant)
- [ ] Rappel money management (décision 04/09) : une passe peut coûter
      −1,5 R nominal — assumé, lot inchangé

### 2. SMC — Surveillance production (règle des 30 trades)

- [ ] **SP500 live** : comparer la production réelle au replay (fréquence
      M15/M5, verdicts) après ~2 semaines de recul — divergence marquée →
      étude calibration dédiée (profil actuel = miroir NAS100)
- [ ] **Mega-orders live** : confirmer l'apport +21,3 R en réel (delta replay
      concentré BTC M5 — réserve documentée)
- [ ] **Ré-armer des couples coupés** via l'outil Timeframes par asset
      (décision 04/09) et mesurer l'effet (comparatif 24 mois = référence :
      M15 +0,051 R/trade)
- [ ] Tout réglage ne bouge que sur preuve ≥ 30 trades remplis par tranche
      (anti-overfitting)

### 3. Rôles IA — accumulation puis analyse (après gate 3)

- [ ] **Corrélation conviction × verdict** : ≥ 30 trades notés par la
      conviction à l'émission (accumulation en cours, asynchrone) → décision
      propriétaire sur un éventuel filtre — pas avant. L'IA note, elle ne
      filtre jamais (constitution).
- [x] **Analyse des passes straddle — FAIT le 08/09** : page d'analyse
      PLEIN ÉCRAN `/straddle/analyse` (les 3 modales d'analyse supprimées,
      plus d'onglets — architecture standard des analyses). Hiérarchie de
      lecture : 🎯 dossier de décision (ΣR · n/30 · WR · passes restantes en
      XL + tableau par source — annonces 14h30 +4,84 R sur 5 vs ouvertures
      DAX −1,36 R sur 7 — + phrase de l'analyste), 🤖 avis complet
      (forts/faibles/propositions priorisées/confiance), damier détails +
      créneaux armés, paramètres. **Fraîcheur 2 vitesses** : chiffres
      calculés EN DIRECT à chaque consultation (une passe clôturée apparaît
      aussitôt), texte LLM en cache du jour (boot + premier accès + ↻ ;
      table `analyse_cache`, migration 0108). Prompt éditable
      `straddle_analyste`. Bug legacy corrigé au passage (`useStraddleStats`
      comparait stratégie/verdicts en majuscules — l'ancienne modale
      affichait des zéros depuis toujours ; verdict ts désormais compté
      gagnant).
- [ ] **Voile des setups SMC** : l'analyste lit annonces vs confirmés vs
      dissipés (avec `smc_scoring_detail`, journalisé depuis le 07/09) et
      identifie les caractéristiques des setups qui tiennent → décision sur
      preuve d'un éventuel filtre temps réel
- [x] *Recommandation agenda + minutage* — couvert par l'agenda intelligent
      §16 (créneaux statistiques notés par l'analyste, armement propriétaire)

### 4. Rockets — Extensions

- [ ] **Véto unlocks** : source libre (calendrier public de déverrouillages
      de tokens) → intégrer au scanner (éliminatoire si unlock majeur < 30 j)
- [ ] **ETF via Tiingo** : lever l'exclusion ETF + profils 2/3/4 % dédiés —
      après validation de l'Observation actions US
- [ ] **Analyse par pilier** : l'analyste relie les critères du /10 aux
      verdicts → propositions de recalibrage chiffrées

### 5. Exécution réelle — la prochaine frontière (§15)

L'app observe, mesure, informe — elle ne passe pas d'ordres. Le miroir MQL5
est validé ; c'est le saut qualitatif.

- [ ] **Décision de principe propriétaire** : un EA exécutant recevant
      lots/niveaux (mode « ordres auto-validés » compatible constitution :
      l'IA ne décide jamais, le moteur déterministe exécute ce qu'il signale
      déjà). Leçon §3 : l'EA exécutant doit consommer le **feed du
      collecteur** (même donnée que les moteurs = écart 0 — la base de ticks
      du Strategy Tester n'est pas représentative)
- [ ] Si acté : cahier des charges (périmètre SMC d'abord, validation
      humaine par trade au démarrage, garde-fous — risque max/jour,
      kill-switch, journal d'ordres), puis EA exécutant + backtest avant
      tout réel. NB : la couche ordres actuelle de l'EA (ordres au marché,
      sauts `lot<=0`, pas de partielles) est à reprendre — la garde absolue
      `MQL_TESTER` la neutralise en attendant

### 6. Finitions (fin de développement)

- [ ] **Transposer l'architecture des pages d'analyse** (dossier de
      décision + avis + damier, cf. `/straddle/analyse`) aux pages
      `/smc/analyse` et `/rockets/analyse` — décidé le 08/09, « on verra
      plus tard »
- [ ] **Relecture finale des prompts IA** : re-passée complète contre les
      mécaniques figées ; purge des prompts morts (`smc_signal`,
      `rockets_opportunites`) ; cohérence constitution ; formats JSON
      robustes ; ancrage conventions ($ réels composés, R pondéré/net)
- [ ] **Export du rapport d'activité** (PDF/Markdown)
- [ ] **ETH** : réactiver et re-backfiller si souhaité (décision propriétaire)
- [ ] **Pine dans TradingView** (action propriétaire) : coller le Pine de
      `docs/reference/` sous « Scalp à Nono » — vérification visuelle des
      zones contre l'app

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
