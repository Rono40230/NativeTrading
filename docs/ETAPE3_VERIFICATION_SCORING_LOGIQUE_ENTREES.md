# ÉTAPE 3 — Vérification du scoring, de la logique SMC et de l'entrée en position

> Feuille de route propriétaire — étape 3, réalisée le 29/08/2026.
> Méthode : cartographie exacte du moteur (Pine étalon + Rust) → recherche web
> sur 4 axes → analyse croisée → recommandations hiérarchisées (replay d'abord,
> règle des 30 trades — aucune modification sans preuve).

Sources principales :
- Scoring/checklist : [fxnx — SMC Trading Checklist](https://fxnx.com/en/blog/smc-trading-checklist-score-setup-before-you-enter) · [TradingView scripts SMC-ICT](https://fr.tradingview.com/scripts/smc-ict/) (concept de « confluence score = somme transparente de conditions pondérées »)
- Méthodologie canonique : [TradingWyckoff — SMC Complete Guide](https://tradingwyckoff.com/en/smart-money-concepts/) · [ICT Top Down Analysis](https://innercircletrader.net/tutorials/ict-top-down-analysis/) · [TradingStrategyGuides Day 10](https://tradingstrategyguides.com/day-10-multi-timeframe-analysis-ict-smc-the-top-down-approach-explained/)
- Entrées : [FluxCharts — Order Blocks](https://www.fluxcharts.com/articles/order-blocks-ob-explained) · [InnerCircleTrader.net — Order Block](https://innercircletrader.net/tutorials/ict-order-block/) · [LuxAlgo — Silver Bullet](https://luxalgo.com/blog/ict-silver-bullet-setup-trading-methods/) · [Backtrex](https://backtrex.com/en/blog/ict-silver-bullet-strategy-trading-guide)
- MTF : [PineGen — repaint/lookahead au niveau du code](https://rangatechnologies.medium.com/how-pinegen-ai-handles-repainting-and-look-ahead-bias-at-the-code-level-3f49620e358d) · [Medium — The Multi-Timeframe Edge](https://medium.com/forex-champs/the-multi-timeframe-edge-how-top-down-analysis-transforms-your-trading-accuracy-f33e4d240435)

---

## 1. État du moteur (référence de comparaison)

**Scoring v11 (`f_score`, 16 composantes)** — poids calibrés par replay WR
(campagnes P1-P11 + FINAL), par asset :

| Composante | Poids | Calibrage documenté |
|---|---|---|
| BOS directionnel | 1-6 dynamique (corps vs ATR) | P1.1/P5.2 — impulsion institutionnelle |
| FVG | 3-5 (BTC 3, métaux 5) | P8 — fiable métaux, peu BTC |
| Sweep frais | 1-4 | P8 — pépite XAU/DAX, neutre XAG/BTC |
| MSS | +3 | P7 — rare 6%, WR 80-82 |
| CHOCH | +4 | P7 — rare 6%, WR 80-82 |
| ATR impulsion | +2 | P1.5 — caractéristique, pas confluence |
| Confluence H1 / H4 / W1 / MN | +1 / +4 / +5 / +6 | P7 — H1 présent 80% WR ≤ global → 1 |
| Imbalance | +3 | — |
| OTE | 2-5 | P8 — instit XAU/DAX, neutre XAG/BTC |
| Kill Zone | 2-3 | P7 — présent 63%, WR neutre → 4→3 |
| prevLiq proximité / sweep | +2 / +4 | — |
| Premium/Discount | +1 | P7 — présent 78%, WR < global → 2→1 |
| Mega-orders (volume ≥ 2× SMA20) | +2 | Phase 5 28/08 — +21.3R replay |
| **Gardes** | plafond 8 si BOS seul ; score 0 si asset non reconnu | P1.2 anti-bruit / Phase 4.1 |
| **Qualification** | `seuilTrade = 0` + **force ≥ 4/10** + znQual (FVG sur zone + DoL au-delà) | campagne forceMin 19/07 : F4 domine F5/F6 |

Bandes par asset : XAU 7/10/12 · XAG 8/99/99 (Moyen-only) · NAS/SPX 10/15/17 ·
DAX 11/16/19 · BTC 8/99/99.

**Entrée en position** — modèle « **Retest (limite)** » (gagnant A/B 15/15) :
TR forcée au **bord de la zone** (proximal : `zone_top` bull / `zone_bot`
bear), fill au retest géré par le lifecycle. SL = bord opposé ± offset ATR
selon `_autoSlMode`, clampé `[_slMin, _slMax]` (0.5-2.5×ATR par asset).
TP1 = 1R · TP2 = 2R · TP3 = DoL plafonné 3R (décision 28/08). Expiration par
TF. Un trade max par bar + `trade_bloquant`.

**Confirmation MTF (MODULE 12)** — `f_htf(swing)` sur H1/H4/W1/MN via
`request.security(lookahead_off)` : pivots → BOS → 3 OB bull + 3 OB bear par
TF. Drapeau `confluenceH*` = close LTF **dans n'importe laquelle** des 6
zones du TF. Scoring : `+N si drapeau ET un OB du sens du trade EXISTE`
(présence, pas containment directionnel). **Repaint documenté et assumé**
(Phase 3.4) : la bougie HTF en formation peut déplacer zones/trend en live —
choix de réactivité pour le scalping intrabar. Moteur B (BSZones) : gate
`baseScore ≥ 6 ET (H1 ou H4 aligné)` — une vraie porte MTF canonique.

---

## 2. Objectif 1 — Pondération des indicateurs (recherche)

**Ce que fait le web** : la référence la plus structurée (fxnx) score sur 10 :
**5 facteurs × 2 points à poids égal** (killzone, premium/discount,
displacement+FVG, sweep, DoL dégagée), **seuil 7/10**, et surtout :
- **Invalidateurs fatals** (score → 0) : fenêtre macro ±15 min, OB/FVG HTF
  opposé devant la cible, **inducement non consommé** sous l'entrée ;
- **Risque gradué par note** : A (9-10) = risque plein, B (7-8) = demi-risque,
  C (≤6) = on passe.
Les indicateurs TradingView SMC popularisent le « confluence score = somme
transparente de conditions pondérées » ; ICT enseigne une checklist de 9
confluences sans poids chiffrés.

**Analyse croisée** :
- Notre pondération est **plus sophistiquée que le canon public** : poids
  empiriques par asset issus de replays (WR par composante, campagnes
  documentées), là où le web reste à poids égal ou à des checklist binaires.
  Les trois campagnes de la journée (BPR +1.0R = bruit, sessions 0.0R,
  mega-orders +21.3R) confirment que la structure additive **sature** : un
  bonus marginal ne franchit que rarement les seuils.
- Ce qui nous manque vs le web : **(a) les invalidateurs fatals** (nous
  n'avons que des bonus/malus gradués — l'inducement notamment n'est pas
  modélisé alors que EQH/EQL existent dans le moteur), **(b) le risque
  gradué par force** (notre lot est à risque fixe quelle que soit la force),
  **(c) un compteur minimum de confluences** (canon : ≥4/6 — nous, seuil de
  force, équivalent fonctionnel).

**Verdict** : pertinence du scoring **validée** (calibrée sur données
propres, règle des 30 trades respectée). Pistes d'emprunt : invalidateur
fatal « inducement » et risque gradué par force (cf. §6).

> **DÉCISION PROPRIÉTAIRE (29/08)** : la pondération des 16 composantes est
> **FIGÉE** — rien à faire de ce côté. Les évolutions éventuelles passeront
> exclusivement par des portes de qualification (R1, R2, R6) ou de
> l'exécution (R7), jamais par une retouche des poids.

---

## 3. Objectif 2 — Méthodologie SMC canonique vs moteur

Flux canonique (TradingWyckoff/ICT, 7 étapes) vs moteur :

| Étape canonique | Statut canon | Moteur v12 | Écart |
|---|---|---|---|
| 1. Biais HTF D1/H4, **skip si contradiction** | porte dure | Confluences scorées (+1/+4/+5/+6) ; **pas de D1** ; gate HTF seulement côté BSZones | ⚠️ partiel |
| 2. **Sweep de liquidité externe = PRÉREQUIS** (« pas de sweep, pas de trade ») | porte dure | Sweep scoré +1..4, jamais exigé | ⚠️ divergence majeure |
| 3. CHoCH (mou) puis MSS (displacement) — **séquence** | séquence | Les deux scorés (+4/+3), indépendants | ⚠️ partiel |
| 4. OB du CHoCH : dernière bougie opposée + displacement + BOS | définition | Détecteur OB = ROC + impulsion + imbalance[1], lifecycle 3 états | ✅ fidèle |
| 5. Premium/Discount + OTE 62-79% — **« jamais acheter en premium »** | porte dure | OTE scoré 2-5 ; P/D **+1 seulement** | ⚠️ divergence notable |
| 6. Killzones — hors fenêtre ≈ nul | porte quasi-dure | KZ scoré +2-3 (dead-zone NY lunch REJETÉE par replay Module C — WR neutre global) | ✅ notre preuve > le canon |
| 7. Exécution (cf. §4), SL distal, TPs échelonnés vers le DOL | — | SL bord opposé + clamp ATR ; TP1 1R / TP2 2R / **TP3 = DoL** | ✅ (TP2 simplifié) |
| Cycle ERL→IRL→ERL, DOL comme cible | cœur | TP3 DoL≤3R + znQual DoL | ✅ |
| Inducement (IDM) | concept clé | Non modélisé (EQH/EQL détectés, pas consommés comme IDM) | ❌ absent |

**Lecture d'ensemble** : le moteur est un **SMC additif calibré par replay**,
le canon est un **SMC à portes séquentielles**. Notre culture d'études a déjà
tranché dans un sens (Module C/D rejetés : les portes globales pénalisent des
gagnants), MAIS les deux portes les plus centrales du canon n'ont jamais été
testées telles quelles : **sweep obligatoire** et **P/D directionnel en
qualification de zone**. Ce sont les candidates prioritaires (cf. §5) — avec
la nuance ICT que le sweep peut être « récent » (≤ N bars) et pas strictement
contemporain du signal.

---

## 4. Objectif 3 — Meilleures méthodes d'entrée en position

**Ce que dit le web** : trois familles —
1. **Agressive** : limite au bord proximal de l'OB (meilleur prix/R:R,
   fiabilité la plus basse — fills à l'aveugle, wicks) ;
2. **Conservatrice** : bougie de rejet dans la zone puis stop-order ;
3. **Confirmation LTF** (recommandée par les guides ICT) : descendre en
   M5/M1, attendre un CHoCH/MSS/CISD **dans** la zone, entrer au retest du
   mini-OB/FVG résultant.

Données publiques : **aucun head-to-head rigoureux** ; les chiffres qui
circulent (Silver Bullet 55-80% revendiqué ; -46% sur un backtest 10 ans)
montrent une variance énorme selon les filtres — « le win rate dépend bien
plus des filtres que du type d'ordre ». Les tutoriels InnerCircleTrader
prênnent l'hybride : limite au retest de l'OB **+** MSS/CISD en 5m/15m.

**Analyse croisée** : notre « Retest (limite) » est **précisément la méthode
1** — choisie non par dogme mais par **A/B replay interne 15/15**, preuve
plus solide que tout ce que le web offre sur la question. Le canon
suggère néanmoins une évolution naturelle **replayable** : le mode 3 hybride
(fill seulement si un shift LTF survient dans la zone avant sa mort) —
même architecture d'étude que le BE forcé / TP3 / BPR (deux branches, règle
des 30 trades).

Sur les sorties : TP1 1R ≈ canon (swing ~1:1) ✅ ; TP3 DoL ≤ 3R =
canon (DOL) ✅ ; TP2 = 2R fixe là où le canon vise le **FVG intermédiaire** —
divergence mineure, déjà tranchée par DoL≤3R (+61.5R).

---

## 5. Objectif 4 — Confirmation MTF du moteur (analyse interne)

**Mécanique exacte** (Pine MODULE 12 / Rust `mtf.rs`, parité vérifiée) :
- Par TF (H1/H4/W1/MN) : pivots swing → dernier BOS → 3 derniers OB bull +
  3 bear, rafraîchis en continu ;
- `confluence*` = le close LTF est **dans l'une des 6 zones** du TF
  (indépendamment du sens) ;
- Le score ajoute +N si drapeau **et** un OB du sens du trade existe —
  subtilité : l'existence, pas le containment directionnel (le prix peut
  être dans l'OB bear H4 pendant qu'un OB bull H4 existe ailleurs → +4 quand
  même pour un trade bull). **Impureté logique réelle mais d'ampleur
  limitée** (les 6 zones se chevauchent rarement toutes en même temps).

**Repaint** : évaluation live de la bougie HTF en formation (choix Phase
3.4, réactivité scalping). La meilleure pratique du web (PineGen, Freqtrade)
recommande de n'agir que sur **HTF clôturé** — le repaint HTF est classé
« tueur silencieux » des backtests. Notre replay agrège les HTF depuis les
clôtures LTF : il reproduit donc le comportement live, pas un faux
historique — mais le **coût du repaint n'a jamais été mesuré** (combien de
confluences disparaissent à la clôture HTF ?).

**Pondérations** : H1 +1 (P7 : présent 80%, WR ≤ global), H4 +4, W1 +5,
MN +6 — hiérarchie conforme au canon (l'HTF a autorité), calibrée sur
données. Le web avance que l'alignement MTF fait passer ~50% → 65-70% de WR
sur des stratégies mono-TF — cohérent avec nos poids croissants.

**BSZones** : la gate `baseScore ≥ 6 ET (H1 ou H4 aligné)` est la seule
vraie **porte** MTF du moteur — c'est elle qui explique une partie de la
complémentarité v11/BS observée dans les replays.

---

## 6. Recommandations hiérarchisées (replay d'abord — rien sans preuve)

| # | Piste | Type | Effort attendu | Justification |
|---|---|---|---|---|
| **R1** | **Sweep obligatoire (ou ≤ N bars) pour qualifier un trade** | porte + étude 2 branches | 1 session | Étape 2 du canon = prérequis absolu ; jamais testée telle quelle ; Modules C/D n'y répondent pas |
| **R2** | **P/D directionnel en znQual** (interdire long en premium / short en discount à la qualification) | porte + étude | ½ session | Canon : « jamais acheter en premium » ; nous : +1 symbolique |
| **R3** | **Entrée confirmation LTF** (ModeEntree : RetestLimite vs Confirmation-MSS-dans-la-zone) | étude A/B lifecycle | 1 session | Méthode 3 du canon, recommandée par les guides ; notre A/B 15/15 n'a comparé que des variantes de limite |
| **R4** | **Confluence MTF sur HTF clôturé** (mesurer le coût du repaint : 2 branches) | étude | ½ session | Meilleure pratique anti-lookahead ; choix actuel assumé mais jamais mesuré |
| R5 | Containment **directionnel** MTF (le prix dans un OB HTF du sens du trade, pas juste existence) | correctif scoring + étude | ½ session | Impureté logique identifiée §5 |
| R6 | Invalideur fatal **inducement** (EQH/EQL non consommés sous l'entrée → pas de trade) | porte + étude | 1 session | Concept clé canon absent ; EQH/EQL déjà détectés (brique disponible) |
| R7 | **Risque gradué par force** (F≥7 → risque plein, F4-6 → demi-risque) | exécution + étude | ½ session | Modèle fxnx (grades A/B) ; s'interface avec l'onglet gestion du risque |
| R8 | **Biais D1 comme gate MTF supérieure** (skip jour si contradiction H4/D1) | porte + étude | ½ session | Étape 1 canon ; D1 déjà en base (amorce MN) |

Toutes passent par la discipline établie : Pine étalon d'abord → portage
Rust derrière drapeau → `comparatif_*` 6 assets × M1/M5/M15 → règle des
30 trades → décision propriétaire documentée. Les leçons du 28/08
(BPR, sessions) rappellent que l'attendu de beaucoup de ces portes est
**faible** — c'est précisément ce que l'étude dira.

---

## Conclusion

- **Scoring** : plus calibré que le canon public (poids empiriques par asset) ;
  structure additive saturée — le levier n'est plus dans les bonus marginaux
  mais dans les **portes** (R1, R2, R6).
- **Logique SMC** : fidèle sur l'ossature (OB/FVG/liquidités/DoT/KZ/PO3 via
  sessions) ; divergences assumées et documentées là où nos replays ont
  tranché (dead-zone, TP2 fixe) ; deux portes canoniques jamais testées.
- **Entrée** : « Retest limite » validé par A/B 15/15 — meilleure preuve
  disponible ; la confirmation LTF (R3) est la seule évolution canonique
  testable.
- **MTF** : mécanique saine et hiérarchie pondérée conforme au canon ;
  repaint assumé non mesuré (R4) et impureté directionnelle mineure (R5).
