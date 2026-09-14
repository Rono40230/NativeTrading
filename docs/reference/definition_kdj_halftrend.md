# KDJ/Halftrend — Document de définition (étalon de conversion)

> Source de vérité unique pour les conversions Rust (7.B) et MQ5 (7.D).
> Toute divergence entre Pine, Rust et MQ5 est un bug — le Pine tranche.

- **Étalon** : `docs/reference/strategie_550_pourcent_v4.pine` (249 lignes)
- **Empreinte MD5 (figée)** : `be3343edd854d0759c6ac23bb2ec2b69`
- Nom court : « 550 % » · Actif de naissance : ETHUSD · Screening phase A : GO (4/8 porteurs, D1 ≫ H1, H4 exclu)

## 1. Paramètres

| Input | Valeur screening | Rôle |
|---|---|---|
| `period` (ilong) | 20 | fenêtre high/low du KDJ |
| `signal` (isig) | 7 | lissage K et D |
| `Amplitude` | 2 | fenêtre highma/lowma du HalfTrend |
| `RatioRisk` | 2 | TP = RatioRisk × distance entrée→EMA200 |

Codés en dur dans l'étalon (ne pas exposer) : `m = 1` (poids lissage),
`channelDeviation = 2`, ATR HalfTrend = `atr(100)/2`, `basePeriods = 26`
(Donchian affichée, hors trading), EMA 50/100 (affichées, hors trading),
SMA100 et EMA200 (filtres de tendance).

## 2. Indicateurs

### 2.1 KDJ
```
RSV = 100 × (close − lowest(low, ilong)) / (highest(high, ilong) − lowest(low, ilong))
pK = (m·RSV + (isig − m)·pK[1]) / isig        // init : nz → 0
pD = (m·pK  + (isig − m)·pD[1]) / isig        // init : nz → 0
pJ = 3·pK − 2·pD
```
Lissage type EMA de période `isig`, initialisé à 0 (`nz(pK[1])`).
Warmup : RSV calculé dès que `ilong` barres existent (le screening brûle
de toute façon 200+ barres via EMA200).

### 2.2 ATR (pour HalfTrend)
`atr2 = atr(100) / 2` — ATR de Wilder : TR = max(h−l, |h−c[1]|, |l−c[1]|),
init = moyenne des 100 premiers TR, puis `atr = (atr[1]·99 + tr)/100`.

### 2.3 HalfTrend (amplitude=2, dev = channelDeviation × atr2 = atr(100))
État persistant : `trend=0`, `nextTrend=0`, `maxLowPrice=low[1]`,
`minHighPrice=high[1]`, `up`, `down`. À chaque barre (valeurs clôturées) :
```
highPrice = max(high, high[1])      // = high[highestbars(2)] — valeur du max
lowPrice  = min(low,  low[1])       // = low[lowestbars(2)]
highma = sma(high, 2) ; lowma = sma(low, 2)

si nextTrend == 1 :
    maxLowPrice = max(lowPrice, maxLowPrice)
    si highma < maxLowPrice et close < low[1] :
        trend = 1 ; nextTrend = 0 ; minHighPrice = highPrice
sinon :
    minHighPrice = min(highPrice, minHighPrice)
    si lowma > minHighPrice et close > high[1] :
        trend = 0 ; nextTrend = 1 ; maxLowPrice = lowPrice

si trend == 0 :
    si trend[1] != 0 :  up = down[1] (ou down si na) ; arrowUp = up − atr2
    sinon            :  up = max(maxLowPrice, up[1]) (ou maxLowPrice si na)
sinon :
    si trend[1] != 1 :  down = up[1] (ou up si na) ; arrowDown = down + atr2
    sinon            :  down = min(minHighPrice, down[1]) (ou minHighPrice si na)
```
`ht = trend == 0 ? up : down`.

Signaux (barre de retournement uniquement) :
- `buySignal  = arrowUp ≠ na   et trend == 0 et trend[1] == 1`
- `sellSignal = arrowDown ≠ na et trend == 1 et trend[1] == 0`

## 3. Conditions et entrées

- **Long** : `SMA100 > EMA200` **et** `pJ > pD` **et** `close > EMA200`, déclenché par `buySignal`.
- **Short** (miroir strict) : `SMA100 < EMA200` **et** `pJ < pD` **et** `close < EMA200`, déclenché par `sellSignal`.
- Les conditions sont évaluées **à la clôture** de la barre N ; l'ordre
  (`strategy.entry`) est exécuté à l'**open de la barre N+1** (modèle TV par
  défaut, `process_orders_on_close` désactivé).

## 4. Gestion du trade — tout est figé à la barre d'entrée

La barre d'entrée est N+1 (première barre où `position_size ≠ 0`) :
- `E` = **open(N+1)** (`valuewhen(EnteredLong, open, 0)`)
- `EMA200_E` = **EMA200(N+1)** (`valuewhen(EnteredLong, EMA200, 0)`)

**Long** :
```
risk%   = (E − EMA200_E) / E × 100
TP      = E × (1 + RatioRisk × risk% / 100)   // = E + RatioRisk × (E − EMA200_E)
SL_niveau = EMA200_E
déclencheurs (évalués à la clôture, sur CROISEMENTS avec des constantes) :
TP_touché = crossover(high, TP)        // high[1] ≤ TP et high > TP
SL_touché = crossunder(low, SL_niveau) // low[1] ≥ SL_niveau et low < SL_niveau
```
**Short** : miroir — `TP = E − RatioRisk × (EMA200_E − E)`, `SL_niveau = EMA200_E`,
`TP_touché = crossunder(low, TP)`, `SL_touché = crossover(high, SL_niveau)`.

**Sorties** (dans l'ordre) :
1. `TP_touché or SL_touché` à la clôture de la barre M → **clôture à open(M+1)**
   (pas au niveau TP/SL — voir piège 1).
2. Signal inverse *complet* (conditions + trigger) → retournement : l'ancien
   trade est fermé au prix d'entrée du nouveau (open(N+1) commun).
3. Fin de données position ouverte → trade « ouvert », exclu des stats clôturées.

## 5. Les pièges (décisions figées)

1. **Aucun ordre limit/stop** : l'étalon n'utilise QUE `strategy.entry/close(when=…)`.
   Les niveaux TP/SL ne sont que des **seuils de détection** : le P&L réel est
   `open(sortie) − open(entrée)`, jamais le niveau. Conséquence : l'ambiguïté
   « TP et SL touchés dans la même barre » **n'existe pas pour les prix** (sortie
   unique à l'open suivant quel que soit l'ordre intrabar). Si les deux crosses
   tombent sur la même barre, le verdict est labellisé **TP** (ordre de
   l'expression `TPlong or StopLong` dans l'étalon).
2. **Repaint HalfTrend** : en live, la machine d'état s'évalue intrabar. Le
   rejeu Rust évalue **à la clôture seule** (bougies clôturées) — zéro repaint.
   Une divergence live-TV intrabar est possible mais le backtest TV
   (Bar Magnifier off) évalue lui aussi à la clôture : le rejeu est fidèle AU
   BACKTEST, référence du screening.
3. **valuewhen au prix de la barre d'entrée** : `E` et `EMA200_E` viennent de
   la barre d'**exécution** (N+1), pas de la barre de signal (N). Cas dégénéré
   possible : open(N+1) sous EMA200(N+1) → `risk% < 0` → TP **sous** l'entrée.
   Comportement reproduit à l'identique — on ne « répare » pas l'étalon.
4. **Croisements, pas de niveaux** : `crossunder(low, K)` exige `low[1] ≥ K`.
   Si le low était déjà sous K à la barre précédente, le SL **ne se déclenche
   pas** (pas de « cross ») tant que le prix n'est pas repassé au-dessus puis
   redescendu — trade « zombie » possible sous son SL. Là aussi : fidélité à
   l'étalon, le screening TV contient ces trades.
5. **Warmup** : SMA na < 100 barres, EMA na < 200 barres puis seedée par la
   SMA des 200 premières closes (convention du moteur TV actuel — celui qui a
   produit le screening ; divergences éventuelles de seed se résorbent dans
   la fenêtre de chauffe), KDJ initialisé à 0. Aucun trade avant ~200 barres
   (filtre EMA200) : les 200 premières barres de chaque rejeu sont brûlées.

## 6. Conventions du rejeu Rust (7.B)

- Évaluation exclusivement sur bougies **clôturées** (convention piège 2).
- Entrée = open(N+1), sortie = open(M+1) ; frais appliqués comme le screening :
  commission 0,05 % par ordre (entrée + sortie) + slippage 2 ticks par ordre.
- R du trade = (open_sortie − E) / (E − EMA200_E) pour un long (miroir short)
  — R mesuré sur la distance d'entrée à l'EMA200 de la barre d'entrée.
- Verdicts : TP / SL / Retournement / Ouvert (fin de données).

## 7. Hors périmètre (affichage pur)

Blocs bicolores (`box.new`), plots KDJ/HalfTrend/EMA/Donchian, alertes,
`showArrows`/`showChannels` — rien de tout cela n'affecte le trading.
