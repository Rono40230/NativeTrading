# Recherche de référence — Stratégies « ROCKETS » (VCP crypto) et « STRADDLE » (volatilité macro)

Date : 2026-08-17
Objectif : documentation factuelle en vue de la redéfinition formelle des deux stratégies avant migration en plugins du runtime de trading.
Méthode : recherches web + lecture des sources citées. Chaque affirmation chiffrée est rattachée à une source. Les zones d'ombre (info introuvable, contradictoire ou d'origine promotionnelle) sont signalées explicitement.

---

# SECTION 1 — « ROCKETS » : VCP (Volatility Contraction Pattern) appliqué au momentum crypto

## 1.1 Définition canonique du VCP

Le VCP est un setup de breakout trend-following popularisé par **Mark Minervini** (2× champion US d'investissement ; retour audité de +334 % en 2021 selon TrendSpider ; il revendique 33 554 % sur 5,5 ans dans *Trade Like a Stock Market Wizard*). Il est décrit en détail dans son livre *Think and Trade Like a Champion*.

Principe : après une forte tendance haussière (phase d'accumulation institutionnelle), le prix entre dans une consolidation où **chaque pullback successif est plus étroit que le précédent** tandis que **le volume diminue à chaque contraction** (« supply drying up » — l'offre s'assèche, transfert des mains faibles vers les mains fortes). La volatilité se contracte jusqu'à un point de rupture : le **pivot**. Le breakout au-dessus du pivot sur volume expansif marque la reprise de la tendance.

- Sources : [TrendSpider — VCP: A Trader's Guide](https://trendspider.com/learning-center/volatility-contraction-pattern-vcp/), [TraderLion — Mastering the VCP](https://traderlion.com/technical-analysis/volatility-contraction-pattern/), [TradingSim — VCP Trading Guide](https://www.tradingsim.com/blog/volatility-contraction-pattern)

Analogie utile (TradingSim) : le VCP prend racine dans le « wave pattern » de Wyckoff et les méthodes de Bill O'Neil (CANSLIM / cup with handle). C'est une formalisation de l'accumulation avant markup.

## 1.2 Prérequis : le contexte de tendance (Trend Template de Minervini + Stage 2 de Weinstein)

Le VCP n'est valide que sur un actif déjà en tendance haussière. Minervini filtre d'abord via son **Trend Template** (chiffres TrendSpider) :

1. Prix au-dessus des MA 50, 150 et 200 jours.
2. MA 50 > MA 150 > MA 200 (alignement).
3. Prix au moins **+30 % au-dessus de son plus bas 52 semaines**.
4. Prix à **moins de 25 % de son plus haut 52 semaines** (« top right » trading — on achète près des plus hauts, pas au fond).
5. **Relative Strength (RS) > 70** (préférence > 90) ; 80 % des grands gagnants avaient un RS > 80 avant leur progression (FinerMarketPoints).
6. MA 200 croissante (plus haute qu'il y a 30 jours).

Contexte Weinstein : le VCP idéal se forme en **Stage 2 (advancing)**. Critères Stage 2 (Stan Weinstein, *Secrets for Profiting in Bull and Bear Markets*) : cassure de la résistance du base Stage 1 sur **volume ≥ 2× la moyenne**, prix au-dessus d'une **MA 30 semaines orientée à la hausse** (Weinstein utilisait originellement la MA 30 semaines ; il recommande désormais la MA 40 semaines ≈ MA 200 jours). FinerMarketPoints affirme que les VCP en Stage 2 ont un taux de réussite ~3× supérieur aux autres stades et que 90 % des trades gagnants de Minervini étaient en Stage 2 (chiffre vendeur, à considérer avec prudence).

Contexte marché : indices / marché global en **tendance haussière confirmée** (au-dessus de leur MA 200, plus hauts ascendants). Trader des VCP en correction du marché ferait chuter le taux de réussite de 60-70 % selon FinerMarketPoints (statistique vendeur, non vérifiable).

- Sources : [TrendSpider](https://trendspider.com/learning-center/volatility-contraction-pattern-vcp/), [TraderLion — Stage Analysis](https://traderlion.com/trading-strategies/stage-analysis/), [FinerMarketPoints — VCP Criteria Checklist](https://www.finermarketpoints.com/post/vcp-criteria-complete-checklist), [TrendSpider — Weinstein Stage Analysis](https://trendspider.com/blog/master-market-trends-with-ai-powered-weinstein-stage-analysis/), [AlphaTarget — Stage Analysis](https://alphatarget.com/insights/stage-analysis-an-overview/)

## 1.3 Phases du pattern

1. **Base (base building)** : consolidation 3 à 12 semaines après un uptrend. Pullback initial typique depuis les plus hauts : **20-30 %** (zone d'accumulation institutionnelle selon TradingSim).
2. **Contractions successives (T1, T2, T3…)** : 2 à 6 contractions (typiquement 3-4). Séries typiques citées : `25 % → 15 % → 8 % → 4 %` (TraderLion), `15 % → 10 % → 5 %` (FinerMarketPoints), `20 % → 10 % → 5 %` (TrendSpider). Progression strictement décroissante requise.
3. **Assèchement du volume (VDU — Volume Dry Up / « dry volume »)** : volume de chaque contraction inférieur au précédent ; sur la contraction finale, 3-5 jours consécutifs de volume exceptionnellement bas (40-60 % de la moyenne 50 jours selon FinerMarketPoints) avec amplitude de prix minimale. Un **shakeout** (mèche sous le range final qui récupère vite) est fréquent juste avant le breakout — c'est un piégeage des mains faibles, pas une invalidation.
4. **Pivot & breakout** : le **pivot** = plus haut de la contraction finale (pas le plus haut global du pattern). Le breakout = cassure du pivot sur **volume expansif**. Entrée sur buy-stop 1-2 % au-dessus du pivot.

## 1.4 Détection du breakout et exécution

- **Volume de breakout** : les sources divergent légèrement — FinerMarketPoints : **≥ 140-150 % du volume moyen 50 jours** ; TradingSim : **+40 à 50 % au-dessus de la moyenne** (et cite Minervini : « les mouvements de prix sans volume sont des pièges ») ; TraderLion : pic de volume **+30-40 %** à +100 % et plus. Point commun : un breakout sans expansion de volume est le principal signal de faux breakout.
- **Confirmation intraday** (TradingSim) : cassure qui **clôture au-dessus de la MA 20 jours** ; en intraday, le prix tient VWAP et les MA 10/20/50.
- **Compression mesurable** : l'ATR tombe à environ **1/3 de sa moyenne 50 jours** sur la contraction finale (TradingSim).
- **Stop-loss** : juste **sous le plus bas de la contraction finale** (la volatilité étant minimale à l'entrée, le stop est serré — c'est l'intérêt du pattern). TraderLion cite un risque de **5-8 %** sous l'entrée.
- **R:R / sortie** : R:R minimum ~3:1, objectif de progression type +15 % ou plus, sortie dans la force (scale-out) ou stop trailing (TraderLion). Risque par trade ≤ 1 % du compte (TradingSim, prudent).
- **Ne pas poursuivre** (chasing) : entrer > 5 % au-dessus du pivot détruit le ratio ; les entrées à ≤ 5 % du pivot donneraient ~40 % de meilleur R:R (FinerMarketPoints).
- Statistiques vendeurs à noter avec recul : taux de réussite du breakout **60-70 %** si volume fort (TraderLion) ; **50 %+ des « superperformers » présentent un VCP avant leur avancée** (TraderLion).

## 1.5 Faux signaux documentés

- **Breakout sans volume** : trap classique (piège de liquidité) — mouvement sans participation institutionnelle.
- **Progression non décroissante** : ex. `12 % → 15 % → 8 %` est invalide (la 2e contraction dépasse la 1re) — FinerMarketPoints.
- **Base trop courte** (< 3 semaines) : pas d'accumulation réelle. **Base trop longue** (> 12-15 semaines) : actif « mort », manque d'acheteurs au breakout — FinerMarketPoints.
- **Chasing** le breakout bien au-dessus du pivot (> 5-10-15 %) — FinerMarketPoints.
- **Contexte marché mauvais** : VCP en Stage 3/4 ou pendant une correction des indices = taux d'échec massif.
- **Confusion avec le « high tight flag »** : critères différents (TradingSim).
- Le bull flag / consolidation « peut se résoudre dans les deux sens » : compression sans assèchement de volume = pas un VCP (TradingSim).
- Patience nécessaire : TradingSim cite l'exemple d'un trader regardant « 20+ setups échouer avant celui qui délivre ».

## 1.6 Application aux cryptomonnaies — spécificités et outils

État des sources : **il n'existe pas de source « canonique » formelle pour le VCP crypto**. L'application crypto est communautaire (TradingView, blogs). Les adaptations documentées :

- **Marché 24/7, pas de gaps** : les contractions overnight/gap-up des actions n'existent pas ; les bougies journalières sont alignées sur l'heure de l'exchange (00:00 UTC en général). Les week-ends ont des volumes structurellement plus faibles → le « dry volume » du weekend doit être pondéré, sinon tout samedi ressemble à un VDU.
- **Volatilité supérieure** : les profondeurs de contraction admissibles sont plus grandes que sur actions ; les séries du type `30 % → 20 % → 10 %` sont communes sur altcoins (interprétation communautaire, pas sourcée formellement — à valider par backtest maison).
- **Relative Strength** : se remplace avantageusement par la performance vs BTC (ou vs un index altcoin), même logique « top right ».
- **Pump and dump / faux breakouts** plus fréquents sur small caps à faible liquidité → le filtre de volume (RVOL) et la liquidité minimale sont encore plus critiques qu'en actions.

Outils existants utilisant des patterns similaires :

- **TradingView** : pas de filtre VCP natif, mais indicateurs communautaires — « VCP Scanner - Minervini Method » (Pine Script v5, by valchak), « Breakout Scanner (Crypto) Multi Exchange », collection [tradingview.com/scripts/vcp](https://www.tradingview.com/scripts/vcp/) ; exemple d'application BTCUSDT : [VCP on BINANCE:BTCUSDT](https://www.tradingview.com/chart/BTCUSDT/GOOIN2DR-Volatility-Contraction-Pattern/).
- **altFINS** (crypto screener) : reconnaissance automatique de **26 patterns de chart** sur ~500 coins, classés *Emerging* (pattern en formation — équivalent « avant breakout ») vs *Complete/Breakout* (cassure déjà effectuée), multi-timeframe 15m→hebdo. Triangles symétriques/ascendants/descendants, wedges et rectangles = familles de contraction proches du VCP. Sources : [altFINS chart patterns](https://altfins.com/knowledge-base/chart-patterns/), [guide des preset screeners](https://altfins.com/knowledge-base/the-complete-guide-to-altfins-preset-screener-filters/).
- **Cointelegraph Markets Pro (VORTECS Score)** : score propriétaire (0-100) combinant prix, sentiment, actualités (NewsQuakes), on-chain et momentum. Stratégies documentées type « Buy 80 / Sell 80 ». **Ce n'est pas un détecteur de VCP** — c'est un score de conditions favorables momentum ; les chiffres de performance (179 % sur 4 alerts, etc.) sont issus du marketing Cointelegraph, non vérifiés. Sources : [Investing.com — VORTECS Report Summary](https://www.investing.com/news/cryptocurrency-news/cointelegraph-markets-pro-vortecs-report-summary--179-gains-from-4-alerts-3032632).
- **Velo Data (velo.xyz)** : agrégation/standardisation de données multi-exchanges, focus dérivés (funding, open interest, liquidations) — utile comme contexte (activité de levier pendant la contraction), pas un screener de patterns. Source : [CryptoSlate — Velo Data](https://cryptoslate.com/companies/velo-data/).
- Autres : Gainium « Breakout Screener », Cryptolume, Altrady screener (100+ indicateurs temps réel).

Honnêteté : aucune de ces plateformes ne documente publiquement un « VCP crypto » nommé comme tel avec seuils chiffrés. La formalisation crypto reste à la charge du projet (d'où l'intérêt de la section critères mesurables ci-dessous).

## 1.7 ROCKETS — Critères mesurables (traduisibles en règles algorithmiques)

Filtre de contexte (Trend Template adapté, échelle journalière) :
| # | Règle | Seuil documenté | Source |
|---|-------|-----------------|--------|
| C1 | Prix > MA50, MA150, MA200 | binaire | TrendSpider |
| C2 | MA50 > MA150 > MA200 | binaire | TrendSpider |
| C3 | Prix ≥ +30 % au-dessus du plus bas 52 semaines (adaptable : 90 jours crypto) | ≥ +30 % | TrendSpider |
| C4 | Prix ≤ 25 % sous le plus haut 52 semaines (90 jours crypto) | ≤ 25 % | TrendSpider |
| C5 | RS (vs BTC ou vs marché) dans le top | > 70 centile (préf. > 90) | TrendSpider / FMP |
| C6 | MA200 croissante sur 30 jours | binaire | TrendSpider |
| C7 | Marché global (BTC / indice) en Stage 2 (au-dessus MA200 croissante) | binaire | Weinstein / TraderLion |

Détection du pattern (base) :
| # | Règle | Seuil documenté | Source |
|---|-------|-----------------|--------|
| P1 | Nombre de contractions (T1..Tn) | 2-6 (typ. 3-4) | TrendSpider / FMP |
| P2 | Profondeur décroissante stricte : T(k+1) < T(k) | ratio ~0,5-0,6 (chaque contraction ~20-30 % plus petite que la précédente) ; séries types 25→15→8→4 % ou 20→10→5 % | FMP / TraderLion / TrendSpider |
| P3 | Durée de la base | 3-12 semaines (< 3 : invalide ; > 12-15 : trop « morte ») | FMP / TraderLion |
| P4 | Pullback initial depuis les plus hauts | 20-30 % | TradingSim |
| P5 | Volume décroissant à chaque contraction | volume contraction ≤ 40-60 % de la moyenne 50 j sur la contraction finale (VDU) | FMP |
| P6 | Compression de volatilité | ATR final ≈ 1/3 de l'ATR moyen 50 j | TradingSim |
| P7 | Contraction finale | 3-5 jours de volume exceptionnellement bas + amplitude minimale | FMP |

Pivot et exécution :
| # | Règle | Seuil documenté | Source |
|---|-------|-----------------|--------|
| E1 | Pivot | plus haut de la contraction finale | TrendSpider / FMP |
| E2 | Ordre d'entrée | buy-stop 1-2 % au-dessus du pivot ; ne pas entrer > 5 % au-dessus du pivot | FMP |
| E3 | Volume au breakout | ≥ 140-150 % du volume moyen 50 j (ou min +40-50 %) | FMP / TradingSim |
| E4 | Confirmation | clôture au-dessus du pivot / de la MA20 | TradingSim |
| E5 | Stop-loss | sous le plus bas de la contraction finale (risque typique 5-8 %) | TrendSpider / TraderLion |
| E6 | Risque par trade | ≤ 1 % du compte | TradingSim |
| E7 | Objectif / R:R | R:R ≥ 3:1 ; sortie dans la force / stop trailing | TraderLion |

---

# SECTION 2 — « STRADDLE » : volatilité autour des annonces macro (version ordres stop, CFD/spot)

## 2.1 Définition canonique et distinction des deux « straddles »

Deux stratégies portent ce nom — ne pas les confondre :

1. **Straddle options** (définition canonique Investopedia) : achat simultané d'un call et d'un put **même strike, même échéance**, pour profiter d'un grand mouvement dans n'importe quelle direction. Risque maximal = prime payée. Faiblesses : décroissance temporelle (theta) et **IV crush** après l'annonce.
2. **Straddle ordres stop (news trading straddle)** — celle qui nous intéresse : peu avant une annonce majeure, placement d'un **buy stop au-dessus du range pré-annonce** et d'un **sell stop en dessous**. Le mouvement initial déclenche un côté ; on **annule l'autre** (OCO — one-cancels-other). Pari sur l'expansion de volatilité, pas sur la direction.

Différence structurelle documentée (synthèse des sources) : le straddle options a les deux jambes remplies *avant* l'événement à coût connu (la prime) et perte maximale bornée ; le straddle stop orders est rempli *après* le breakout à des prix incertains (slippage) et sa perte potentielle est non bornée (double déclenchement + slippage), mais ne paie ni prime ni theta.

- Sources : [Investopedia — Straddle](https://www.investopedia.com/terms/s/straddle.asp), [tastylive — Straddle](https://tastylive.com/learn/trading-products/options/straddle/), [ForexOP — Straddle trade intro](https://forexop.com/learning/straddle-trade-intro/), [FxGlory — Forex Straddle Strategy](https://fxglory.com/learn/forex-strategies/forex-straddle-strategy/)

## 2.2 Mécanique exacte des ordres (version stop orders)

Séquence type, consolidée depuis les sources :

1. **Sélection de l'annonce** : calendrier économique, uniquement les annonces à fort impact (voir classement 2.3).
2. **Repérage du range pré-annonce** : la volatile se contracte dans un petit range dans les minutes précédant la sortie (FXSSI : placer les ordres aux bornes support/résistance de ce range ; NAMH : utiliser le range des 30 dernières minutes avant la sortie).
3. **Placement des ordres** : buy stop au-dessus du range + sell stop en dessous. Distances documentées : **10-15 pips** au-delà des bornes du range (FXSSI), **15-20 pips** autour du prix courant (NYC Servers, NFP), **20-30 pips** (Taurex). FxGlory recommande de baser la distance sur la taille du range, la volatilité, le spread et la distance de stop — pas un chiffre fixe.
4. **Timing de placement** : de **~1 minute** (EarnForex, version positions market) à **2 minutes** (FXSSI, Taurex) et **5 minutes** (NYC Servers) avant la sortie. Toutes les sources insistent : ordres déjà en place AVANT la sortie ; jamais de market order dans la seconde de l'annonce.
5. **Déclenchement** : l'annonce fait sauter un côté ; **annulation immédiate de l'autre ordre** (OCO). Attention (FxGlory) : ne jamais supposer que les deux ordres sont liés OCO côté broker — il faut une règle explicite d'annulation écrite à l'avance.
6. **Gestion** : SL sur la position déclenchée (15-30 pips selon sources), TP à 3-5× le SL ou stop trailing ; time-stop.
7. **Sortie temporelle** : clôture de toute position restante 1 h après l'annonce (EarnForex).

Variantes documentées :
- **Straddle pré-annonce** (ci-dessus) : attrape le premier mouvement, mais subit le pire du spread/slippage/whipsaw.
- **Straddle post-annonce** (FxGlory) : attendre le premier spike, puis straddler le petit range qui se reforme — réduit le risque du premier tick, risque de rater le mouvement.
- **Straddle positions market** (EarnForex) : ouvrir buy ET sell ~1 min avant la sortie avec SL 10-20 pips chacun et TP = 5× SL : un côté prend son TP, l'autre son SL. Contourne le problème de remplissage des stops (les positions existent déjà), mais double le coût de spread.

## 2.3 Annonces qui bougent le plus (classement)

Consensus des sources, par impact :

1. **NFP (Non-Farm Payrolls, US)** — premier vendredi du mois, **8:30 AM ET**. Mouvement type **50-100+ pips sur EUR/USD** (Axiory, NYC Servers).
2. **FOMC (décision de taux + statement US)** — statement **14:00 ET**, conférence de presse **14:30 ET** (format depuis 2011) — deux vagues de volatilité distinctes à 30 min d'intervalle. Moves de **100+ pips** possibles (TradingView).
3. **CPI (inflation US)** — **8:30 AM ET**. Devenu l'annonce la plus regardée depuis 2021-2022 (sensibilité de la Fed à l'inflation) ; classé top-tier avec NFP/FOMC.
4. **Décisions de taux ECB / BoE** (et BCE conférence de presse) — fort impact sur EUR/GBP.
5. **US GDP, PCE, retail sales, PPI** — tier en dessous mais tradables (EarnForex liste GDP, NFP, taux, PCE ; Taurex note 3-10× l'ATR normal sur la première bougie).

Horaires de référence : presque toutes les données US majeures sortent à **8:30 AM ET** (slot standard — NFP, CPI, PPI, retail sales, GDP). Sources officielles : [calendrier FOMC (Federal Reserve)](https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm), [calendrier CPI (BLS)](https://www.bls.gov/schedule/news_release/cpi.htm), [CME — CPI report guide](https://www.cmelitegroup.com/knowledge-hub/cpi-report-release-time-schedule-latest-data-and-how-it-moves-markets/), [NY Fed sr512](https://www.newyorkfed.org/medialibrary/media/research/staff_reports/sr512.html).

Amplitude : première bougie d'annonce = **3 à 10× l'ATR normal** (Taurex).

## 2.4 Gestion du whipsaw (double déclenchement)

Le whipsaw — le prix déclenche un côté, fait le plein de stops, puis s'inverse et déclenche l'autre côté — est le risque n°1 documenté :

- Causes (FxGlory, Financial Source) : liquidité mince, chasse aux stops (« stop clearing »), **annonces mixtes** (ex. NFP fort mais salaires faibles), réactions au headline avant lecture des détails, rapidité du repricing, élargissement du spread qui déclenche prématurément les ordres.
- Mitigations documentées :
  - **SL plus larges** (20-30 pips plutôt que 5-15 : un trader Reddit stoppé avec 5 pips sur NFP, survivait avec 20) ;
  - **annulation immédiate et explicite** de l'ordre opposé dès le premier remplissage (OCO forcé) ;
  - **règle de non-re-entrée** (one-attempt rule — FxGlory) ;
  - **règle de spread maximal** : si le spread dépasse X, ne pas trader / tout annuler (FxGlory, « spread limit with no-trade rule ») ;
  - **variante post-annonce** : laisser passer le premier spike et straddler le range suivant (FxGlory) ;
  - alternative « momentum » (Financial Source) : entrer dans les 30 premières secondes après l'annonce dans le sens du spike initial, stop sous la bougie du spike, cibles 30-100 pips, replis Fibonacci 23/38/50 % pour ré-entrées.
- FxNX (2024+) note que le straddle « classique à 15 pips » est désormais vulnérable aux **liquidity hunts** organisés (brokers/institutionnels) — l'edge s'est dégradé sur les paires majeures.

## 2.5 SL/TP et fenêtres temporelles documentés (par source)

| Source | Placement avant annonce | Distance des stops | SL | TP | Divers |
|--------|------------------------|--------------------|----|----|--------|
| [EarnForex](https://www.earnforex.com/forex-strategy/important-news-trading-strategy/) | 1 min (positions market des 2 côtés) | — | 10-20 pips | 5× SL (R:R 1:5) | breakeven dès profit = SL ; sortie forcée 1 h après |
| [FXSSI](https://fxssi.com/news-trading-strategies) | ~2 min (ordres stop) | 10-15 pips au-delà des bornes du range pré-annonce | ~15 pips | trailing (« quelques pips ») | annuler l'ordre non déclenché ; variante hedge : SL 15-20 pips, TP 3× SL |
| [NYC Servers](https://newyorkcityservers.com/blog/how-to-trade-nfp) | 5 min avant NFP (8:30 ET) | 15-20 pips autour du prix | 20-30 pips | 30-50 pips = bon résultat (moves 50-100+) | EUR/USD spread 1 → 5-10 pips à l'annonce ; slippage 5-15 pips ; taille de position 0,5-1 % |
| [Taurex](https://www.tradetaurex.com/forex-insights/forex-news-trading-strategies/) | ~2 min | 20-30 pips autour du prix | — | — | bougie d'annonce = 3-10× ATR |
| [NAMH Global](https://www.namhglobal.com/zh/market-analysis/nfp-trading-guide-2026) | range des 30 min précédentes | 10-20 pips au-delà du range | — | — | — |
| [FxGlory](https://fxglory.com/learn/forex-strategies/forex-straddle-strategy/) | qualitatif (avant ou après le 1er spike) | fonction du range/volatilité/spread | sous la borne opposée | niveau fixe / mesure du range / trailing / time-stop | règles qualitatives, aucun chiffre |

## 2.6 Risques documentés (et quand NE PAS trader)

- **Whipsaw / double déclenchement** : voir 2.4.
- **Spread qui s'élargit** : EUR/USD de ~1 pip à 5-10 pips à l'annonce (NYC Servers) ; un spread élargi peut déclencher un stop prématurément et fausse la distance réelle des ordres (FxGlory, FP Markets, Axiory).
- **Slippage** : 5-15 pips à prendre en compte sur entrées ET stops (NYC Servers) ; la liquidité « s'évapore » pendant les releases majeures, remplissages loin du prix (Financial Source, Investopedia — slippage) ; le slippage est cité comme **la raison n°1 d'échec** du straddle news trading (Financial Source).
- **Limites broker** : élargissement de spread, requotes, rejets d'ordres, restrictions de trading autour des news (Financial Source, FxGlory) — vérifier la politique « news trading » du broker/CFD.
- **Coûts** : spread + commissions sur des trades très fréquents érodent l'edge (Financial Source).
- **Annonce déjà pricée** : si le marché a déjà intégré le consensus (ou si la décision est acquise), la réaction peut être minimale ou uniquement technique (FxGlory : ne trader que si l'annonce n'est pas déjà priced in et justifie le risque). Cas typique : FOMC où la décision de taux est connue d'avance — le mouvement vient alors du statement/conférence (14:30), pas du taux.
- **Chasse à la liquidité** : le straddle naïf à distance fixe est une cible connue des algorithmes (FXNX).
- **Paires à éviter** : exotiques (spread/liquidité) — se limiter aux majors USD (EarnForex, FxGlory).

Aucune étude académique validant des performances du straddle stop orders n'a été trouvée ; toute la littérature est issue de sites broker/éducation trading, souvent avec intérêts commerciaux. À traiter comme hypothèses à backtester, pas comme edge prouvé.

## 2.7 STRADDLE — Critères mesurables (traduisibles en règles algorithmiques)

Sélection d'événement :
| # | Règle | Valeur documentée |
|---|-------|-------------------|
| S1 | Annonces éligibles | NFP, FOMC (statement + conf.), CPI US, taux ECB/BoE (tier 1) ; GDP/PCE/retail sales (tier 2) |
| S2 | Horaires de référence | data US : 08:30 ET ; FOMC statement 14:00 ET, conférence 14:30 ET |
| S3 | Trade seulement si surprise potentielle (annonce non entièrement pricée / décision non acquise) | qualitatif — implémentable via écart consensus vs probabilité de marché |

Placement :
| # | Règle | Plage documentée (par défaut médian) |
|---|-------|--------------------------------------|
| S4 | Fenêtre de placement | 1 à 5 min avant la sortie (défaut : 2-3 min) |
| S5 | Distance des ordres | 10-30 pips autour du range pré-annonce (défaut : 15-20 pips au-delà des bornes du range des 15-30 dernières minutes) |
| S6 | Type d'ordre | buy stop au-dessus + sell stop en dessous, OCO forcé côté système (règle d'annulation explicite) |

Gestion de la position déclenchée :
| # | Règle | Plage documentée |
|---|-------|------------------|
| S7 | SL | 15-30 pips (jamais < 10) sous l'entrée |
| S8 | TP | 3-5× la distance de SL, ou stop trailing serré |
| S9 | Breakeven | dès que le profit ≥ distance de SL initiale (EarnForex) |
| S10 | Time-stop | clôture forcée ≤ 60 min après l'annonce |
| S11 | Risque par trade | 0,5-1 % du compte (réduit vs trading normal, à cause du slippage) |

Gardiens anti-whipsaw / anti-coûts :
| # | Règle | Valeur |
|---|-------|--------|
| S12 | Spread max autorisé à la pose des ordres | ex. ≤ 2 pips EUR/USD (suggestion de conception ; FxGlory dit « spread limit + no-trade rule » sans chiffre) |
| S13 | Slippage à budgeter | 5-15 pips (l'inclure dans le calcul du SL et de la distance d'ordre) |
| S14 | Double déclenchement | annulation de l'ordre opposé à exécuter dans la même seconde que le fill ; pas de ré-entrée (one-attempt) |
| S15 | Ne pas trader | spread anormal avant l'annonce, range pré-annonce trop large ou inexistant, annonce 100 % attendue, paire non-major, broker avec exécution news dégradée |

---

# Sources

## ROCKETS / VCP
- TrendSpider — Volatility Contraction Pattern (VCP): A Trader's Guide : https://trendspider.com/learning-center/volatility-contraction-pattern-vcp/
- TraderLion — Mastering the Volatility Contraction Pattern : https://traderlion.com/technical-analysis/volatility-contraction-pattern/
- FinerMarketPoints — VCP Criteria: Complete Checklist : https://www.finermarketpoints.com/post/vcp-criteria-complete-checklist
- FinerMarketPoints — What is a VCP pattern (Minervini explained) : https://www.finermarketpoints.com/post/what-is-a-vcp-pattern-mark-minervini-s-volatility-contraction-pattern-explained
- TradingSim — VCP Pattern: Volatility Contraction Trading Guide : https://www.tradingsim.com/blog/volatility-contraction-pattern
- TraderLion — The Complete Guide to Stan Weinstein's Stage Analysis : https://traderlion.com/trading-strategies/stage-analysis/
- TrendSpider — Weinstein Stage Analysis : https://trendspider.com/blog/master-market-trends-with-ai-powered-weinstein-stage-analysis/
- AlphaTarget — Stage Analysis: An Overview : https://alphatarget.com/insights/stage-analysis-an-overview/
- Next Big Trade — Stage Analysis : https://www.nextbigtrade.com/stage-analysis/
- TradingView — VCP on BINANCE:BTCUSDT : https://www.tradingview.com/chart/BTCUSDT/GOOIN2DR-Volatility-Contraction-Pattern/
- TradingView — VCP Scanner (Minervini Method, Pine Script) : https://www.tradingview.com/script/XKX8RJNc-VCP-Scanner-Minervini-Method/
- TradingView — scripts VCP : https://www.tradingview.com/scripts/vcp/
- altFINS — Crypto Chart Patterns (26 patterns détectés) : https://altfins.com/knowledge-base/chart-patterns/
- altFINS — Complete Guide to Preset Screener Filters : https://altfins.com/knowledge-base/the-complete-guide-to-altfins-preset-screener-filters/
- Investing.com — Cointelegraph Markets Pro VORTECS Report Summary : https://www.investing.com/news/cryptocurrency-news/cointelegraph-markets-pro-vortecs-report-summary--179-gains-from-4-alerts-3032632
- CryptoSlate — Velo Data (profil) : https://cryptoslate.com/companies/velo-data/
- Gainium — Crypto Screener (Breakout Screener) : https://gainium.io/crypto-screener

## STRADDLE
- Investopedia — Straddle (options) : https://www.investopedia.com/terms/s/straddle.asp
- tastylive — Long Straddle : https://tastylive.com/learn/trading-products/options/straddle/
- ForexOP — Introduction to the straddle trade : https://forexop.com/learning/straddle-trade-intro/
- FxGlory — Forex Straddle Strategy: News Orders & Risk : https://fxglory.com/learn/forex-strategies/forex-straddle-strategy/
- EarnForex — Forex News Trading Strategy (News Volatility Straddle) : https://www.earnforex.com/forex-strategy/important-news-trading-strategy/
- FXSSI — News Trading Strategies (straddle breakout) : https://fxssi.com/news-trading-strategies
- NYC Servers — How to Trade NFP : https://newyorkcityservers.com/blog/how-to-trade-nfp
- Taurex — News Trading Strategies in Forex: NFP, CPI, and Key Rates : https://www.tradetaurex.com/forex-insights/forex-news-trading-strategies/
- NAMH Global — NFP Trading Guide 2026 : https://www.namhglobal.com/zh/market-analysis/nfp-trading-guide-2026
- Financial Source — Why straddle news trading doesn't work : https://financialsource.co/why-straddle-news-trading-doesnt-work
- FXNX — Mastering NFP & CPI Trading: Avoid Liquidity Hunts : https://fxnx.com/en/blog/mastering-nfp-cpi-liquidity-hunt-guide
- Investopedia — Slippage : https://www.investopedia.com/terms/s/slippage.asp
- FP Markets — How forex spreads change during news events : https://www.fpmarkets.com/en-jo/education/trading-guides/how-forex-spreads-change-during-news-events/
- Axiory — Which news have the most impact on Forex : https://www.axiory.com/trading-resources/basics/most-important-news/
- Reddit r/Forex — discussion straddle NFP (retours d'expérience) : https://www.reddit.com/r/Forex/comments/144r718/what_do_you_think_of_this_strategy/
- Federal Reserve — FOMC meeting calendars : https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm
- BLS — CPI release schedule : https://www.bls.gov/schedule/news_release/cpi.htm
- CME Group — CPI Report Guide : https://www.cmelitegroup.com/knowledge-hub/cpi-report-release-time-schedule-latest-data-and-how-it-moves-markets/
- NY Fed — Staff Report sr512 (8:30 AM ET release slot) : https://www.newyorkfed.org/medialibrary/media/research/staff_reports/sr512.html
- TradingView — NFP, FOMC, CPI: Trading Major Economic Releases : https://www.tradingview.com/chart/GOLD/7kLXNERN-NFP-FOMC-CPI-Trading-Major-Economic-Releases/

## Limites de la recherche (honnêteté)
- Aucune source académique/indépendante validant statistiquement le VCP ou le straddle stop orders n'a été trouvée ; les chiffres de taux de réussite proviennent de sites éducatifs/vendeurs (TraderLion, FinerMarketPoints) et doivent être traités comme indicatifs.
- Les seuils de volume de breakout divergent selon les sources (+30-40 %, +40-50 %, 140-150 % de la moyenne) — le choix d'un seuil est une décision de conception à backtester.
- Il n'existe pas de méthodologie « VCP crypto » formelle publiée : l'adaptation (RS vs BTC, pondération des volumes weekend, profondeurs élargies) est de l'ingénierie à assumer côté projet.
- Les chiffres de performance VORTECS sont du marketing Cointelegraph.
