# Validation 3 voies KDJ/Halftrend — mode d'emploi (7.D)

> Objectif : prouver par les nombres que l'EA MQL5 (`mt5/kdj_halftrend_ea.mq5`)
> et le moteur Rust (`backend/crates/kdj_halftrend`) produisent les MÊMES trades
> sur les mêmes bougies. Écart attendu = 0. La référence de définition :
> `docs/reference/definition_kdj_halftrend.md` (Pine étalon md5
> `be3343edd854d0759c6ac23bb2ec2b69`).

Les deux côtés journalisent au format CSV identique :
`ts_entree,dir,open_entree,fill_entree,sl_niveau,tp_niveau,ts_sortie,open_sortie,fill_sortie,verdict`
— comparaison sur les prix **théoriques** (open de barre), pas les fills.

## 1. Lancer le backtest MT5 (action propriétaire)

1. Copier `mt5/kdj_halftrend_ea.mq5` dans `MQL5/Experts/` puis compiler (F7).
2. Strategy Tester (`Ctrl+R`) : Expert `kdj_halftrend_ea` · Symbole `XAUUSD` ·
   Période **H1** · Dates : 2 derniers mois complets (bornes notées au jour près).
3. Lot/dépôt : indifférent (on compare les SIGNAUX). Modélisation : « 1 minute
   OHLC » suffit — tout se joue à la clôture de barre H1.
4. Inputs par défaut (period 20, signal 7, amplitude 2, RatioRisk 2.0,
   fenêtre 3000, DiagCsv true).
5. Après exécution : récupérer `MQL5/Files/kdj_diag_XAUUSD_PERIOD_H1.csv`.

Le journal Expert contient aussi chaque fige (`[KDJ diag FIGE]`) et trade
(`[KDJ diag TRADE]`) si le CSV est indisponible.

## 2. Extraire les trades Rust

**Parité au centime (recommandé)** — rejouer sur les bougies exactes du tester :
le test EA (ci-dessus) produit AUSSI `kdj_bougies_XAUUSD_PERIOD_H1.csv`
(dump de la fenêtre à la fin du test, dans le dossier de l'agent). Le Rust
les rejoue telles quelles :

```bash
cd backend && cargo run -q -p api --bin replay_kdj -- \
  --assets XAUUSD --tfs H1 --amplitudes 2 --ratios 2 \
  --bougies <chemin>/kdj_bougies_XAUUSD_PERIOD_H1.csv \
  --dump /tmp/rust_kdj.csv
```

**Alternative (base du collecteur)** — sans le dump de bougies : remplacer
`--bougies ...` par `--fenetre 0` (tout l'historique) ou `--barres 3000`
(fenêtre alignée sur InpFenetre). Attendu : mêmes ENTRÉES au centime, mais
des niveaux TP/SL décalés (EMA200 seedée sur un historique différent :
l'historique du tester Axi ≠ base du collecteur sur l'ancien) — ce n'est PAS
un bug de miroir (leçon SMC : feed du collecteur ≠ base du tester).

## 3. Comparer

```bash
python3 scripts/diff_kdj_3voies.py /tmp/rust_kdj.csv kdj_diag_XAUUSD_PERIOD_H1.csv
```

Le comparateur détecte seul le décalage horaire (le broker Axi horodate en
heure serveur UTC+2/+3, la base en UTC), apparie les trades autour des
changements d'heure (DST) et n'exclut que les trades Rust hors fenêtre du
test EA.

| Contrôle | Tolérance |
|---|---|
| Trade matché (ts entrée + dir) | chaque côté a sa paire |
| open_entree / sl_niveau / tp_niveau | ±0,01 (au centime) en mode `--bougies` |
| ts_sortie + open_sortie + verdict | identiques |

## 4. Écart ≠ 0 : le tracer

1. **Bougies différentes** (mode base du collecteur seulement) : vérifier la
   barre du ts en divergence des deux côtés. En mode `--bougies`, les barres
   sont identiques par construction — un écart y est TOUJOURS un bug.
2. **Bord de fenêtre** : le dernier trade peut être « Ouvert » chez l'EA et
   clôturé chez le Rust (le test s'arrête avant) — écart attendu, ±1 barre.
3. **DST** : autour du changement d'heure, l'appariement automatique ±2 h
   règle le cas ; un Δts résiduel de ±3600 s sur ces trades est attendu.
4. Tout autre écart = bug réel → corriger des trois côtés (Pine = étalon).

## 5. Limites connues (convenues)

- L'EA ne rattrape pas un état de position au redémarrage (backtest de
  validation d'abord ; l'intégration live viendra en 7.E avec la verticale).
- `ErreurFenetre` dans le dump = barre d'entrée sortie de la fenêtre glissante
  (InpFenetre trop court) — augmenter la fenêtre.
- Le verdict « TP » a priorité si TP et SL sont crossés à la même clôture
  (ordre de l'expression `TPlong or StopLong` dans l'étalon) — des deux côtés.
