# Rockets — Définition canonique (figée)

Sources : « Rocket Hunter » (© Rocket Trading — Arnaud Biegun, PDF archivé `07 - Rocket hunter.pdf`)
+ affinages validés le 24/08/2026, issus des canoniques momentum breakout
(Minervini VCP/Trend Template, O'Neil CANSLIM/tasse-avec-anse, Darvas,
Bollinger squeeze).

**Périmètre** — deux univers, même classement /10 (extension actions du
01/09) :
- **Crypto** : scan top 300 Binance en volume (élargi du top 100 le 31/08
  pour couvrir les candidats rotation, blacklist v1 conservée) ;
- **Actions US** : univers = répertoire officiel NASDAQ Trader (~5 667
  actions communes, ETF/preferreds/warrants exclus), prix D1 Tiingo
  (volume réel), marché de référence QQQ même source. Pré-screen trend
  template Minervini (8 conditions) pour ENTRER dans le périmètre scanné,
  puis même classement /10. News : flux Yahoo Finance par ticker.
  Avertissement earnings (badge 📊, pas de veto — décision 31/08).
  **Observation silencieuse** : journalisation seule, aucun signal ni
  position jusqu'à décision du propriétaire sur preuve.
**Timeframes** : détection D1 ; surveillance du pivot en temps réel.

---

## LE CLASSEMENT — 10 points, 4 piliers, seuil d'élimination à 7

### FONDAMENTAL — 3 points (équivalents crypto validés)

| Critère (1 pt chacun) | Définition crypto |
|---|---|
| **Sentiment de marché** | Crypto : BTC haussier D1 (BTC > MM50 > MM200) ET secteur du token en tendance (perf secteur > BTC sur 4 s). Actions US : QQQ haussier D1 (même règle) ET action battant le QQQ sur 4 semaines (RS d'O'Neil) |
| **Contexte** | Sortie de large base + prise de position sur la 1ère base (2ème maximum) — inchangé, indépendant de la classe d'actif |
| **News catalyseur** | Crypto : flux ETF nets positifs, listing, upgrade réseau, réglementation favorable ET aucun déverrouillage dans les 15-30 j. Actions US : dépêches Yahoo Finance (résultats, contrats, homologations, relèvements d'analystes) — et date de résultats extraite si mentionnée (badge, pas de veto) |

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

## GESTION DES TRADES — reformulation extraite de l'appli « Journal de Trading » (PROPOSITION à valider)

Source : `/home/rono/Applis Nono/Journal de Trading/` (composants rocket/ —
RocketEntryForm, RocketNeutralizationModal, useTradeEntryLogic, useLivePrices).

### Le cycle de vie d'une rocket

```
DÉTECTION (scanner D1, classement /10)          [définition ci-dessus]
   └─ ORDRE STOP-LIMIT au pivot (pending)
        entry_stop = prix de déclenchement (pivot)
        entry_limit = plafond accepté (garde le slippage d'une cassure violente)
        invalidation = stop sous la dernière contraction / micro-base
        trailing_stop démarre = invalidation
        quantité = (capital × risque%) / |entrée − stop|, plafonnée à 5 % du capital
   └─ ACTIVATION (open) : ordre exécuté → entry_executed (prix réel)
   └─ R1 ATTEINT (entrée + 1R) → NEUTRALISATION :
        ① vendre 50 % de la position (sécurisation)
        ② trailing stop en pourcentage du prix (défaut 5 %, réglable)
   └─ SORTIE : prix touche le trailing stop → clôture du solde
        P&L = 50 % vendu à R1 + solde à la sortie trailing
        (sortie anticipée possible : invalidation avant R1 = -1R)
```

### Profils de risque (money management de l'appli)

| Profil | Action / Crypto | ETF |
|---|---|---|
| Peu Risqué | 0,5 % du capital | 2 % |
| Neutre | 1 % | 3 % |
| Risqué | 2 % | 4 % |

Plafond de position : **5 % du capital** par rocket (montant, indépendant du
risque). Mapping proposé avec le classement : ROCKET (7-8) → profil Neutre ;
ROCKET ALPHA (9-10) → profil Risqué ; Peu Risqué = choix prudent libre.

## ENRICHISSEMENT IA — objectifs cadrés (onglet de la page Définition)

- Évaluer le catalyseur « news » (flux ETF, listings, réglementation) — le
  volet lecture qui complète les critères chiffrables du moteur.
- Ranker les faux pivots (conviction sur les candidats détectés).
- Analyser la performance par pilier du classement (quels critères gagnent).
- Garde-fou commun : l'IA n'ouvre jamais de trade ; moteurs figés ; toute
  modification de réglage = acte du propriétaire.
