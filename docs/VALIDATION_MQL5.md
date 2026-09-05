# Validation numérique du miroir MQL5 — mode d'emploi (Étape 5)

> Objectif : prouver par les nombres que l'EA MQL5 (`mt5/smc_ea_v12.mq5`),
> le moteur Rust et le Pine étalon produisent les MÊMES signaux sur les mêmes
> bornes. Écart attendu ≈ 0. La parité par construction est déjà acquise
> (226 tests Rust, constantes et ordre d'exécution identiques) — il reste à la
> prouver sur l'historique réel.

## 1. Lancer le backtest MT5 (action propriétaire)

1. MT5 → Affichage → Strategy Tester (`Ctrl+R`).
2. Expert : `smc_ea_v12` · Symbole : `XAUUSD` (M15) · Période : **2 derniers mois complets** (bornes notées au jour près).
3. Dépôt/lot : indifférent (on compare les SIGNAUX, pas l'argent).
4. Modélisation : « Every tick based on real ticks » si l'historique tick Axi le permet, sinon « 1 minute OHLC » (le noter — les verdicts intrabar peuvent différer entre les deux).
5. Exécuter, puis relever dans l'onglet Résultats :
   - nombre total de trades,
   - répartition des clôtures (SL / TP1+BE / TP2+BE / TP3 / TS / Expire),
   - profit factor et R cumulé (ou recalcul : somme des trades en unités de risque).

## 2. Extraire les chiffres de référence Rust (même bornes)

```bash
# Après un démarrage de l'app (backend frais) :
curl -s localhost:8080/api/smc/rejeu | jq '.rejeu.clotures
  | map(select(.asset=="XAUUSD" and .tf=="M15"))
  | {nb: length,
     verdicts: (group_by(.verdict) | map({(.[0].verdict): length}) | add),
     r_pondere: (map(.r_pondere) | add)}'
```

Alternative hors-ligne (aucun backend requis) :

```bash
cd backend && DATABASE_PATH=../data/trading.db \
  cargo run --release --bin replay_v12 -- --asset XAUUSD --tf M15
```

## 3. Comparer

| Métrique | Tolérance |
|---|---|
| Nombre de signaux | écart 0 attendu (±1 si bougie d'ouverture de borne incluse/exclue) |
| Répartition des verdicts | identique ; un SL↔TP1 basculé = tracer la minute exacte |
| R cumulé | ≈ 0 d'écart ; sinon divergence d'ordre intrabar (voir §4) |

## 4. Écart ≠ 0 : le tracer

1. Identifier le premier trade divergent (horodatage commun aux deux journaux).
2. Comparer ses niveaux (entrée/SL/TP1/TP2/TP3) — un niveau ≠ 1$ = bug de constantes.
3. Niveaux identiques mais verdict ≠ : regarder la bougie M1 de la clôture dans MT5 —
   c'est un arbitrage **intrabar** (SL touché avant TP dans la même minute) : le
   Rust live évalue au tick (~1 s), le replay Rust à la barre M1 (précédence
   conservatrice SL d'abord), MT5 dépend du mode de modélisation choisi en §1.
   Un écart purement intrabar n'est PAS un bug de miroir — le noter et l'exclure
   de la comparaison.
4. Tout autre écart = bug réel → le corriger des trois côtés (Pine/Rust/MQL5) —
   règle : Pine = étalon.

## 5. Pine dans TradingView (action propriétaire)

Coller `docs/reference/` (le Pine étalon) dans TV sous le nom « Scalp à Nono » —
vérifier visuellement que les zones d'un jour donné coïncident avec l'app.

## État

- [ ] Backtest MT5 lancé (bornes : …/…)
- [ ] Référence Rust extraite (mêmes bornes)
- [ ] Comparaison : écart … (trades / verdicts / R)
- [ ] Verdict : miroir exact / divergences tracées
