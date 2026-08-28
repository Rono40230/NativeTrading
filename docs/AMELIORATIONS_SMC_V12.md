# AMÉLIORATIONS SMC V12 — Audit ICT et roadmap d'implémentation

> État au 28 août 2026. Source : audit complet des 27 indicateurs du Pine v12
> contre les standards SMC/ICT, croisé avec les propositions externes
> (`docs/reference/ameliorations_proposees_v12.md`).
>
> **Constitution** : le Pine est l'étalon. Toute amélioration est portée d'abord
> dans le Pine, puis clonée en Rust, puis affichée. Chaque changement de
> stratégie est immédiatement répercuté dans les prompts IA.

---

## Contexte

L'audit du 28/08 a comparé les 27 indicateurs du v12 contre les standards
SMC/ICT (LuxAlgo, InnerCircleTrader, FibAlgo, DailyPriceAction, FluxCharts).
Résultat : 22 parfaitement conformes, 4 améliorables, 1 correction visuelle
(adelivrée — Premium/Discount horizontal, CE des gaps, Breaker avec sweep).

Les 10 propositions externes ont été analysées : **5 validées, 3 invalidées,
2 avec réserves**.

---

## Décisions de validation

| ID | Module | Verdict | Justification |
|---|---|---|---|
| **A** | BPR (Balanced Price Range) | ✅ Retenu | Vrai concept ICT — chevauchement FVG bull/bear = zone d'équilibre institutionnelle. Nouveau module de scoring. |
| **B** | Scoring des Gaps NDOG/NWOG | ❌ Rejeté | **Déjà testé et retiré le 19/07/2026** (campagne FINAL, jamais positif en BT). La constitution exige 30 trades pour recalibrer. |
| **C** | Dead Zone (NY Lunch) | ⚠️ Malus, pas blocage | Concept légitime (NY Lunch = volatilité morte). Mais le blocage dur (`return`) est agressif — **préférer un malus de scoring (-3 pts)**, validé par replay. |
| **D** | Filtre de Régime (Range/Trend) | ✅ Retenu (sous conditions) | Le MODULE 12b existe dans le Pine mais est désactivé. **Comprendre pourquoi avant de réactiver.** |
| **E** | Filtre RSI | ❌ Rejeté | **RSI n'est pas un concept SMC/ICT.** Contredit la philosophie du moteur (« clone fidèle Pine SMC pur »). |
| **F** | Scoring Sessions H/L | ✅ Retenu | Cohérent avec la liquidité ICT — les H/L de session sont des cibles BSL/SSL. Données déjà présentes (MODULE 14). |
| **G** | DoL comme TP3 dynamique | ✅ Appliqué 28/08 — **DoL plafonné à 3R** | Replay 24 mois : DoL pur **coûtait 67R** (liquidité souvent > 3R, inatteignable avant expire). DoL≤3R = +61.5R vs production précédente. Pine + Rust + prompt synchronisés. |
| **H** | Mega-Orders (volume 3×) | ⚠️ Retenu à calibrer | Concept valable mais 3× SMA20 trop strict. **Calibrer à 2× et tester en replay.** |
| **I** | OFI (Order Flow Imbalance) | ❌ Rejeté | Approximation grossière (`buyVol = volume × (close-low)/(high-low)` suppose distribution uniforme — faux). Complexité maximale pour un signal douteux. Pas ICT authentique. |
| **J** | Alerte composite BPR | ✅ Retenu (faible) | Dépend du module A. Confort uniquement. |

---

## Corrections déjà livrées (28/08 — commit `e9141b2`)

- [x] Premium/Discount horizontal (3 lignes : limite Premium, EQ, limite Discount)
- [x] CE (Consequent Encroachment) des gaps NDOG/NWOG — ligne à 50% du gap
- [x] Breaker avec sweep préalable obligatoire (≤ 10 bars)

---

## Phase 1 — Validation préalable ✅ TERMINÉE (28/08)

### Module D (filtre régime) → ❌ REJETÉ
Le Pine documente (ligne 2429) : « jamais testé en BT, décidé campagne FINAL 2026-07-19 ». Le code a été entièrement retiré (pas juste désactivé). L'OTE-only voisin a été supprimé dans la même campagne pour dégradation prouvée du PF. Ne pas réactiver sans preuve.

### Module C (dead zone NY Lunch) → ❌ REJETÉ comme règle globale
Replay 24 mois (6 assets × M1/M5/M15) : le NY Lunch n'est mortel que sur BTC M1 (WR 27% vs 51%). Sur XAU M15 et NAS M15, le lunch est neutre (50% WR) voire meilleur. Une règle globale pénaliserait des trades gagnants. Échantillon lunch BTC M1 = 26 trades (sous le seuil de 30). Supprimé de la roadmap.

### Module H (mega-orders) → ⚠️ DONNÉES INSUFFISANTES
Le volume n'est pas jointif avec le journal des trades (timestamps non alignés entre `runtime_emissions` et `bougies`). À réévaluer en Phase 5 avec un replay dédié volume-corrigé. Reporter.

### Modules restants après Phase 1
- **G (DoL TP3)** — priorité maximale
- **A (BPR)** — phase 3
- **F (sessions H/L)** — phase 4
- H — en attente (Phase 5)

## Phase 2 — Module G : DoL comme TP3 ✅ TERMINÉE (28/08) — DoL≤3R APPLIQUÉ en production

### Constat initial : le module était DÉJÀ en production
Le TP3-DoL existe déjà dans le Pine étalon (`_bsDolTarget` ligne 3294 + TP3 v11
lignes 3556-3574) ET dans le moteur Rust (`nearest_liq`, commit « décisions
trading »). La proposition externe partait d'une version antérieure du Pine.

**Correctif de parité livré au passage** : le TP3 des trades BSZones incluait à
tort l'Asian High/Low côté Rust — le Pine ne l'inclut que pour les trades v11
(`_tAHH3`), pas dans `_bsDolTarget`. Aligné sur l'étalon (+ test dédié).

### Étude replay — 3 modes × 6 assets × M1/M5/M15 (≈ 14-24 mois, BE = Supprimé)
Binaire : `comparatif_tp3` · Sortie : `data/comparatif_tp3.txt`

| Mode | R total | R moyen | TP3 touchés | TP2+BE | max DD |
|---|---|---|---|---|---|
| **DoL** (production actuelle) | +704.6R | +0.254 | 95 | 316 | 12.0R |
| **Fixe3R** (contre-factuel) | **+772.0R** | **+0.278** | 186 | 226 | 12.0R |
| **DoL≤3R** (plafonné) | +766.1R | +0.276 | **195** | 217 | 12.0R |

### Lecture des chiffres — le DoL pur COÛTE ~67R
Paradoxe apparent : le DoL (cible plus « intelligente ») produit MOINS de TP3
que le 3R fixe (95 vs 186). Explication : la liquidité la plus proche (EQH/PDH/
PWH) est généralement AU-DELÀ de 3R (surtout M1/M5 où le R clampé ATR est
petit) → le prix n'atteint jamais la cible avant l'expire → le trade retombe
en TP2+BE au lieu de TP3. Les 91 bascules TP2+BE→TP3 (≈ +1R chacune) font
presque tout le delta.

Le plafonnement `DoL≤3R` (liquidité si elle est PLUS PROCHE que 3R, sinon 3R)
récupère +61.5R : il garde la logique ICT (cibler la liquidité atteignable)
sans courir après des cibles inatteignables. Fixe3R pur vs DoL≤3R :
5.9R d'écart sur 2 776 trades = bruit statistique.

### Décision VALIDÉE et APPLIQUÉE (28/08) — DoL≤3R
1. ✅ **Pine étalon** : TP3 = `min(DoL, entry±3R)` dans les 4 fonctions de création
   (v11 BUY/SELL `_tp3 := _m3` plafonné, BS BUY/SELL `_bsDolTarget` plafonné ;
   le garde-fou `si TP3 casse la monotonie → 3R` est conservé)
2. ✅ **Rust** : défaut production `ModeTp3::DolCappe3R` dans `SignalGenerator::new()`
   + test dédié (`build_levels_tp3_dol_plafonne_3r_par_defaut`)
3. ✅ **Prompt IA** : « TP3 = liquidité la plus proche plafonnée à 3R »
4. Alternative Fixe3R pur (+5.9R de plus) : statistiquement indistinguable,
   mais abandonne toute logique de liquidité — non retenue.

Échantillon de validation : 2 776 trades (règle des 30 largement respectée).
Attendu en production : +61.5R vs l'ancien DoL pur sur fenêtre équivalente.

### Constat annexe — SP500 muet
SP500 = 0 signal SMC sur toutes les fenêtres replay (18 mois). Les 5 autres
assets produisent 69-371 trades par fenêtre. À investiguer en Phase 5
(calibration ? zones jamais qualifiées ?).

## Phase 3 — Module A : BPR (2 sessions)

### Session A — Pine étalon ✅ LIVRÉE (28/08) — validation visuelle en attente

**Références** (recherche du 28/08) :
- **ICT mentorship 2023** (épisodes Power of Three / market reviews) : BPR = zone de chevauchement de 2 FVG opposés, typiquement sur spike news ; entrée au bord proche dans le sens du biais
- **LuxAlgo — concept BPR** : intersection STRICTE (pas l'union) ; le déplacement le plus RÉCENT fixe le rôle (gap bull récent sur gap bear = support, miroir = résistance) ; invalidation = clôture au-delà du bord lointain
- **LuxAlgo — indicateur BPR** : fenêtre d'appariement 10 bars (défaut), « only the most recent overlapping gap is paired », midline pointillée, trigger de violation = Close

**MODULE 6b livré dans le Pine étalon** :
- Détection : à la naissance d'un FVG, appariement avec le FVG opposé **le plus récent** né dans les **10 bars** qui chevauche → BPR = intersection stricte. Placée AVANT les lifecycle FVG (le gap opposé est apparié même si sa clôture de remplissage le retire du pool : « les deux gaps sont délivrés »)
- Rôle = sens du gap le plus récent (LuxAlgo/ICT)
- Lifecycle : 0 frais → 1 partiel (prix entré, CE intacte) → 2 profond (CE atteinte) → **figée** (clôture au-delà du bord lointain, trigger Close) · âge max actif 15 bars
- Affichage : box ambre `#FFB300` (haussier/support) / orange `#FF6D00` (baissier/résistance), label « BPR », **CE pointillée** (midline LuxAlgo), atténuée au state 1/2, **figée en gris** à sa mort (grisonnement + hors scoring — l'étendue continue jusqu'à la bougie en cours comme les autres blocs, décision propriétaire 28/08), FIFO 20 (« Show Last 20 » LuxAlgo), toggle ON par défaut
- Scoring (la qualification, PAS le f_score de bar) : `f_bprBonus` +4 frais / +3 partiel / +1 profond si chevauchement avec BPR **active** de même sens — greffé aux 4 points : accumulation OB bull/bear + `_dynS` BSZones bull/bear. Dénominateur `_toForce10` (27) inchangé — recalibration en Phase 5.

**Correctif invisibilité (28/08, retour propriétaire)** : première version supprimait les boxes à leur mort. Or le replay montre : ~1 BPR / 116 bars, durée de vie ≈ 9 bars, **toutes** mortes par clôture au travers → espérance de boxes vivantes à un instant donné ≈ 0,08 → graphique vide 92% du temps. Correctif conforme LuxAlgo : les zones mortes sont **figées (grisées), conservées à l'affichage** jusqu'à l'éviction FIFO 20, hors scoring. Sonde `probe_bpr` (Rust, DB réelle) : 4-8 boxes visibles par fenêtre de 500 bars.

**À valider par le propriétaire dans TradingView** (charger `docs/reference/smc_indicateur_v12.pine`) :
- [x] Boxes BPR aux bons endroits (validation croisée 4 UT : zone structurelle ~7720-7742 SP500 détectée sur plusieurs UT, UT courte sans gap bull = 0 BPR — cohérent règles)
- [x] Rôle cohérent (capture UT haute : BPR posé sur FVG vert retesté par le prix — cas d'école ICT)
- [x] Boxes s'arrêtant à la bougie en cours (correctif double du 28/08 : bord droit = `bar_index` dès la naissance + bloc d'extension sorti du garde « vivant » — les figées étaient jamais ramenées et débordaient)

**Validation visuelle ✅ (28/08, retour propriétaire « c'est ok »).**

### Session B — Rust + frontend + replay ✅ LIVRÉE (28/08)

- [x] **Moteur Rust** : `smc/src/v12/bpr.rs` — détecteur fidèle (fenêtre 10, intersection stricte, plus récent d'abord, anti-doublon ≥80% sur ACTIFS, FIFO 20, âge 15, états sticky 0/1/2, figé=dead conservé). 12 tests unitaires. Point subtil de parité : l'appariement lit les pools FVG **pré-lifecycle** (snapshot pris par le moteur avant `fvg.update` — le Pine apparie avant `f_fvg*BearLifecycle`)
- [x] **Scoring** : `bonus_bpr()` (+4/+3/+1) greffé aux 4 points paritaires — accumulation OB v11 bull/bear (`scoring_v11.rs`, Pine 2504/2520) + `_dynS` BSZones naissance/lifecycle (`scoring_bs_zones.rs`, Pine 3602/3648). Drapeau `avec_scoring_bpr(bool)` pour le contre-factuel
- [x] **Frontend** : `BprV12` (api.smc.ts) + endpoint `bprs` (BprOut : top/bot/ce/state/dead) + rendu overlay ambre/orange, CE pointillée, figées grisées + toggle « BPR » (défaut ON, IndicatorPanel). Split `smcV12OverlayExtraLignes.ts` (règle < 600 lignes)
- [x] **Replay comparatif** `comparatif_bpr` : BPR ON vs OFF, BE=Supprimé + TP3=DoL≤3R (production), 6 assets × M1/M5/M15, ticks simulés → `data/comparatif_bpr.txt`

#### Résultats (28/08) — le bonus est un BRUIT, décision : scoring RETIRÉ

| Branche | R total | Clôtures | R moyen | max DD |
|---|---|---|---|---|
| **BPR ON** (bonus actif) | +763.1R | 2 834 | +0.269 | 11.0R |
| **BPR OFF** (contre-factuel) | +762.1R | 2 770 | +0.275 | 12.0R |

Delta = **+1.0R sur ~2 800 trades** (0,03% du total) : bruit statistique. Deltas
par cellule entre -5R (XAU M15) et +4R (BTC M5), non corrélés entre assets. Le
bonus produit 64 trades supplémentaires (zones franchissant les seuils de
qualification force ≥ 4 / score ≥ 7) au R moyen légèrement inférieur.

**Décision appliquée (règle pré-validée)** :
1. ✅ **Pine étalon** : `f_bprBonus` + ses 4 points de greffe retirés — la détection, le lifecycle et l'affichage (MODULE 6b) demeurent intacts, avec commentaire de décision à l'emplacement du bonus
2. ✅ **Rust** : défaut `bpr_scoring = false` — les greffons restent en place, ré-activables par `avec_scoring_bpr(true)` pour une future ré-étude (le bonus reste hors production tant qu'une nouvelle étude ne le justifie pas)
3. ✅ **Affichage conservé partout** : Pine + endpoint `bprs` + overlay frontend (zones ambre/orange actives, grises figées, CE pointillée) — valeur d'analyse visuelle intacte
4. SP500 : muet dans les deux branches (8 vs 0 trades) — confirmé pour investigation Phase 5 ; DAX M1 sous le seuil 30 (24 trades, identique dans les deux branches)

## Phase 4 — Module F : Sessions H/L ✅ TERMINÉE (28/08) — bonus RETIRÉ

**Livré** :
- **Pine étalon** : MODULE 14b — London High/Low (08:00-16:30 Paris, mêmes constantes `SES_PARIS_LONDON*` ; range pendant session → drawn à la fin → consommé à l'atteinte ; affichage `i_showLondonHL` défaut false). Greffon f_score +2 proximité (0.35×ATR, état N-1 — les drawn déclarés avant f_score car Pine exécute top-down) — **retiré après étude** (commentaire de décision en place)
- **Rust** : `asian_hl.rs` généralisé (fenêtre paramétrable `avec_fenetre`) + instance Londres dans le moteur + `SessHlLevels` (état N-1, parité f_score Pine) + greffon `sess_hl_near` dans `live_score_detaille` derrière `avec_scoring_sessions(bool)` (défaut inactif). 4 tests Londres + 1 test greffon
- **Études** : `comparatif_sessions` (ON vs OFF) + `probe_sessions` (sonde d'activation) → `data/comparatif_sessions.txt`

#### Résultats (28/08) — ON ≡ OFF bit-à-bit, décision : bonus RETIRÉ

| Branche | R total | Clôtures | R moyen | max DD |
|---|---|---|---|---|
| **SessHL ON** (bonus actif) | +756.1R | 2 771 | +0.273 | 12.0R |
| **SessHL OFF** (contre-factuel) | +756.1R | 2 771 | +0.273 | 12.0R |

**Zéro trade de différence** — mêmes carnets jusqu'aux verdicts. La sonde
`probe_sessions` écarte l'hypothèse du greffon mort : proximité vraie sur
~1-2% des bars (100-400 par 20 000) et 4-11 trades par cellule M15 nés en
fenêtre de proximité — le +2 s'appliquait mais n'a **jamais fait franchir un
seuil de qualification** (seuilTrade / force ≥ min). La garde anti-bruit
(plafond 8 sur BOS seul) peut aussi l'absorber.

**Décision appliquée (règle pré-validée, delta = 0 ≤ 0)** :
1. ✅ Pine : greffon + flags `nearSess*` + inputs `i_sessHl*` retirés, commentaire de décision à l'emplacement
2. ✅ Rust : défaut `sess_hl_scoring = false` — greffon conservé derrière le drapeau pour ré-étude
3. ✅ Conservé : tracking Londres MODULE 14b (Pine + Rust), affichage `i_showLondonHL`
4. ✅ Prompt IA inchangé — le comportement de production est identique d'avant/après Phase 4 (aucun trade changé)
5. Note méthodo : les échantillons des études bougent légèrement entre runs (DB alimentée en continu par les collecteurs — ~1-2 trades/heure d'écart entre l'étude BPR 19h05 et l'étude sessions 20h11)

## Phase 5 — Validation finale ✅ TERMINÉE (28/08)

### 1. SP500 muet → RÉSOLU (profil manquant)

**Cause racine** : SP500 absent de `_assetReconnu` (Pine ligne 45) et
`AssetCalibration` (Rust) depuis l'origine — le garde « asset non reconnu →
scoring 0 » (Phase 4.1) tuait les deux moteurs. Les données étaient saines
(49 454 bougies M15 / 24 mois, 0% volume zéro). Ce n'était NI une calibration
spécifique à découvrir, NI un problème de zones : le profil n'existait pas.

**Correctif** : profil miroir NAS100 (indices US CFD jumeaux) — Pine
`_isSPX` (SP500/SPX/US500) + Rust `is_spx` : pvLot 20, SL mode 1×ATR, seuils
force 10/15/17, scoreMax 19, SL clamp 0.5/1.5×ATR, TP3 max 30/120 min.

**Validation** (`validation_spx`, 24 mois × 3 TF) :

| TF | R total | Clôtures | Règle 30 |
|---|---|---|---|
| M15 | +7.0R | 109 | ✅ |
| M5 | +2.8R | 59 | ✅ |
| M1 | +6.0R | 9 | ⚠ (<30, informatif) |
| **Total** | **+15.8R** | **177** | R moyen +0.089 · max DD 11R |

Expectancy positive d'emblée sans calibration dédiée. Calibration spécifique
SP500 possible en étude future si justifiée (règle 30 trades).

### 2. Module H (mega-orders) → ✅ CONSERVÉ (+21.3R)

L'audit recommandait 2× SMA20 (le 3× d'origine trop strict) — le `_volScore`
BSZones l'appliquait déjà côté zones ; le Module H l'ajoute au f_score v11
(+2 si volume[1] ≥ 2× SMA20[1], même sémantique). Volume propre sur les
6 assets (0% de zéros).

**Étude** (`comparatif_mega`, production : BE=Supp, TP3=DoL≤3R, BPR/sessions off) :

| Branche | R total | Clôtures | R moyen | max DD |
|---|---|---|---|---|
| **Mega ON** (production) | **+791.2R** | 3 115 | +0.254 | **11.0R** |
| Mega OFF (contre-factuel) | +769.9R | 2 951 | +0.261 | 12.0R |

Delta = **+21.3R** (+2.8%) — 20× le bruit BPR (+1.0R), max DD amélioré.
Le bonus qualifie 164 trades supplémentaires d'expectancy nette positive.
Réserve documentée : concentration du delta (BTC M5 porte +10R à lui seul ;
sans lui : +11.3R, toujours positif ; somme des cellules ≥30 trades :
+17.3R). **Décision : conservé en production** (défauts actifs Pine + Rust).

### 3. Replay intégral — bilan du programme

| Configuration | R total | Clôtures | Commentaire |
|---|---|---|---|
| Avant programme (DoL pur, 5 assets — SP500 muet) | +704.6R | 2 776 | étude comparatif_tp3 |
| **Production actuelle** (DoL≤3R + SP500 + mega-orders, 6 assets) | **+791.2R** | 3 115 | branche Mega ON |

**+86.6R d'apport net documenté** sur fenêtres équivalentes : DoL≤3R (+61.5R,
Phase 2), SP500 débloqué (+15.8R), mega-orders (+21.3R), moins la
redistribution des échantillons (DB alimentée en continu entre les runs).
BPR et sessions H/L : affichage conservé, scoring neutre (retrait documenté).

### 4. Recalibration des pondérations → ❌ NON JUSTIFIÉE

Aucune étude de la campagne n'a montré de bande de force déréglée : les
verdicts par bande restent cohérents par asset, et recalibrer sur les deltas
cellulaires (BTC M5 +10R, NAS M15 -4R...) reviendrait à de l'overfitting sur
des échantillons de 50-400 trades. Décision : statuquo — la règle des 30
trades s'applique à toute future recalibration, idéalement ≥ 100 trades/bande.

### 5. Conformité prompt IA → ✅ VÉRIFIÉE

`smc_definition` décrit la stratégie génériquement (structure, zones, force
≥ 4/10, TP3 DoL≤3R, lifecycle sans BE forcé) — aucune énumération d'assets ni
de bonus de scoring : l'ajout de SP500 et du bonus mega-volume ne le
contredit pas. BPR/sessions n'y figuraient pas non plus (affichage/scoring
interne). **Aucun changement requis** (constitution respectée : le prompt
décrivait déjà la production résultante).

### 6. Paramètres recommandés par asset (documentés)

| Asset | Swing M15/H1 | SL clamp (×ATR) | Seuils force | TP3 max M15/H1 | Notes |
|---|---|---|---|---|---|
| XAUUSD | 3/5 | 0.5–1.5 | 7/10/12 | 60/240 min | métaux : FVG +5 |
| XAGUSD | 3/4 | 0.6–1.8 | 7/99/99 (Moyen-only) | 45/180 min | Fort/Instit désactivés |
| NAS100 | 3/5 | 0.5–1.5 | 10/15/17 | 30/120 min | — |
| **SP500** | 3/5 | 0.5–1.5 | 10/15/17 | 30/120 min | **miroir NAS (Phase 5)** |
| DAX | 3/5 | 0.5–1.5 | 11/16/19 | 30/120 min | seuils les plus exigeants |
| BTC | 3/5 | 0.8–2.5 | 8/99/99 (Moyen-only) | 90/360 min | SL 2×ATR |

Modules transverses actifs : BPR affiché (scoring neutre), London H/L tracké,
bonus f_score : prevLiq (+2/+4), premium/discount (+1), **mega-orders (+2)**.

---

## Bilan final du programme (28/08)

**5 phases, 10 propositions auditées → 3 changements de production validés par
replay** : DoL≤3R (+61.5R), SP500 débloqué (+15.8R), mega-orders (+21.3R).
**2 modules livrés en affichage sans scoring** (BPR, London H/L) et 2 bonus
retirés pour bruit mesuré (BPR +1.0R, sessions 0.0R). Recalibration refusée
(anti-overfitting). Prompt IA conforme. Le programme d'améliorations SMC v12
est **clos** — prochaine étape naturelle : surveillance production.
