# Rockets — Définition canonique (figée)

Sources : « Rocket Hunter » (© Rocket Trading — Arnaud Biegun, PDF archivé `07 - Rocket hunter.pdf`)
+ affinages validés le 24/08/2026, issus des canoniques momentum breakout
(Minervini VCP/Trend Template, O'Neil CANSLIM/tasse-avec-anse, Darvas,
Bollinger squeeze).

**Périmètre** : crypto uniquement à la naissance (scan top 100+ volume,
blacklist v1 conservée). Actions US prévues via MT5 (phase 5).
**Timeframes** : détection D1 ; surveillance du pivot en temps réel.

---

## LE CLASSEMENT — 10 points, 4 piliers, seuil d'élimination à 7

### FONDAMENTAL — 3 points (équivalents crypto validés)

| Critère (1 pt chacun) | Définition crypto |
|---|---|
| **Sentiment de marché** | BTC en tendance haussière D1 (BTC > MM50 > MM200, moyennes empilées MM50 > MM200) ET secteur du token en tendance (performance secteur > BTC sur 4 semaines ; dominance BTC baissière = rotation vers les alts) |
| **Contexte** | Sortie de large base + prise de position sur la 1ère base (2ème maximum) — inchangé, indépendant de la classe d'actif |
| **News catalyseur** | Catalyseur identifié : flux ETF nets positifs, annonce de listing, upgrade réseau, réglementation favorable (dans l'idéal) ET aucun déverrouillage de tokens dans les 15-30 prochains jours |

### VÉTO ÉLIMINATOIRE (quel que soit le classement)

Déverrouillage majeur (≥ 1-2 % de la supply flottante) dans les 30 prochains
jours → **ÉLIMINÉ**. Fondement : étude Keyrock (16 000+ unlocks) — 90 %
créent une pression vendeuse, impact démarrant ~30 jours avant l'événement.

### TECHNIQUE — 3 points

- **Tendance (1 pt)** : Prix > MM50 ET Prix > MM200, MM50 > MM200 (empilement
  complet), prix à moins de 25 % de son plus haut 52 semaines.
- **Volatilité (1 pt)** : Phase 1 = compression longue avec largeur de
  Bollinger à son plus bas des 30 derniers jours ; Phase 2 = divergence des
  bandes (la largeur se remet à croître).
- **Intérêt (1 pt)** : volumes qui s'effondrent pendant la compression
  (Phase 1) et explosent pendant l'expansion (Phase 2).

### CHARTISME — 2 points

- **Figure (1 pt)** : pattern de continuation haussière (VCP / tasse avec
  anse : tasse 12-33 % de profondeur, anse 1-4 semaines) ; pivot travaillé
  de nombreuses fois, pivot sur les hauteurs de la figure ; contractions
  décroissantes (chaque repli ≈ la moitié du précédent, 2 à 4 contractions) ;
  micro-base extrêmement resserrée sur la fin (boîte de Darvas dont le
  plafond coïncide avec le pivot).
- **Gaps (1 pt)** : pas de gros gaps, pas de trous de cotation pendant la
  construction de la figure.

### CHANDELIERS JAPONAIS — 2 points

- **Breakout (1 pt)** : chandelier de Type 1 (Marubozu), cassure décisive
  (3-5 % au-delà du pivot), volume ≥ 150 % de la moyenne 50 jours.
- **Liquidité (1 pt)** : pas de longues mèches excessives au-delà du pivot.

### FORCE RELATIVE (RS) — intégrée au classement

L'actif doit surperformer : performance 4-8 semaines dans le top quartile de
l'univers scanné (ou > BTC pour un alt). Critère commun O'Neil (RS ≥ 80) /
Minervini — sans RS, le point Tendance ne s'applique pas.

### Classification

| Classement | Verdict | Posture |
|---|---|---|
| 9-10 | **ROCKET ALPHA** | Trading neutre/offensif |
| 7-8 | **ROCKET** | Trading neutre |
| < 7 (ou véto unlocks) | **ÉLIMINÉ** | — |

---

## GESTION DES TRADES — À COMPLÉTER

Entrées (stop-limit ?), invalidation, stop (références connues : sous la
dernière contraction, ~8 % sous le pivot — O'Neil), sorties, trailing :
**en attente de la source de l'application propriétaire** (décision étape 5,
point 4). Ce document sera complété et re-figé à réception.

## ENRICHISSEMENT IA — objectifs cadrés (onglet de la page Définition)

- Évaluer le catalyseur « news » (flux ETF, listings, réglementation) — le
  volet lecture qui complète les critères chiffrables du moteur.
- Ranker les faux pivots (conviction sur les candidats détectés).
- Analyser la performance par pilier du classement (quels critères gagnent).
- Garde-fou commun : l'IA n'ouvre jamais de trade ; moteurs figés ; toute
  modification de réglage = acte du propriétaire.
